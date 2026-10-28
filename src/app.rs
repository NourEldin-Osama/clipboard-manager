//! The egui user interface.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Key, RichText, Sense, Stroke, TextureHandle, Vec2,
};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::clipboard::{self, Captured, Flags, SkipReason};
use crate::config::{
    Config, EmbedBackend, LaunchConfig, Paths, SearchMode, ServerConfig, SortOrder, Theme, ViewMode,
};
use crate::embed::Embedding;
use crate::i18n::{self, log_error, t};
use crate::images::ImageLoader;
use crate::library::{Library, Removed};
use crate::models::{self, Progress, SIGLIP_MODELS};
use crate::search::{self, Filter, Hit, Searcher};
use crate::store::{looks_like_link, Item, Kind, Origin, Store};
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
    /// The address and key as typed, and when they last changed.
    typed: Option<(String, Instant)>,
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

    /// Checks the server once its address and key have stayed the same for a
    /// moment, so a half-typed address or key is never sent. The first check
    /// after opening Settings runs at once.
    fn fetch_when_settled(&mut self, s: &ServerConfig, ctx: &egui::Context) {
        const SETTLE: Duration = Duration::from_millis(700);
        let key = Self::key(s);
        if self.url == key || s.base_url.trim().is_empty() {
            self.typed = None;
            return;
        }
        if self.url.is_empty() && self.pending.is_none() {
            self.fetch(s, ctx);
            return;
        }
        let since = match &self.typed {
            Some((typed, since)) if *typed == key => *since,
            _ => {
                let now = Instant::now();
                self.typed = Some((key, now));
                now
            }
        };
        let waited = since.elapsed();
        if waited < SETTLE {
            ctx.request_repaint_after(SETTLE - waited);
        } else if self.pending.is_none() {
            self.typed = None;
            self.fetch(s, ctx);
        }
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
    /// Set by `show`: the next frame selects the top item.
    opened: Arc<AtomicBool>,
}

