//! The egui user interface.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Key, RichText, Sense, Stroke, TextureHandle, Vec2,
};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::clipboard::{self, Captured, Flags};
use crate::config::{
    Config, EmbedBackend, LaunchConfig, Paths, SearchMode, ServerConfig, SortOrder, Theme, ViewMode,
};
use crate::embed::Embedding;
use crate::i18n::{self, tr, trf};
use crate::images::ImageLoader;
use crate::import;
use crate::models::{self, Progress, SIGLIP_MODELS};
use crate::search::{self, Filter, Hit};
use crate::store::{looks_like_link, now, Item, Kind, Origin, Store};
use crate::tray::{Tray, TrayCmd};
use crate::update::{self, Release};
use crate::worker::{Event, Job, Worker};

const QUERY_DEBOUNCE: Duration = Duration::from_millis(180);
/// While indexing, re-rank at most this often instead of after every vector.
const RESEARCH_EVERY: Duration = Duration::from_millis(400);
/// Thumbnail textures kept before the cache is dropped and refilled from what is visible.
const MAX_THUMBS: usize = 300;

/// Derived from the selected item once, not every frame.
struct DetailText {
    id: u64,
    shown: String,
    monospace: bool,
    chars: usize,
    words: usize,
}

enum Preview {
    Loading,
    Missing,
    Ready(TextureHandle),
}

#[derive(PartialEq)]
enum BackendState {
    Off,
    Loading,
    Ready { key: String, images: bool },
}

struct Download {
    repo: String,
    progress: Arc<Progress>,
    handle: std::thread::JoinHandle<anyhow::Result<()>>,
}

/// Models offered by the configured server, fetched in the background.
#[derive(Default)]
struct ModelList {
    url: String,
    found: crate::embed::server::ServerModels,
    error: Option<String>,
    pending: Option<Receiver<Result<crate::embed::server::ServerModels, String>>>,
    show_key: bool,
}

impl ModelList {
    fn poll(&mut self) {
        if let Some(rx) = &self.pending {
            if let Ok(r) = rx.try_recv() {
                self.pending = None;
                match r {
                    Ok(m) => {
                        self.found = m;
                        self.error = None;
                    }
                    Err(e) => {
                        self.found = Default::default();
                        self.error = Some(e);
                    }
                }
            }
        }
    }

    fn key(s: &ServerConfig) -> String {
        format!("{}|{}", s.base_url, s.api_key)
    }

    fn fetch(&mut self, s: &ServerConfig, ctx: &egui::Context) {
        let (tx, rx) = channel();
        let (u, k, c) = (s.base_url.clone(), s.api_key.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ =
                tx.send(crate::embed::server::list_models(&u, &k).map_err(|e| format!("{e:#}")));
            c.request_repaint();
        });
        self.url = Self::key(s);
        self.pending = Some(rx);
    }
}

/// Shows and hides the main window from any thread: hotkeys, the tray, the UI.
/// With the tray on, hiding takes the window off the taskbar; otherwise it minimises.
#[derive(Clone)]
struct WindowCtl {
    ctx: egui::Context,
    tray: Arc<AtomicBool>,
    hidden: Arc<AtomicBool>,
    focus_search: Arc<AtomicBool>,
}

impl WindowCtl {
    fn show(&self) {
        self.hidden.store(false, Ordering::SeqCst);
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Visible(true));
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        self.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.focus_search.store(true, Ordering::SeqCst);
        self.ctx.request_repaint();
    }

    fn hide(&self) {
        if self.tray.load(Ordering::SeqCst) {
            self.hidden.store(true, Ordering::SeqCst);
            self.ctx
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else {
            self.ctx
                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }
}

/// Where an update stands.
pub(super) enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    /// Found; waiting for "Download" (automatic downloads are off).
    Available(Release),
    Downloading(Release, Arc<Progress>),
    /// Verified and saved next to the exe; installs on restart or quit.
    Ready(Release, PathBuf),
    Failed {
        error: String,
        release: Option<Release>,
    },
}

enum UpdateMsg {
    Checked {
        result: Result<Release, String>,
        manual: bool,
    },
    Downloaded(Result<PathBuf, String>),
}

/// How often a running app looks for a new release.
const UPDATE_EVERY: Duration = Duration::from_secs(24 * 3600);

/// A short message at the bottom of the window, with an optional button.
struct Toast {
    text: String,
    shown: Instant,
    how_long: Duration,
    action: Option<ToastAction>,
}

/// What a toast's button does.
enum ToastAction {
    /// Records a copy the secret filter held back.
    SaveSkipped(String),
}

impl ToastAction {
    fn label(&self) -> &'static str {
        match self {
            Self::SaveSkipped(_) => "Save anyway",
        }
    }
}

/// Keeps the registrations alive; dropping the manager unregisters them.
struct Hotkeys {
    _manager: GlobalHotKeyManager,
}