impl WindowCtl {
    fn show(&self) {
        self.hidden.store(false, Ordering::SeqCst);
        self.opened.store(true, Ordering::SeqCst);
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
/// After a failed background check (offline, say), try again sooner.
const UPDATE_RETRY: Duration = Duration::from_secs(3600);

/// A short message at the bottom of the window, with an optional button.
struct Toast {
    text: String,
    shown: Instant,
    how_long: Duration,
    /// The button's text and what it does.
    action: Option<(String, Command)>,
}

/// Everything the user can do to the history, from a row, a card, the
/// details pane, a key, a menu or a toast. The views send these to `run`
/// instead of changing the history themselves.
pub(super) enum Command {
    Select(u64),
    Copy(u64),
    Delete(u64),
    Pin(u64),
    /// Shows the item in the details pane.
    Open(u64),
    /// Comma-separated tags for one item.
    AddTags(u64, String),
    RemoveTag(u64, String),
    /// Removes a tag from every item that has it.
    DeleteTag(String),
    Import(Vec<PathBuf>),
    ClearHistory,
    /// Brings back the item just deleted.
    UndoDelete,
    /// Records a copy the secret filter held back.
    SaveSkipped(String),
    /// Copies text read from an image; it is not recorded as a new copy.
    CopyText(String),
}

/// An action in Settings waiting for its second click: a destructive one,
/// or sending the history to a server outside this PC.
#[derive(Clone, Copy, PartialEq)]
enum Confirm {
    ClearHistory,
    ResetSettings,
    SendToServer,
}

/// The shortcuts that can be changed in Settings.
#[derive(Clone, Copy, PartialEq)]
enum ShortcutField {
    Show,
    IgnoreNext,
    Copy,
    Pin,
    Delete,
    Tag,
    View,
}

/// How long a deleted item can still be brought back.
const UNDO_FOR: Duration = Duration::from_secs(8);

/// The last deleted item, kept for "Undo" until its toast ends; only then is
/// it removed from the database, with its vectors and image file.
struct PendingDelete {
    removed: Removed,
    since: Instant,
}

/// Keeps the registrations alive; dropping the manager unregisters them.
struct Hotkeys {
    _manager: GlobalHotKeyManager,
}

pub struct App {
    paths: Paths,
    cfg: Config,
    draft: Option<Config>,
    worker: Worker,
    events: Receiver<Event>,
    captures: Receiver<Captured>,
    flags: Arc<Flags>,

    /// The clipboard history and every change to it.
    library: Library,
    backend: BackendState,
    queue: usize,
    status: String,
    error: Option<(String, Instant)>,
    toast: Option<Toast>,
    pending_delete: Option<PendingDelete>,

    query: String,
    query_changed: Option<Instant>,
    query_seq: u64,
    query_vec: Option<(String, Arc<Embedding>)>,
    filter: Filter,
    results: Vec<Hit>,
    /// The history or the query changed: search again.
    dirty: bool,
    searcher: Searcher,
    /// The newest search sent, and whether its results are still to come.
    search_seq: u64,
    searching: bool,
    selected: Option<u64>,
    scroll_to_selected: bool,
    focus_search: Arc<AtomicBool>,
    focused: Arc<AtomicBool>,

    thumbs: HashMap<u64, Option<TextureHandle>>,
    preview: Option<(u64, Option<TextureHandle>)>,
    loader: ImageLoader,
    detail: Option<DetailText>,
    research_pending: bool,
    last_search: Instant,
    download: Option<Download>,
    hotkeys: Option<Hotkeys>,
    /// Global shortcuts are off while one is being recorded in Settings.
    hotkeys_paused: bool,
    hotkey_error: Option<String>,
    /// Global shortcuts Windows refused because another app, or Windows
    /// itself, already uses them.
    hotkey_conflicts: Vec<String>,
    model_list: ModelList,
    settings_page: SettingsPage,
    confirm: Option<Confirm>,
    /// Scroll offset and visible height of the list, from the last frame.
    list_view: (f32, f32),
    /// Grid and List views show the details pane only after "Open details".
    details_open: bool,
    /// Cards per row in the grid, from the last frame, for ↑ ↓ navigation.
    grid_cols: usize,
    win: WindowCtl,
    /// Marks this as the running copy; another start shows this window.
    instance: crate::single_instance::Guard,
    tray: Option<Tray>,
    tray_tx: std::sync::mpsc::Sender<TrayCmd>,
    tray_rx: Receiver<TrayCmd>,
    /// "Quit" from the tray: let the close through instead of hiding.
    quitting: bool,
    was_minimized: bool,
    /// Frames left in which to hide a window started by sign-in.
    start_hidden: u8,
    /// The shortcut field listening for keys in Settings, and why the last try was refused.
    recording: Option<ShortcutField>,
    record_problem: Option<String>,
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
    /// Image saving, copying back and imports, off the UI thread.
    background: Background,
    /// Imports still reading files, and the files they added so far.
    imports_running: usize,
    imported: usize,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        paths: Paths,
        instance: crate::single_instance::Guard,
    ) -> anyhow::Result<Self> {
        let ctx = cc.egui_ctx.clone();
        let (cfg, settings_problem) = Config::load(&paths.config);
        i18n::set(cfg.language, cfg.time_format);
        add_fonts(&ctx);
        icons::register(&ctx);
        set_accent(cfg.accent);
        style(&ctx);
        apply_theme(&ctx, cfg.theme);

        let library = Library::open(Arc::new(Store::open(&paths.db, &paths.images)?))?;

        let (ev_tx, events) = channel();
        let c = ctx.clone();
        let worker = Worker::spawn(
            library.store().clone(),
            paths.models.clone(),
            ev_tx,
            move || c.request_repaint(),
        );

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
            opened: Arc::new(AtomicBool::new(false)),
        };
        let (tray_tx, tray_rx) = channel();
        let mut app = Self {
            win,
            instance,
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
            worker,
            events,
            captures,
            flags,
            library,
            backend: BackendState::Off,
            queue: 0,
            status: String::new(),
            error: None,
            toast: None,
            pending_delete: None,
            query: String::new(),
            query_changed: None,
            query_seq: 0,
            query_vec: None,
            filter: Filter::All,
            results: Vec::new(),
            dirty: true,
            searcher: {
                let c = ctx.clone();
                Searcher::spawn(move || c.request_repaint())
            },
            search_seq: 0,
            searching: false,
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
            research_pending: false,
            last_search: Instant::now(),
            download: None,
            hotkeys: None,
            hotkeys_paused: false,
            hotkey_error: None,
            hotkey_conflicts: Vec::new(),
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
            background: Background::spawn(ctx.clone()),
            imports_running: 0,
            imported: 0,
            recording: None,
            record_problem: None,
            cfg,
        };
        app.set_backend(app.cfg.backend.clone());
        if app.cfg.ocr_images {
            for id in app.library.images_without_ocr() {
                app.worker.send_backfill(Job::Ocr(id));
            }
        }
        app.register_hotkeys(&ctx);
        if app.cfg.tray {
            app.set_tray(true);
        }
        // Only a release build fixes the entry's path at start: a dev build
        // would otherwise point the installed app's entry at itself.
        if app.cfg.start_at_login && update::build().is_some() {
            if let Err(e) = crate::autostart::refresh() {
                log::warn!("{e:#}");
            }
        }
        if let Some(problem) = settings_problem {
            let message = match problem.copy {
                Ok(copy) => log_error!(
                    "error-settings-unreadable",
                    error = problem.error,
                    copy = copy.display().to_string()
                ),
                Err(reason) => log_error!(
                    "error-settings-unreadable-no-copy",
                    error = problem.error,
                    reason = reason
                ),
            };
            app.set_error(message);
        }
        let win = app.win.clone();
        app.instance.on_show_request(move || win.show());
        Ok(app)
    }

    // ----- state changes -------------------------------------------------

    fn set_backend(&mut self, b: EmbedBackend) {
        self.library.clear_embeddings();
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
        self.hotkey_conflicts.clear();
        let manager = match GlobalHotKeyManager::new() {
            Ok(m) => m,
            Err(e) => {
                self.hotkey_error = Some(log_error!(
                    "error-hotkeys-unavailable",
                    error = e.to_string()
                ));
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
                    // Windows refused it: another app (or Windows) holds these keys.
                    Err(global_hotkey::Error::AlreadyRegistered(_)) => {
                        self.hotkey_conflicts.push(s.trim().to_string());
                        self.hotkey_error = Some(log_error!("error-hotkey-taken", keys = s.trim()));
                        None
                    }
                    Err(e) => {
                        self.hotkey_error = Some(log_error!(
                            "error-hotkey-invalid",
                            keys = s.trim(),
                            error = e.to_string()
                        ));
                        None
                    }
                },
                Err(e) => {
                    self.hotkey_error = Some(log_error!(
                        "error-hotkey-invalid",
                        keys = s.trim(),
                        error = e.to_string()
                    ));
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
                Err(e) => self.set_error(log_error!(
                    "error-tray-unavailable",
                    error = format!("{e:#}")
                )),
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
        // Opening the window starts at the top: the newest item, or the best
        // match for the search. The next results select it.
        if self.win.opened.swap(false, Ordering::SeqCst) {
            self.selected = None;
            self.dirty = true;
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
        let msg = match rx.try_recv() {
            Ok(msg) => msg,
            Err(TryRecvError::Empty) => {
                if matches!(self.update, UpdateState::Downloading(..)) {
                    ctx.request_repaint_after(Duration::from_millis(250));
                }
                return;
            }
            // The check or download thread died (a panic): without this the
            // channel stays "busy" and no later check would ever start.
            Err(TryRecvError::Disconnected) => {
                log::warn!("the update thread stopped without an answer");
                self.update_rx = None;
                self.update = UpdateState::Idle;
                self.next_update_check = Instant::now() + UPDATE_RETRY;
                return;
            }
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
                if !manual {
                    self.next_update_check = Instant::now() + UPDATE_RETRY;
                }
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
                let (release, progress) =
                    match std::mem::replace(&mut self.update, UpdateState::Idle) {
                        UpdateState::Downloading(r, p) => (r, p),
                        _ => return,
                    };
                self.update = match result {
                    Ok(path) => UpdateState::Ready(release, path),
                    // Cancelled: back to asking.
                    Err(_) if progress.cancel.load(Ordering::Relaxed) => {
                        UpdateState::Available(release)
                    }
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
        let result = update::install(path).and_then(|exe| match update::relaunch(&exe) {
            Ok(()) => Ok(()),
            // The new version wouldn't start: keep this one.
            Err(e) => match update::roll_back() {
                Ok(()) => Err(e),
                Err(back) => {
                    Err(e.context(format!("putting this version back failed too: {back:#}")))
                }
            },
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
            self.set_error(log_error!("error-saving-settings", error = e.to_string()));
        }
    }

    /// Shows `e` in the status bar. Make it with `log_error!`, which also
    /// writes it to the log.
    fn set_error(&mut self, e: String) {
        self.error = Some((e, Instant::now()));
    }

    fn toast(&mut self, s: impl Into<String>) {
        self.toast_for(s, Duration::from_millis(1400));
    }

    fn toast_for(&mut self, s: impl Into<String>, how_long: Duration) {
        self.toast = Some(Toast {
            text: s.into(),
            shown: Instant::now(),
            how_long,
            action: None,
        });
    }

    /// Carries out what the user asked for.
    fn run(&mut self, command: Command) {
        match command {
            Command::Select(id) => self.selected = Some(id),
            Command::Copy(id) => {
                self.selected = Some(id);
                self.copy(id);
            }
            Command::Delete(id) => self.delete(id),
            Command::Pin(id) => self.toggle_pin(id),
            Command::Open(id) => {
                self.selected = Some(id);
                self.details_open = true;
            }
            Command::AddTags(id, tags) => self.add_tags(id, &tags),
            Command::RemoveTag(id, tag) => self.remove_tag(id, &tag),
            Command::DeleteTag(tag) => self.delete_tag(&tag),
            Command::Import(paths) => self.import(paths),
            Command::ClearHistory => self.clear_history(),
            Command::UndoDelete => self.undo_delete(),
            Command::SaveSkipped(text) => {
                self.on_capture(Captured::Text(text));
                self.toast(t!("toast-saved"));
            }
            Command::CopyText(text) => {
                let flags = self.flags.clone();
                self.background.run(move || {
                    Done::TextCopied(write_clipboard(&flags, || clipboard::copy_text(&text)))
                });
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
        let (id, kind) = (item.id, item.kind);
        // Saved before the jobs go out: the worker reads the item from the disk.
        if let Err(e) = self.library.add(item) {
            self.set_error(log_error!("error-saving-item", error = e.to_string()));
            return;
        }
        for job in self.indexing_jobs(id, kind) {
            self.worker.send(job);
        }
        self.dirty = true;
    }

    /// What the worker does for a new item: read the text in an image, and
    /// make the item's vector.
    fn indexing_jobs(&self, id: u64, kind: Kind) -> Vec<Job> {
        let mut jobs = Vec::new();
        if kind == Kind::Image && self.cfg.ocr_images {
            jobs.push(Job::Ocr(id));
        }
        if self.ready_key().is_some() {
            jobs.push(Job::Embed(id));
        }
        jobs
    }

    fn on_capture(&mut self, c: Captured) {
        if let Captured::Skipped { reason, text } = c {
            // Longer when there is a button to reach.
            let how_long = Duration::from_secs(if text.is_some() { 8 } else { 4 });
            let message = match reason {
                SkipReason::Private => t!("toast-skipped-private"),
                SkipReason::Secret => t!("toast-skipped-secret"),
            };
            self.toast_for(message, how_long);
            if let Some(toast) = &mut self.toast {
                toast.action = text.map(|t| (t!("toast-save-anyway"), Command::SaveSkipped(t)));
            }
            return;
        }
        let hash = c.hash();
        if self.cfg.dedupe {
            let found = match self.library.touch_duplicate(hash) {
                Ok(found) => found,
                // Only saving the time of the item found can fail.
                Err(e) => {
                    self.set_error(log_error!("error-saving-item", error = e.to_string()));
                    true
                }
            };
            if found {
                self.dirty = true;
                return;
            }
        }
        let id = self.library.new_id();
        let mut item = Item::new(id, Kind::Text, Origin::Clipboard, "".into());
        // The capture's own hash, which also covers image pixels and file lists.
        item.hash = hash;
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
                // Writing the PNG takes a while for a big image: the item joins
                // the history once the file is saved (`Done::ImageSaved`).
                let store = self.library.store().clone();
                self.background.run(move || {
                    let saved = store.save_png(id, &img).map_err(|e| format!("{e:#}"));
                    Done::ImageSaved { item, saved }
                });
                return;
            }
        }
        self.add_item(item);
        self.prune();
    }

    fn prune(&mut self) {
        match self.library.prune(self.cfg.max_items) {
            Ok(removed) => self.drop_removed(&removed),
            Err(e) => self.set_error(log_error!("error-deleting", error = e.to_string())),
        }
    }

    /// Reads the files in the background; `finish` saves them as they come.
    fn import(&mut self, paths: Vec<PathBuf>) {
        self.imports_running += 1;
        self.background.import(paths);
    }

    /// Takes in what the background threads finished.
    fn finish(&mut self, done: Done) {
        match done {
            Done::ImageSaved {
                mut item,
                saved: Ok(path),
            } => {
                item.image = Some(path);
                self.add_item(item);
                self.prune();
            }
            Done::ImageSaved { saved: Err(e), .. } => {
                self.set_error(log_error!("error-saving-image", error = e.to_string()))
            }
            Done::Copied { id, result: Ok(()) } => {
                if let Err(e) = self.library.record_use(id) {
                    self.set_error(log_error!("error-saving-item", error = e.to_string()));
                }
                self.dirty = true;
                self.toast(t!("toast-copied"));
                if self.cfg.minimize_after_copy {
                    self.win.hide();
                }
            }
            Done::Copied { result: Err(e), .. } => {
                self.set_error(log_error!("error-copying", error = e.to_string()))
            }
            Done::TextCopied(Ok(())) => self.toast(t!("toast-copied-text")),
            Done::TextCopied(Err(e)) => {
                self.set_error(log_error!("error-copying", error = e.to_string()))
            }
            Done::Imported(batch) => match self.library.add_imported(batch) {
                Ok(added) => {
                    // Files added in bulk wait behind new copies.
                    for (id, kind) in &added {
                        for job in self.indexing_jobs(*id, *kind) {
                            self.worker.send_backfill(job);
                        }
                    }
                    self.imported += added.len();
                    self.dirty = true;
                }
                Err(e) => self.set_error(log_error!("error-saving-item", error = e.to_string())),
            },
            Done::ImportFinished => {
                self.imports_running = self.imports_running.saturating_sub(1);
                if self.imports_running == 0 {
                    let n = std::mem::take(&mut self.imported);
                    self.toast(t!("toast-imported", count = n));
                }
            }
            Done::Crashed => self.set_error(log_error!("error-background-crash")),
        }
    }

    fn index_missing(&mut self) {
        for id in self.library.missing_embeddings() {
            self.worker.send_backfill(Job::Embed(id));
        }
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        self.model_list.poll();
        while let Ok(c) = self.captures.try_recv() {
            self.on_capture(c);
        }
        while let Ok(done) = self.background.done.try_recv() {
            self.finish(done);
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
                    self.library.set_embeddings(stored);
                    self.status.clear();
                    self.backend = BackendState::Ready { key, images };
                    self.index_missing();
                    self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                    self.dirty = true;
                }
                Event::Ocr { id, text } => {
                    let reembed = matches!(self.backend, BackendState::Ready { images: false, .. })
                        && !text.is_empty();
                    self.library.set_ocr(id, &text);
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
                            self.library.set_embedding(id, e);
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
                        self.query_vec = Some((self.query.clone(), Arc::new(embedding)));
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
                        self.toast(t!("toast-downloaded", model = d.repo.as_str()));
                        if matches!(&self.cfg.backend, EmbedBackend::SigLip { repo } if *repo == d.repo)
                        {
                            self.set_backend(self.cfg.backend.clone());
                        }
                    }
                    Ok(Err(e)) => self.set_error(log_error!(
                        "error-download-failed",
                        error = format!("{e:#}")
                    )),
                    Err(_) => self.set_error(log_error!("error-download-crashed")),
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
        if self.dirty {
            self.dirty = false;
            self.last_search = Instant::now();
            let vector = self
                .query_vec
                .as_ref()
                .filter(|(q, _)| *q == self.query)
                .map(|(_, v)| v.clone());
            let query = search::Query {
                text: self.query.clone(),
                vector,
                mode: self.cfg.search_mode,
                filter: self.filter,
                tag: self.tag_filter.clone(),
                min_similarity: self.cfg.min_similarity,
                image_vectors: matches!(self.backend, BackendState::Ready { images: true, .. }),
                sort: self.sort(),
                // While searching, the best matches lead even when pinned items
                // don't match as well.
                pinned_first: self.cfg.pinned_first && self.query.trim().is_empty(),
            };
            self.search_seq += 1;
            self.searching = true;
            self.searcher
                .search(self.search_seq, self.library.snapshot(), query);
        }
        // Looked for after sending, so results from before a change never show.
        if let Some(hits) = self.searcher.results_for(self.search_seq) {
            self.show_results(hits);
        }
    }

    /// Before a key acts on the selected item, the list must match what was
    /// just typed: waits for the search still running, if any.
    fn finish_search(&mut self) {
        self.refresh_results();
        if self.searching {
            let waited = self
                .searcher
                .wait_for(self.search_seq, Duration::from_secs(3));
            match waited {
                Some(hits) => self.show_results(hits),
                // Act on the list as shown. Without this, a search that crashed
                // would make every key wait again.
                None => self.searching = false,
            }
        }
    }

    fn show_results(&mut self, hits: Vec<Hit>) {
        self.searching = false;
        self.results = hits;
        let still_there = self
            .selected
            .is_some_and(|id| self.results.iter().any(|h| h.id == id));
        if !still_there {
            self.selected = self.results.first().map(|h| h.id);
            self.scroll_to_selected = true;
        }
    }

    /// Takes removed items out of the list at once, before the next search
    /// confirms it, and drops their thumbnails.
    fn drop_removed(&mut self, ids: &[u64]) {
        if ids.is_empty() {
            return;
        }
        let gone: HashSet<u64> = ids.iter().copied().collect();
        self.results.retain(|h| !gone.contains(&h.id));
        for id in ids {
            self.thumbs.remove(id);
        }
        self.dirty = true;
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

    /// Puts the item on the clipboard in the background; `finish` reports it.
    fn copy(&mut self, id: u64) {
        let Some(item) = self.library.get(id).cloned() else {
            return;
        };
        let flags = self.flags.clone();
        self.background.run(move || Done::Copied {
            id,
            result: write_clipboard(&flags, || clipboard::copy_item(&item)),
        });
    }

    fn toggle_pin(&mut self, id: u64) {
        if let Err(e) = self.library.toggle_pin(id) {
            self.set_error(log_error!("error-saving-item", error = e.to_string()));
        }
        self.dirty = true;
    }

    fn add_tags(&mut self, id: u64, input: &str) {
        if let Err(e) = self.library.add_tags(id, input) {
            self.set_error(log_error!("error-saving-item", error = e.to_string()));
        }
        self.dirty = true;
    }

    fn remove_tag(&mut self, id: u64, tag: &str) {
        if let Err(e) = self.library.remove_tag(id, tag) {
            self.set_error(log_error!("error-saving-item", error = e.to_string()));
        }
        self.dirty = true;
        self.drop_stale_tag_filter();
    }

    /// Removes a tag from every item that has it.
    fn delete_tag(&mut self, tag: &str) {
        let removed = self.library.delete_tag(tag);
        self.dirty = true;
        self.drop_stale_tag_filter();
        match removed {
            Ok(n) => self.toast(t!("toast-tag-removed", tag = tag, count = n)),
            Err(e) => self.set_error(log_error!("error-saving-item", error = e.to_string())),
        }
    }

    fn drop_stale_tag_filter(&mut self) {
        if let Some(t) = &self.tag_filter {
            if !self.library.any_has_tag(t) {
                self.tag_filter = None;
            }
        }
    }

    /// Takes the item out of the list at once; the database follows when the
    /// "Undo" toast ends (see `finish_delete`).
    fn delete(&mut self, id: u64) {
        // One undo at a time: an earlier delete becomes final now.
        self.finish_delete();
        let Some(removed) = self.library.take(id) else {
            return;
        };
        self.loader.forget(id, false);
        self.drop_stale_tag_filter();
        // Keep the selection on a neighbour: the row that takes its place.
        let row = self.results.iter().position(|h| h.id == id);
        self.drop_removed(&[id]);
        if let Some(i) = row {
            self.selected = self
                .results
                .get(i.min(self.results.len().saturating_sub(1)))
                .map(|h| h.id);
        }
        self.pending_delete = Some(PendingDelete {
            removed,
            since: Instant::now(),
        });
        self.toast_for(t!("toast-deleted"), UNDO_FOR);
        if let Some(toast) = &mut self.toast {
            toast.action = Some((t!("toast-undo"), Command::UndoDelete));
        }
    }

    /// Makes the pending delete final: database row, vectors and image file.
    pub(super) fn finish_delete(&mut self) {
        if let Some(pending) = self.pending_delete.take() {
            if let Err(e) = self.library.forget(pending.removed) {
                self.set_error(log_error!("error-deleting", error = e.to_string()));
            }
        }
    }

    fn undo_delete(&mut self) {
        let Some(pending) = self.pending_delete.take() else {
            return;
        };
        self.selected = Some(pending.removed.item.id);
        self.library.put_back(pending.removed);
        self.dirty = true;
    }

    fn move_selection(&mut self, delta: i32) {
        if self.results.is_empty() {
            return;
        }
        let cur = self
            .selected
            .and_then(|id| self.results.iter().position(|h| h.id == id));
        let next = match cur {
            Some(c) => (c as i32 + delta).clamp(0, self.results.len() as i32 - 1) as usize,
            None => 0,
        };
        self.selected = Some(self.results[next].id);
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
        match self.library.clear() {
            Ok(removed) => {
                self.drop_removed(&removed);
                self.toast(t!("toast-cleared", count = removed.len()));
            }
            Err(e) => self.set_error(log_error!("error-deleting", error = e.to_string())),
        }
    }
}

/// Writes to the clipboard from the background thread. The watcher skips
/// the change this makes instead of recording it as a new copy.
fn write_clipboard(
    flags: &Flags,
    write: impl FnOnce() -> anyhow::Result<()>,
) -> Result<(), String> {
    flags.ignore_next.store(true, Ordering::SeqCst);
    let result = write();
    if result.is_err() {
        flags.ignore_next.store(false, Ordering::SeqCst);
    }
    result.map_err(|e| format!("{e:#}"))
}

mod background;
mod bidi;
mod icons;
mod settings;
mod shortcut;
mod view;

use background::{Background, Done};
use settings::SettingsPage;
use view::{add_fonts, apply_theme, looks_like_code, set_accent, style};