pub struct App {
    paths: Paths,
    cfg: Config,
    draft: Option<Config>,
    store: Arc<Store>,
    worker: Worker,
    events: Receiver<Event>,
    captures: Receiver<Captured>,
    flags: Arc<Flags>,

    items: Vec<Item>,
    next_id: u64,
    embeddings: HashMap<u64, Embedding>,
    backend: BackendState,
    queue: usize,
    status: String,
    error: Option<(String, Instant)>,
    toast: Option<Toast>,

    query: String,
    query_changed: Option<Instant>,
    query_seq: u64,
    query_vec: Option<(String, Embedding)>,
    filter: Filter,
    results: Vec<Hit>,
    dirty: bool,
    selected: Option<u64>,
    scroll_to_selected: bool,
    focus_search: Arc<AtomicBool>,
    focused: Arc<AtomicBool>,

    thumbs: HashMap<u64, Option<TextureHandle>>,
    preview: Option<(u64, Option<TextureHandle>)>,
    loader: ImageLoader,
    detail: Option<DetailText>,
    /// Lower-cased searchable text per item, so typing doesn't re-lowercase everything.
    lower: HashMap<u64, String>,
    research_pending: bool,
    last_search: Instant,
    download: Option<Download>,
    hotkeys: Option<Hotkeys>,
    /// Global shortcuts are off while one is being recorded in Settings.
    hotkeys_paused: bool,
    hotkey_error: Option<String>,
    model_list: ModelList,
    settings_page: SettingsPage,
    /// A destructive action waiting for a second click.
    confirm: Option<&'static str>,
    /// Scroll offset and visible height of the list, from the last frame.
    list_view: (f32, f32),
    /// Grid and List views show the details pane only after "Open details".
    details_open: bool,
    /// Cards per row in the grid, from the last frame, for ↑ ↓ navigation.
    grid_cols: usize,
    win: WindowCtl,
    tray: Option<Tray>,
    tray_tx: std::sync::mpsc::Sender<TrayCmd>,
    tray_rx: Receiver<TrayCmd>,
    /// "Quit" from the tray: let the close through instead of hiding.
    quitting: bool,
    was_minimized: bool,
    /// Frames left in which to hide a window started by sign-in.
    start_hidden: u8,
    /// The shortcut field listening for keys in Settings, and why the last try was refused.
    recording: Option<&'static str>,
    record_problem: Option<&'static str>,
    update: UpdateState,
    update_rx: Option<Receiver<UpdateMsg>>,
    /// "Later" hides the update bar until the next change.
    update_hidden: bool,
    next_update_check: Instant,
    /// Only items carrying this tag are listed.
    tag_filter: Option<String>,
    /// The tag being typed in the details pane, for that item.
    tag_input: Option<(u64, String)>,
    /// Give the tag field focus on its next frame (it was just opened).
    tag_focus: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Paths) -> anyhow::Result<Self> {
        let ctx = cc.egui_ctx.clone();
        let cfg = Config::load(&paths.config);
        i18n::set(cfg.language, cfg.time_format);
        add_fonts(&ctx);
        icons::register(&ctx);
        set_accent(cfg.accent);
        style(&ctx);
        apply_theme(&ctx, cfg.theme);

        let store = Arc::new(Store::open(&paths.db, &paths.images)?);
        let items = store.all_items()?;
        let next_id = store.next_id()?;

        let (ev_tx, events) = channel();
        let c = ctx.clone();
        let worker = Worker::spawn(store.clone(), paths.models.clone(), ev_tx, move || {
            c.request_repaint()
        });

        let flags = Arc::new(Flags::default());
        flags.enabled.store(cfg.monitor_clipboard, Ordering::SeqCst);
        flags.skip_secrets.store(cfg.skip_secrets, Ordering::SeqCst);
        let (cap_tx, captures) = channel();
        let c = ctx.clone();
        clipboard::spawn_watcher(flags.clone(), cap_tx, move || c.request_repaint());

        let focus_search = Arc::new(AtomicBool::new(true));
        let win = WindowCtl {
            ctx: ctx.clone(),
            tray: Arc::new(AtomicBool::new(false)),
            hidden: Arc::new(AtomicBool::new(false)),
            focus_search: focus_search.clone(),
        };
        let (tray_tx, tray_rx) = channel();
        let mut app = Self {
            win,
            tray: None,
            tray_tx,
            tray_rx,
            quitting: false,
            was_minimized: false,
            start_hidden: if crate::autostart::launched_minimized() {
                3
            } else {
                0
            },
            paths,
            draft: None,
            store,
            worker,
            events,
            captures,
            flags,
            items,
            next_id,
            embeddings: HashMap::new(),
            backend: BackendState::Off,
            queue: 0,
            status: String::new(),
            error: None,
            toast: None,
            query: String::new(),
            query_changed: None,
            query_seq: 0,
            query_vec: None,
            filter: Filter::All,
            results: Vec::new(),
            dirty: true,
            selected: None,
            scroll_to_selected: false,
            focus_search,
            focused: Arc::new(AtomicBool::new(true)),
            thumbs: HashMap::new(),
            preview: None,
            loader: {
                let c = ctx.clone();
                ImageLoader::new(move || c.request_repaint())
            },
            detail: None,
            lower: HashMap::new(),
            research_pending: false,
            last_search: Instant::now(),
            download: None,
            hotkeys: None,
            hotkeys_paused: false,
            hotkey_error: None,
            model_list: ModelList::default(),
            settings_page: SettingsPage::Search,
            confirm: None,
            list_view: (0.0, 0.0),
            details_open: false,
            grid_cols: 1,
            update: UpdateState::Idle,
            update_rx: None,
            update_hidden: false,
            // A little after start, so launching stays quick.
            next_update_check: Instant::now() + Duration::from_secs(10),
            tag_filter: None,
            tag_input: None,
            tag_focus: false,
            recording: None,
            record_problem: None,
            cfg,
        };
        app.lower = app
            .items
            .iter()
            .map(|i| (i.id, i.searchable_text().to_lowercase()))
            .collect();
        app.set_backend(app.cfg.backend.clone());
        if app.cfg.ocr_images {
            for it in &app.items {
                if it.kind == Kind::Image && it.ocr.is_none() {
                    app.worker.send(Job::Ocr(it.id));
                }
            }
        }
        app.register_hotkeys(&ctx);
        if app.cfg.tray {
            app.set_tray(true);
        }
        if app.cfg.start_at_login {
            // Refreshes the stored path in case the app was moved.
            if let Err(e) = crate::autostart::set(true) {
                log::warn!("{e:#}");
            }
        }
        Ok(app)
    }

    // ----- state changes -------------------------------------------------

    fn set_backend(&mut self, b: EmbedBackend) {
        self.embeddings.clear();
        self.query_vec = None;
        self.backend = if b == EmbedBackend::None {
            BackendState::Off
        } else {
            BackendState::Loading
        };
        self.worker.send(Job::SetBackend(b));
        self.dirty = true;
    }

    fn register_hotkeys(&mut self, ctx: &egui::Context) {
        self.hotkeys = None;
        self.hotkeys_paused = false;
        self.hotkey_error = None;
        let manager = match GlobalHotKeyManager::new() {
            Ok(m) => m,
            Err(e) => {
                self.hotkey_error = Some(format!("global hotkeys unavailable: {e}"));
                return;
            }
        };
        let mut parse = |s: &str| -> Option<HotKey> {
            if s.trim().is_empty() {
                return None;
            }
            match HotKey::from_str(s) {
                Ok(h) => match manager.register(h) {
                    Ok(()) => Some(h),
                    Err(e) => {
                        self.hotkey_error = Some(format!("{s}: {e}"));
                        None
                    }
                },
                Err(e) => {
                    self.hotkey_error = Some(format!("{s}: {e}"));
                    None
                }
            }
        };
        let show = parse(&self.cfg.show_hotkey);
        let ignore = parse(&self.cfg.ignore_next_hotkey);
        let (show_id, ignore_id) = (show.map(|h| h.id()), ignore.map(|h| h.id()));
        let (c, flags, focused, win) = (
            ctx.clone(),
            self.flags.clone(),
            self.focused.clone(),
            self.win.clone(),
        );
        GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
            if e.state() != HotKeyState::Pressed {
                return;
            }
            if Some(e.id()) == show_id {
                if focused.load(Ordering::SeqCst) && !win.hidden.load(Ordering::SeqCst) {
                    win.hide();
                } else {
                    win.show();
                }
            } else if Some(e.id()) == ignore_id {
                flags.ignore_next.store(true, Ordering::SeqCst);
            }
            c.request_repaint();
        }));
        self.hotkeys = Some(Hotkeys { _manager: manager });
    }

    /// Adds or removes the tray icon. Without a tray, hiding falls back to minimising.
    fn set_tray(&mut self, on: bool) {
        self.tray = None;
        if on {
            let (rgba, size) = crate::icon_rgba();
            let paused = !self.flags.enabled.load(Ordering::SeqCst);
            let c = self.win.ctx.clone();
            match Tray::new(rgba, size, paused, self.tray_tx.clone(), move || {
                c.request_repaint()
            }) {
                Ok(t) => self.tray = Some(t),
                Err(e) => self.set_error(format!("system tray unavailable: {e:#}")),
            }
        }
        self.win.tray.store(self.tray.is_some(), Ordering::SeqCst);
        if self.tray.is_none() && self.win.hidden.swap(false, Ordering::SeqCst) {
            self.win.show();
        }
    }

    fn set_recording(&mut self, on: bool) {
        self.cfg.monitor_clipboard = on;
        // Open Settings applies its draft every frame, which would undo this.
        if let Some(draft) = &mut self.draft {
            draft.monitor_clipboard = on;
        }
        self.flags.enabled.store(on, Ordering::SeqCst);
        self.save_config();
    }

    /// Hides to the tray on close or minimise, and handles tray commands.
    /// Runs every frame, including while the window is hidden.
    fn window_logic(&mut self, ctx: &egui::Context) {
        self.update_logic(ctx);
        while let Ok(cmd) = self.tray_rx.try_recv() {
            match cmd {
                TrayCmd::Show => self.win.show(),
                TrayCmd::TogglePause => {
                    let on = !self.flags.enabled.load(Ordering::SeqCst);
                    self.set_recording(on);
                }
                TrayCmd::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if let Some(t) = &mut self.tray {
            t.set_paused(!self.flags.enabled.load(Ordering::SeqCst));
        }
        if self.start_hidden > 0 {
            self.start_hidden -= 1;
            self.win.hide();
            ctx.request_repaint();
        }
        let (close, minimized) = ctx.input(|i| {
            (
                i.viewport().close_requested(),
                i.viewport().minimized.unwrap_or(false),
            )
        });
        if self.tray.is_some() {
            if close && !self.quitting {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.win.hide();
            }
            // Only the moment of minimising: the window stays minimised while hidden.
            if minimized && !self.was_minimized && !self.win.hidden.load(Ordering::SeqCst) {
                self.win.hide();
            }
        }
        self.was_minimized = minimized;
    }

    // ----- updates -------------------------------------------------------

    /// Looks for a new release in the background.
    fn check_for_update(&mut self, manual: bool, ctx: &egui::Context) {
        if self.update_rx.is_some() || update::build().is_none() {
            return;
        }
        let (tx, rx) = channel();
        let c = ctx.clone();
        std::thread::spawn(move || {
            let result = update::check().map_err(|e| format!("{e:#}"));
            let _ = tx.send(UpdateMsg::Checked { result, manual });
            c.request_repaint();
        });
        self.update_rx = Some(rx);
        if !matches!(
            self.update,
            UpdateState::Downloading(..) | UpdateState::Ready(..)
        ) {
            self.update = UpdateState::Checking;
        }
        self.next_update_check = Instant::now() + UPDATE_EVERY;
    }

    fn download_update(&mut self, r: Release, ctx: &egui::Context) {
        let progress = Arc::new(Progress::default());
        let (tx, rx) = channel();
        let (p, rr, c) = (progress.clone(), r.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(UpdateMsg::Downloaded(
                update::download(&rr, &p).map_err(|e| format!("{e:#}")),
            ));
            c.request_repaint();
        });
        self.update_rx = Some(rx);
        self.update = UpdateState::Downloading(r, progress);
        self.update_hidden = false;
    }

    fn update_logic(&mut self, ctx: &egui::Context) {
        if self.cfg.update_check && Instant::now() >= self.next_update_check {
            self.check_for_update(false, ctx);
        }
        let Some(rx) = &self.update_rx else { return };
        let Ok(msg) = rx.try_recv() else {
            if matches!(self.update, UpdateState::Downloading(..)) {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
            return;
        };
        self.update_rx = None;
        match msg {
            UpdateMsg::Checked {
                result: Ok(r),
                manual,
            } => {
                if matches!(
                    self.update,
                    UpdateState::Downloading(..) | UpdateState::Ready(..)
                ) {
                    return;
                }
                if !r.is_newer() {
                    self.update = if manual {
                        UpdateState::UpToDate
                    } else {
                        UpdateState::Idle
                    };
                } else if !manual && r.key() == self.cfg.skipped_update {
                    self.update = UpdateState::Idle;
                } else if self.cfg.update_download {
                    self.download_update(r, ctx);
                } else {
                    self.update = UpdateState::Available(r);
                    self.update_hidden = false;
                }
            }
            UpdateMsg::Checked {
                result: Err(e),
                manual,
            } => {
                log::warn!("update check: {e}");
                // A failed background check (offline, say) isn't worth a bar.
                self.update = if manual {
                    UpdateState::Failed {
                        error: e,
                        release: None,
                    }
                } else {
                    UpdateState::Idle
                };
            }
            UpdateMsg::Downloaded(result) => {
                let release = match std::mem::replace(&mut self.update, UpdateState::Idle) {
                    UpdateState::Downloading(r, _) => r,
                    _ => return,
                };
                self.update = match result {
                    Ok(path) => UpdateState::Ready(release, path),
                    // Cancelled: back to asking.
                    Err(e) if e == "cancelled" => UpdateState::Available(release),
                    Err(e) => {
                        log::warn!("update download: {e}");
                        UpdateState::Failed {
                            error: e,
                            release: Some(release),
                        }
                    }
                };
                self.update_hidden = false;
            }
        }
    }

    /// Swaps in the downloaded version and starts it.
    fn restart_to_update(&mut self, ctx: &egui::Context) {
        let UpdateState::Ready(r, path) = &self.update else {
            return;
        };
        let r = r.clone();
        let result = update::install(path).and_then(|exe| {
            update::relaunch(&exe).inspect_err(|_| {
                // The new version wouldn't start: keep this one.
                let _ = update::roll_back();
            })
        });
        match result {
            Ok(()) => {
                self.update = UpdateState::Idle;
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(e) => {
                self.update = UpdateState::Failed {
                    error: format!("{e:#}"),
                    release: Some(r),
                }
            }
        }
    }

    /// A downloaded update not yet installed goes in on quit, for the next start.
    pub(super) fn install_on_exit(&mut self) {
        if let UpdateState::Ready(_, path) = &self.update {
            if let Err(e) = update::install(path) {
                log::warn!("installing update on exit: {e:#}");
            }
        }
    }

    fn save_config(&mut self) {
        if let Err(e) = self.cfg.save(&self.paths.config) {
            self.set_error(format!("saving settings: {e}"));
        }
    }

    fn set_error(&mut self, e: String) {
        log::error!("{e}");
        self.error = Some((e, Instant::now()));
    }

    fn toast(&mut self, s: impl Into<String>) {
        self.toast_for(s, Duration::from_millis(1400));
    }

    /// Shows `s`, translated when it's one of the fixed messages.
    fn toast_for(&mut self, s: impl Into<String>, how_long: Duration) {
        let s = s.into();
        self.toast = Some(Toast {
            text: tr(&s).to_string(),
            shown: Instant::now(),
            how_long,
            action: None,
        });
    }

    fn run_toast_action(&mut self, action: ToastAction) {
        match action {
            ToastAction::SaveSkipped(text) => {
                self.on_capture(Captured::Text(text));
                self.toast("Saved");
            }
        }
    }

    fn ready_key(&self) -> Option<&str> {
        match &self.backend {
            BackendState::Ready { key, .. } => Some(key),
            _ => None,
        }
    }

    fn add_item(&mut self, item: Item) {
        if let Err(e) = self.store.put(&item) {
            self.set_error(format!("saving item: {e}"));
            return;
        }
        if item.kind == Kind::Image && self.cfg.ocr_images {
            self.worker.send(Job::Ocr(item.id));
        }
        if self.ready_key().is_some() {
            self.worker.send(Job::Embed(item.id));
        }
        self.lower
            .insert(item.id, item.searchable_text().to_lowercase());
        self.items.push(item);
        self.dirty = true;
    }

    fn on_capture(&mut self, c: Captured) {
        if let Captured::Skipped { reason, text } = c {
            // Longer when there is a button to reach.
            let how_long = Duration::from_secs(if text.is_some() { 8 } else { 4 });
            self.toast_for(reason, how_long);
            if let Some(toast) = &mut self.toast {
                toast.action = text.map(ToastAction::SaveSkipped);
            }
            return;
        }
        let hash = c.hash();
        if self.cfg.dedupe {
            if let Some(it) = self
                .items
                .iter_mut()
                .find(|i| i.hash == hash && i.origin == Origin::Clipboard)
            {
                it.created = now();
                let (id, created) = (it.id, it.created);
                if let Err(e) = self.store.update(id, |stored| stored.created = created) {
                    self.set_error(format!("saving item: {e}"));
                }
                self.dirty = true;
                return;
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut item = Item {
            id,
            kind: Kind::Text,
            origin: Origin::Clipboard,
            text: "".into(),
            image: None,
            width: 0,
            height: 0,
            ocr: None,
            pinned: false,
            created: now(),
            hash,
            source: None,
            tags: Vec::new(),
            uses: 0,
        };
        match c {
            Captured::Text(t) => {
                item.kind = if looks_like_link(&t) {
                    Kind::Link
                } else {
                    Kind::Text
                };
                item.text = t.into();
            }
            Captured::Files(f) => {
                item.kind = Kind::Files;
                item.text = clipboard::join_paths(&f).into();
            }
            Captured::Skipped { .. } => unreachable!(),
            Captured::Image(img) => {
                item.kind = Kind::Image;
                item.width = img.width();
                item.height = img.height();
                match self.store.save_png(id, &img) {
                    Ok(p) => item.image = Some(p),
                    Err(e) => return self.set_error(format!("saving image: {e}")),
                }
            }
        }
        self.add_item(item);
        self.prune();
    }

    fn prune(&mut self) {
        let mut history: Vec<(i64, u64)> = self
            .items
            .iter()
            .filter(|i| i.origin == Origin::Clipboard && !i.is_kept())
            .map(|i| (i.created, i.id))
            .collect();
        if history.len() <= self.cfg.max_items {
            return;
        }
        history.sort();
        let excess = history.len() - self.cfg.max_items;
        let doomed: HashSet<u64> = history.into_iter().take(excess).map(|(_, id)| id).collect();
        for it in self.items.iter().filter(|i| doomed.contains(&i.id)) {
            let _ = self.store.delete(it);
            self.lower.remove(&it.id);
            self.embeddings.remove(&it.id);
            self.thumbs.remove(&it.id);
        }
        self.items.retain(|i| !doomed.contains(&i.id));
        self.dirty = true;
    }

    fn import(&mut self, paths: Vec<PathBuf>) {
        let mut next = self.next_id;
        let known: HashSet<u64> = self
            .items
            .iter()
            .filter(|i| i.origin == Origin::Imported)
            .map(|i| i.hash)
            .collect();
        let new: Vec<Item> = import::collect(&paths, &mut next)
            .into_iter()
            .filter(|i| !known.contains(&i.hash))
            .collect();
        self.next_id = next;
        let n = new.len();
        for it in new {
            self.add_item(it);
        }
        self.toast(trf("Imported {n} files", &[("n", &n)]));
    }

    fn index_missing(&mut self) {
        for it in &self.items {
            if !self.embeddings.contains_key(&it.id) {
                self.worker.send(Job::Embed(it.id));
            }
        }
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        self.model_list.poll();
        while let Ok(c) = self.captures.try_recv() {
            self.on_capture(c);
        }
        while let Ok(e) = self.events.try_recv() {
            match e {
                Event::Status(s) => self.status = s,
                Event::Error(s) => self.set_error(s),
                Event::Queue(n) => self.queue = n,
                Event::BackendOff => {
                    self.backend = BackendState::Off;
                    self.status.clear();
                }
                Event::BackendReady {
                    key,
                    images,
                    stored,
                } => {
                    // Learn the server's default model name for the status bar.
                    if let EmbedBackend::Server(sc) = &self.cfg.backend {
                        if sc.model.trim().is_empty()
                            && self.model_list.pending.is_none()
                            && self.model_list.url != ModelList::key(sc)
                        {
                            let sc = sc.clone();
                            self.model_list.fetch(&sc, ctx);
                        }
                    }
                    self.embeddings = stored.into_iter().collect();
                    self.status.clear();
                    self.backend = BackendState::Ready { key, images };
                    self.index_missing();
                    self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                    self.dirty = true;
                }
                Event::Ocr { id, text } => {
                    let reembed = matches!(self.backend, BackendState::Ready { images: false, .. })
                        && !text.is_empty();
                    if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
                        it.ocr = Some(text.into());
                        self.lower.insert(id, it.searchable_text().to_lowercase());
                    }
                    if self.detail.as_ref().map(|d| d.id) == Some(id) {
                        self.detail = None;
                    }
                    if reembed {
                        self.worker.send(Job::Embed(id));
                    }
                    self.dirty = true;
                }
                Event::Embedded { id, key, embedding } => {
                    if self.ready_key() == Some(key.as_str()) {
                        if let Some(e) = embedding {
                            self.embeddings.insert(id, e);
                            self.research_pending |= !self.query.trim().is_empty();
                        }
                    }
                }
                Event::QueryVector {
                    seq,
                    key,
                    embedding,
                } => {
                    if seq == self.query_seq && self.ready_key() == Some(key.as_str()) {
                        self.query_vec = Some((self.query.clone(), embedding));
                        self.dirty = true;
                    }
                }
            }
        }
        let loaded: Vec<_> = self.loader.finished().collect();
        for l in loaded {
            if l.skipped {
                self.loader.forget(l.id, true);
                continue;
            }
            // Mipmaps keep thumbnails smooth when shrunk into a list row.
            let opts = if l.preview {
                egui::TextureOptions::LINEAR
            } else {
                egui::TextureOptions {
                    mipmap_mode: Some(egui::TextureFilter::Linear),
                    ..egui::TextureOptions::LINEAR
                }
            };
            let tex = l.image.map(|ci| {
                ctx.load_texture(
                    format!("{}{}", if l.preview { "preview" } else { "thumb" }, l.id),
                    ci,
                    opts,
                )
            });
            if l.preview {
                // A preview for an item the user already moved past is dropped.
                if self.selected == Some(l.id) {
                    self.preview = Some((l.id, tex));
                }
                self.loader.forget(l.id, true);
            } else {
                self.thumbs.insert(l.id, tex);
            }
        }
        if let Some(d) = &self.download {
            if d.handle.is_finished() {
                let d = self.download.take().unwrap();
                match d.handle.join() {
                    Ok(Ok(())) => {
                        self.toast(trf("Downloaded {name}", &[("name", &d.repo)]));
                        if matches!(&self.cfg.backend, EmbedBackend::SigLip { repo } if *repo == d.repo)
                        {
                            self.set_backend(self.cfg.backend.clone());
                        }
                    }
                    Ok(Err(e)) => self.set_error(format!("download failed: {e:#}")),
                    Err(_) => self.set_error("download thread crashed".into()),
                }
            }
        }
    }

    fn refresh_results(&mut self) {
        // Ask the model for a query vector once typing pauses.
        if let Some(t) = self.query_changed {
            if t.elapsed() >= QUERY_DEBOUNCE {
                self.query_changed = None;
                if self.ready_key().is_some()
                    && self.cfg.search_mode != SearchMode::Keyword
                    && !self.query.trim().is_empty()
                {
                    self.query_seq += 1;
                    self.worker
                        .query(self.query_seq, self.query.trim().to_string());
                }
            }
        }
        if self.research_pending && self.last_search.elapsed() >= RESEARCH_EVERY {
            self.research_pending = false;
            self.dirty = true;
        }
        if !self.dirty {
            return;
        }
        self.dirty = false;
        self.last_search = Instant::now();
        let vector = self
            .query_vec
            .as_ref()
            .filter(|(q, _)| *q == self.query)
            .map(|(_, v)| v);
        self.results = search::search(
            &self.items,
            &self.lower,
            &self.embeddings,
            &search::Query {
                text: &self.query,
                vector,
                mode: self.cfg.search_mode,
                filter: self.filter,
                tag: self.tag_filter.as_deref(),
                min_similarity: self.cfg.min_similarity,
                image_vectors: matches!(self.backend, BackendState::Ready { images: true, .. }),
            },
        );
        let sort = self.sort();
        // While searching, the best matches lead even when pinned items don't match as well.
        let pinned_first = self.cfg.pinned_first && self.query.trim().is_empty();
        search::order(&self.items, &mut self.results, sort, pinned_first);
        let still_there = self
            .selected
            .map(|id| self.results.iter().any(|h| self.items[h.idx].id == id))
            .unwrap_or(false);
        if !still_there {
            self.selected = self.results.first().map(|h| self.items[h.idx].id);
        }
    }

    /// The order in effect: searching and browsing each remember their own.
    fn sort(&self) -> SortOrder {
        if self.query.trim().is_empty() {
            match self.cfg.sort {
                SortOrder::BestMatch => SortOrder::Recent,
                s => s,
            }
        } else {
            self.cfg.search_sort
        }
    }

    fn set_sort(&mut self, s: SortOrder) {
        if self.query.trim().is_empty() {
            self.cfg.sort = s;
        } else {
            self.cfg.search_sort = s;
        }
        self.save_config();
        self.dirty = true;
    }

    fn item(&self, id: u64) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    fn copy(&mut self, id: u64) {
        let Some(it) = self.item(id).cloned() else {
            return;
        };
        self.flags.ignore_next.store(true, Ordering::SeqCst);
        let r = match it.kind {
            Kind::Text | Kind::Link => clipboard::copy_text(&it.text),
            Kind::Files => {
                clipboard::copy_files(&it.text.lines().map(PathBuf::from).collect::<Vec<_>>())
            }
            Kind::Image => it
                .image
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing image"))
                .and_then(|p| Ok(image::open(p)?.into_rgba8()))
                .and_then(|img| clipboard::copy_image(&img)),
        };
        match r {
            Ok(()) => {
                if let Some(x) = self.items.iter_mut().find(|i| i.id == id) {
                    // Copying from the list counts as a use, and makes a copied item recent.
                    x.uses = x.uses.saturating_add(1);
                    if x.origin == Origin::Clipboard {
                        x.created = now();
                    }
                    let (uses, created) = (x.uses, x.created);
                    let saved = self.store.update(id, |stored| {
                        stored.uses = uses;
                        stored.created = created;
                    });
                    if let Err(e) = saved {
                        self.set_error(format!("saving item: {e}"));
                    }
                    self.dirty = true;
                }
                self.toast("Copied");
                if self.cfg.minimize_after_copy {
                    self.win.hide();
                }
            }
            Err(e) => {
                self.flags.ignore_next.store(false, Ordering::SeqCst);
                self.set_error(format!("copy failed: {e}"));
            }
        }
    }

    fn toggle_pin(&mut self, id: u64) {
        if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
            it.pinned = !it.pinned;
            let pinned = it.pinned;
            if let Err(e) = self.store.update(id, |stored| stored.pinned = pinned) {
                self.set_error(format!("saving item: {e}"));
            }
            self.dirty = true;
        }
    }

    /// Every tag in use with its item count, most used first.
    fn all_tags(&self) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, (String, usize)> = HashMap::new();
        for t in self.items.iter().flat_map(|i| &i.tags) {
            counts
                .entry(t.to_lowercase())
                .or_insert_with(|| (t.clone(), 0))
                .1 += 1;
        }
        let mut v: Vec<(String, usize)> = counts.into_values().collect();
        v.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });
        v
    }

    /// Saves an item's tags, and only them, so OCR text saved meanwhile stays.
    fn save_tags(&mut self, id: u64) {
        let Some(it) = self.items.iter().find(|i| i.id == id) else {
            return;
        };
        let (tags, lower) = (it.tags.clone(), it.searchable_text().to_lowercase());
        if let Err(e) = self.store.update(id, |stored| stored.tags = tags) {
            self.set_error(format!("saving item: {e}"));
            return;
        }
        self.lower.insert(id, lower);
        self.dirty = true;
    }

    /// Adds comma-separated tags, keeping the spelling of a tag already in use.
    fn add_tags(&mut self, id: u64, input: &str) {
        let known = self.all_tags();
        let Some(it) = self.items.iter_mut().find(|i| i.id == id) else {
            return;
        };
        for t in input
            .split(',')
            .map(|t| t.trim().trim_start_matches('#').trim())
            .filter(|t| !t.is_empty())
        {
            let t: String = t.chars().take(40).collect();
            if it.has_tag(&t) {
                continue;
            }
            let t = known
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(&t))
                .map(|(k, _)| k.clone())
                .unwrap_or(t);
            it.tags.push(t);
        }
        self.save_tags(id);
    }

    fn remove_tag(&mut self, id: u64, tag: &str) {
        if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
            it.tags.retain(|t| !t.eq_ignore_ascii_case(tag));
        }
        self.save_tags(id);
        self.drop_stale_tag_filter();
    }

    /// Removes a tag from every item that has it.
    fn delete_tag(&mut self, tag: &str) {
        let ids: Vec<u64> = self
            .items
            .iter()
            .filter(|i| i.has_tag(tag))
            .map(|i| i.id)
            .collect();
        for id in &ids {
            if let Some(it) = self.items.iter_mut().find(|i| i.id == *id) {
                it.tags.retain(|t| !t.eq_ignore_ascii_case(tag));
            }
            self.save_tags(*id);
        }
        self.drop_stale_tag_filter();
        self.toast(trf(
            "Removed #{tag} from {n} items",
            &[("tag", &tag), ("n", &ids.len())],
        ));
    }

    fn drop_stale_tag_filter(&mut self) {
        if let Some(t) = &self.tag_filter {
            if !self.items.iter().any(|i| i.has_tag(t)) {
                self.tag_filter = None;
            }
        }
    }

    fn delete(&mut self, id: u64) {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            let it = self.items.remove(pos);
            if let Err(e) = self.store.delete(&it) {
                self.set_error(format!("delete failed: {e}"));
            }
            self.embeddings.remove(&id);
            self.thumbs.remove(&id);
            self.lower.remove(&id);
            self.loader.forget(id, false);
            self.drop_stale_tag_filter();
            // Keep the selection on a neighbour.
            let idx = self.results.iter().position(|h| h.idx == pos);
            self.dirty = true;
            self.refresh_results();
            if let Some(i) = idx {
                self.selected = self
                    .results
                    .get(i.min(self.results.len().saturating_sub(1)))
                    .map(|h| self.items[h.idx].id);
            }
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.results.is_empty() {
            return;
        }
        let cur = self
            .selected
            .and_then(|id| self.results.iter().position(|h| self.items[h.idx].id == id));
        let next = match cur {
            Some(c) => (c as i32 + delta).clamp(0, self.results.len() as i32 - 1) as usize,
            None => 0,
        };
        self.selected = Some(self.items[self.results[next].idx].id);
        self.scroll_to_selected = true;
    }

    fn preview(&mut self, it: &Item) -> Preview {
        match &self.preview {
            Some((id, Some(t))) if *id == it.id => return Preview::Ready(t.clone()),
            Some((id, None)) if *id == it.id => return Preview::Missing,
            _ => {}
        }
        match &it.image {
            Some(p) => {
                self.loader.request(it.id, p, true);
                Preview::Loading
            }
            None => Preview::Missing,
        }
    }

    fn detail_text(&mut self, it: &Item) -> &DetailText {
        if self.detail.as_ref().map(|d| d.id) != Some(it.id) {
            let shown: String = it.text.chars().take(100_000).collect();
            self.detail = Some(DetailText {
                id: it.id,
                monospace: looks_like_code(&shown),
                chars: it.text.chars().count(),
                words: it.text.split_whitespace().count(),
                shown,
            });
        }
        self.detail.as_ref().unwrap()
    }

    /// Deletes clipboard items; pinned, tagged and imported items stay.
    fn clear_history(&mut self) {
        let doomed: Vec<Item> = self
            .items
            .iter()
            .filter(|i| i.origin == Origin::Clipboard && !i.is_kept())
            .cloned()
            .collect();
        for it in &doomed {
            if let Err(e) = self.store.delete(it) {
                self.set_error(format!("delete failed: {e}"));
            }
            self.lower.remove(&it.id);
            self.embeddings.remove(&it.id);
            self.thumbs.remove(&it.id);
        }
        let gone: HashSet<u64> = doomed.iter().map(|i| i.id).collect();
        self.items.retain(|i| !gone.contains(&i.id));
        self.dirty = true;
        self.toast(trf("Cleared {n} items", &[("n", &gone.len())]));
    }
}

mod bidi;
mod icons;
mod settings_page;
mod shortcut;
mod view;

use settings_page::SettingsPage;
use view::{add_fonts, apply_theme, looks_like_code, set_accent, style};
