//! The egui user interface.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Key, RichText, Sense, Stroke, TextureHandle, Vec2,
};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::clipboard::{self, Capture, Captured, Flags, SkipReason};
use crate::config::{
    Config, EmbedBackend, LaunchConfig, Paths, SearchEngine, SearchMode, SortOrder, Theme,
    UpdateNote, ViewMode,
};
use crate::download::Progress;
use crate::embed::Embedding;
use crate::i18n::{self, log_error, t};
use crate::images::ImageLoader;
use crate::library::{Library, Removed};
use crate::paste;
use crate::search::{self, Filter, Hit, Searcher};
use crate::store::{looks_like_link, Item, Kind, Origin, Store};
use crate::tray::{Tray, TrayCmd};
use crate::update::{self, Release};
use crate::worker::{Event, Job, Worker};

const QUERY_DEBOUNCE: Duration = Duration::from_millis(180);
/// While indexing, re-rank at most this often instead of after every vector.
const RESEARCH_EVERY: Duration = Duration::from_millis(400);
/// Thumbnail textures kept; past this the ones shown longest ago go, down
/// to `THUMBS_KEPT`, so dropping happens now and then, not every frame.
const MAX_THUMBS: usize = 300;
const THUMBS_KEPT: usize = 240;

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

/// Models offered by a server, fetched in the background.
#[derive(Default)]
struct ModelList {
    /// The address and key last asked (see `key`); empty before any check.
    url: String,
    found: crate::embed::server::ServerModels,
    error: Option<String>,
    pending: Option<Receiver<Result<crate::embed::server::ServerModels, String>>>,
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

    fn key(url: &str, api_key: &str) -> String {
        format!("{url}|{api_key}")
    }

    /// Asks the server at `url` what it offers. With `embedding_list`, its
    /// embedding models are asked for apart (OpenRouter).
    fn fetch(&mut self, url: &str, api_key: &str, embedding_list: bool, ctx: &egui::Context) {
        let (tx, rx) = channel();
        let (u, k, c) = (url.to_string(), api_key.to_string(), ctx.clone());
        std::thread::spawn(move || {
            let found = crate::embed::server::list_models(&u, &k, embedding_list);
            let _ = tx.send(found.map_err(|e| crate::net::error_text(&e)));
            c.request_repaint();
        });
        self.url = Self::key(url, api_key);
        self.pending = Some(rx);
    }

    /// Checks the server once its address and key have stayed the same for a
    /// moment, so a half-typed address or key is never sent. The first check
    /// after opening the server runs at once.
    fn fetch_when_settled(
        &mut self,
        url: &str,
        api_key: &str,
        embedding_list: bool,
        ctx: &egui::Context,
    ) {
        const SETTLE: Duration = Duration::from_millis(700);
        let key = Self::key(url, api_key);
        if self.url == key || url.trim().is_empty() {
            self.typed = None;
            return;
        }
        if self.url.is_empty() && self.pending.is_none() {
            self.fetch(url, api_key, embedding_list, ctx);
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
            self.fetch(url, api_key, embedding_list, ctx);
        }
    }
}

/// Shows and hides the main window and the quick popup from any thread:
/// hotkeys, the tray, the UI. With the tray on, hiding takes the window off
/// the taskbar; otherwise it minimises. The commands name the main window,
/// because one may come while the popup is being drawn.
#[derive(Clone)]
struct WindowCtl {
    ctx: egui::Context,
    tray: Arc<AtomicBool>,
    hidden: Arc<AtomicBool>,
    focus_search: Arc<AtomicBool>,
    /// Set by `show_over` and `show_popup`: the next frame selects the top item.
    opened: Arc<AtomicBool>,
    /// The other program's window that was in front when a shortcut opened
    /// this app: where pasting goes. `paste::NO_WINDOW` otherwise.
    previous: Arc<AtomicIsize>,
    /// The quick popup is open.
    popup: Arc<AtomicBool>,
    /// The popup has the keyboard.
    popup_focused: Arc<AtomicBool>,
    /// Set by `show_popup`: the next frame places the popup at the mouse
    /// pointer and gives it the keyboard.
    popup_placing: Arc<AtomicBool>,
}

impl WindowCtl {
    /// Shows the full window; with no program to go back to, pasting only copies.
    fn show(&self) {
        self.show_over(paste::NO_WINDOW);
    }

    /// Shows the full window over `previous`, the window pasting goes to.
    fn show_over(&self, previous: paste::Window) {
        self.previous.store(previous, Ordering::SeqCst);
        self.opened.store(true, Ordering::SeqCst);
        self.show_full();
    }

    /// Shows the full window and closes the popup; pasting still goes
    /// where it went.
    fn show_full(&self) {
        self.popup.store(false, Ordering::SeqCst);
        self.hidden.store(false, Ordering::SeqCst);
        self.main_window(egui::ViewportCommand::Visible(true));
        self.main_window(egui::ViewportCommand::Minimized(false));
        self.main_window(egui::ViewportCommand::Focus);
        self.focus_search.store(true, Ordering::SeqCst);
        self.ctx.request_repaint();
    }

    /// Opens the quick popup at the mouse pointer, over `previous`, the
    /// window pasting goes to.
    fn show_popup(&self, previous: paste::Window) {
        self.previous.store(previous, Ordering::SeqCst);
        self.opened.store(true, Ordering::SeqCst);
        self.popup_focused.store(false, Ordering::SeqCst);
        self.popup_placing.store(true, Ordering::SeqCst);
        self.popup.store(true, Ordering::SeqCst);
        self.focus_search.store(true, Ordering::SeqCst);
        self.ctx.request_repaint();
    }

    fn is_popup_open(&self) -> bool {
        self.popup.load(Ordering::SeqCst)
    }

    /// Closes the popup if it is open, else hides the full window.
    fn hide(&self) {
        if self.popup.swap(false, Ordering::SeqCst) {
            self.ctx.request_repaint();
        } else if self.tray.load(Ordering::SeqCst) {
            self.hidden.store(true, Ordering::SeqCst);
            self.main_window(egui::ViewportCommand::Visible(false));
        } else {
            self.main_window(egui::ViewportCommand::Minimized(true));
        }
    }

    fn main_window(&self, command: egui::ViewportCommand) {
        self.ctx
            .send_viewport_cmd_to(egui::ViewportId::ROOT, command);
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

/// An item's menu, opened by a right-click or by Shift+F10.
struct ItemMenu {
    id: u64,
    /// Where it opens: at the pointer, or at the selected row for Shift+F10.
    at: egui::Pos2,
    /// It belongs to the popup's list, not the full window's.
    in_popup: bool,
    /// Its first entry is to take the keyboard, as after Shift+F10.
    focus_first: bool,
}

/// Everything the user can do to the history, from a row, a card, the
/// details pane, a key, a menu or a toast. The views send these to `run`
/// instead of changing the history themselves.
pub(super) enum Command {
    Select(u64),
    /// Puts the item back on the clipboard, and maybe pastes it.
    Copy(u64, Paste),
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
    /// Records a copy that the secret filter or a rule held back.
    SaveSkipped(Capture),
    /// Copies text read from an image; it is not recorded as a new copy.
    CopyText(String),
    /// Turns off Send images for the search model with this key, after its
    /// server refused an image.
    StopSendingImages(String),
    /// Shows the weaker matches folded below the results.
    ShowWeaker,
    /// Ctrl+click: adds an item to the ones selected together, or takes it
    /// out.
    ToggleMark(u64),
    /// Shift+click: selects the items from where the range starts to this one.
    MarkRange(u64),
    /// Pins the items, or unpins them all when all are pinned.
    PinGroup(Vec<u64>),
    /// Deletes the items; one Undo brings them all back.
    DeleteGroup(Vec<u64>),
    /// Puts the items' texts on the clipboard together, and maybe pastes them.
    CopyTogether(Vec<u64>, Paste),
    /// Opens what is new in this version over the full window.
    ShowWhatsNew,
}

/// Whether putting an item back on the clipboard also pastes it into the
/// program the window opened over. Without such a program it only copies.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Paste {
    No,
    Yes,
    /// Pastes text without its formatting.
    Plain,
}

/// One way to put an item back, as a button or menu entry shows it.
pub(super) struct CopyWay {
    pub(super) text: String,
    pub(super) keys: String,
    pub(super) paste: Paste,
}

/// A destructive action in Settings waiting for its second click.
#[derive(Clone, Copy, PartialEq)]
enum Confirm {
    ClearHistory,
    ResetSettings,
}

/// The shortcuts that can be changed in Settings.
#[derive(Clone, Copy, PartialEq)]
enum ShortcutField {
    Popup,
    Window,
    IgnoreNext,
    Paste,
    PastePlain,
    CopyOnly,
    Pin,
    Delete,
    Tag,
    View,
}

/// How long a deleted item can still be brought back.
const UNDO_FOR: Duration = Duration::from_secs(8);
/// How long a message that names a key stays.
const KEY_HINT_FOR: Duration = Duration::from_secs(4);
/// What Copy together puts on the clipboard: the texts, links and file
/// paths of `items`, one after another; with how many there are and how
/// many pictures are left out.
fn texts_together<'a>(items: impl Iterator<Item = &'a Item>) -> (usize, String, usize) {
    let (pictures, texts): (Vec<_>, Vec<_>) = items.partition(|it| it.kind == Kind::Image);
    let texts: Vec<&str> = texts.iter().map(|it| it.text.trim_end()).collect();
    (texts.len(), texts.join("\n"), pictures.len())
}

/// eframe's entry for the window's place and size in app.ron.
const WINDOW_KEY: &str = "window";
/// The window's size at the first start, in points.
pub const WINDOW_SIZE: [f32; 2] = [1040.0, 680.0];
/// The smallest the window can be made, in points.
pub const SMALLEST_WINDOW: [f32; 2] = [560.0, 380.0];

/// eframe saves the window's place and size just before the app's own
/// `save`, every 30 seconds and when the app quits. A minimized window has
/// neither: Windows puts it at -32000 with no size, and the next start would
/// open it tiny, in a corner of the screen. So while the window is minimized,
/// the entry saved before goes back in; otherwise the new entry is kept.
fn keep_window_state(
    storage: &mut dyn eframe::Storage,
    minimized: bool,
    kept: &mut Option<String>,
) {
    if !minimized {
        *kept = storage.get_string(WINDOW_KEY);
    } else if let Some(entry) = kept {
        storage.set_string(WINDOW_KEY, entry.clone());
    }
}

/// How long the message about a server that refused an image stays.
const IMAGES_REFUSED_FOR: Duration = Duration::from_secs(15);
/// How often deleting by age runs while it is on.
const AGE_PRUNE_EVERY: Duration = Duration::from_secs(60 * 60);

/// The last deleted item, kept for "Undo" until its toast ends; only then is
/// it removed from the database, with its vectors and image file.
struct PendingDelete {
    /// Taken from the last to the first, so each comes back in its place
    /// when they go back in the other order.
    removed: Vec<Removed>,
    since: Instant,
}

/// Keeps the registrations alive; dropping the manager unregisters them.
struct Hotkeys {
    _manager: GlobalHotKeyManager,
}

/// The ids of the registered global shortcuts, `NO_HOTKEY` when one isn't
/// set. The global-hotkey library keeps only the first event handler it is
/// given, so that handler reads the current ids from here.
#[derive(Default)]
struct HotkeyIds {
    popup: AtomicU64,
    window: AtomicU64,
    ignore: AtomicU64,
}

const NO_HOTKEY: u64 = u64::MAX;

pub struct App {
    paths: Paths,
    cfg: Config,
    draft: Option<Config>,
    worker: Worker,
    events: Receiver<Event>,
    captures: Receiver<Capture>,
    /// `cfg.rules`, ready to match copies.
    rules: crate::rules::Rules,
    flags: Arc<Flags>,

    /// The clipboard history and every change to it.
    library: Library,
    backend: BackendState,
    queue: usize,
    /// Items in the current indexing run, for its progress bar (`run_total`).
    queue_total: usize,
    status: String,
    error: Option<(String, Instant)>,
    toast: Option<Toast>,
    pending_delete: Option<PendingDelete>,
    /// When deleting by age runs next; `None` runs it at the next frame.
    next_age_prune: Option<Instant>,

    query: String,
    query_changed: Option<Instant>,
    query_seq: u64,
    query_vec: Option<(String, Arc<Embedding>)>,
    filter: Filter,
    results: Vec<Hit>,
    /// Weaker matches, folded under one line below the results.
    weaker: Vec<Hit>,
    /// The search the weaker matches were shown for; another search folds
    /// them again.
    weaker_shown_for: Option<String>,
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
    /// The frame each image item was last drawn in, for dropping thumbnails.
    thumb_seen: HashMap<u64, u64>,
    /// Which paths of the item in the details pane are folders, by item.
    folders_of: Option<(u64, Vec<bool>)>,
    /// Tab moved the focus last, so the focused control is outlined; a
    /// click hides the outline again.
    keyboard_focus: bool,
    /// How wide the top bar's buttons were in the last frame, so the
    /// search field takes the rest.
    top_actions_w: f32,
    /// A text field in Settings had the keyboard when the keys were last
    /// looked at (see `keyboard`).
    settings_typing: bool,
    preview: Option<(u64, Option<TextureHandle>)>,
    loader: ImageLoader,
    /// The names and icons of the programs copies came from.
    programs: programs::Programs,
    /// The window pasting goes to and its program, read once per window.
    paste_program: (paste::Window, Option<PathBuf>),
    /// What the reading model's llama-server is doing while it starts.
    reader_status: String,
    /// The window's place and size as eframe last saved them while it was
    /// not minimized (see `keep_window_state`).
    window_state: Option<String>,
    /// The window's size at start was looked at.
    start_size_checked: bool,
    detail: Option<DetailText>,
    research_pending: bool,
    last_search: Instant,
    hotkeys: Option<Hotkeys>,
    hotkey_ids: Arc<HotkeyIds>,
    /// Global shortcuts are off while one is being recorded in Settings.
    hotkeys_paused: bool,
    hotkey_error: Option<String>,
    /// Global shortcuts Windows refused because another app, or Windows
    /// itself, already uses them.
    hotkey_conflicts: Vec<String>,
    /// What the search model's server offers, for the default model's name.
    model_list: ModelList,
    settings_page: SettingsPage,
    confirm: Option<Confirm>,
    /// The server being added or edited in Settings.
    server_editor: Option<ServerEditor>,
    /// A choice that would send to a server outside this PC, waiting for
    /// the user's yes.
    pending_send: Option<PendingSend>,
    /// Scroll offset and visible height of the list, from the last frame.
    list_view: (f32, f32),
    /// The same for the list in the popup.
    popup_list_view: (f32, f32),
    /// Where the popup last opened, in points.
    popup_at: Option<egui::Pos2>,
    /// The popup was made hidden in the last frame; it shows in this one.
    popup_hidden_last_frame: bool,
    /// The popup's bar is being dragged: the next frame starts moving it.
    popup_moving: bool,
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
    /// The open menu of an item.
    item_menu: Option<ItemMenu>,
    /// Items selected together with the selected one, in the full window:
    /// Ctrl+click, Shift+click, Shift+arrows or Ctrl+A. Fewer than two
    /// shown means one item is selected (see `group`).
    marked: HashSet<u64>,
    /// Where a Shift range starts.
    anchor: Option<u64>,
    /// The field for a tag for every item selected together, while open.
    group_tag: Option<String>,
    /// Shift+F10 was pressed: the selected row or card opens its menu the
    /// next time it is drawn.
    menu_by_key: bool,
    /// The sheet of every key is open, over the popup (`true`) or the full
    /// window (`false`).
    keys_sheet: Option<bool>,
    /// This is the first start of a newer version: the first window that
    /// shows says so.
    whats_new_pending: bool,
    /// What is new in this version is open over the full window.
    whats_new_open: bool,
    /// The command being run came from a key, so its message doesn't name it.
    by_key: bool,
    /// Keys a message has named in this run: each is named once.
    taught: HashSet<String>,
    /// The key to name when the copy in progress, done with the mouse, says
    /// "Copied".
    copy_hint: Option<String>,
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
        // No settings yet means a new install, which has nothing new to hear.
        let had_settings = paths.config.exists();
        let (cfg, settings_problem, updated) = Config::load(&paths.config);
        i18n::set(cfg.language, cfg.time_format);
        add_fonts(&ctx);
        icons::register(&ctx);
        set_accent(cfg.accent);
        style(&ctx);
        apply_theme(&ctx, cfg.theme);

        let store = Arc::new(Store::open(&paths.db, &paths.images)?);
        // The built-in search model of versions before 0.5 is gone: so are its
        // vectors and the files it and the built-in text reader downloaded.
        if let Err(e) = store.delete_vectors_of_models("siglip:") {
            log::warn!("deleting the built-in search model's vectors: {e:#}");
        }
        paths.remove_built_in_models();
        let library = Library::open(store)?;
        let image_files = library.store().images.clone();

        let (ev_tx, events) = channel();
        let c = ctx.clone();
        let worker = Worker::spawn(
            library.store().clone(),
            cfg.text_reader(),
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
            previous: Arc::new(AtomicIsize::new(paste::NO_WINDOW)),
            popup: Arc::new(AtomicBool::new(false)),
            popup_focused: Arc::new(AtomicBool::new(false)),
            popup_placing: Arc::new(AtomicBool::new(false)),
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
            rules: crate::rules::Rules::new(&cfg.rules),
            flags,
            library,
            backend: BackendState::Off,
            queue: 0,
            queue_total: 0,
            status: String::new(),
            error: None,
            toast: None,
            pending_delete: None,
            next_age_prune: None,
            query: String::new(),
            query_changed: None,
            query_seq: 0,
            query_vec: None,
            filter: Filter::All,
            results: Vec::new(),
            weaker: Vec::new(),
            weaker_shown_for: None,
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
            thumb_seen: HashMap::new(),
            folders_of: None,
            keyboard_focus: false,
            settings_typing: false,
            top_actions_w: 300.0,
            preview: None,
            loader: {
                let c = ctx.clone();
                ImageLoader::new(image_files, move || c.request_repaint())
            },
            programs: programs::Programs::new(ctx.clone()),
            paste_program: (paste::NO_WINDOW, None),
            reader_status: String::new(),
            window_state: cc.storage.and_then(|s| s.get_string(WINDOW_KEY)),
            start_size_checked: false,
            detail: None,
            research_pending: false,
            last_search: Instant::now(),
            hotkeys: None,
            hotkey_ids: Arc::default(),
            hotkeys_paused: false,
            hotkey_error: None,
            hotkey_conflicts: Vec::new(),
            model_list: ModelList::default(),
            settings_page: SettingsPage::Search,
            confirm: None,
            server_editor: None,
            pending_send: None,
            list_view: (0.0, 0.0),
            popup_list_view: (0.0, 0.0),
            popup_at: None,
            popup_hidden_last_frame: false,
            popup_moving: false,
            details_open: false,
            grid_cols: 1,
            update: UpdateState::Idle,
            update_rx: None,
            update_hidden: false,
            // A little after start, so launching stays quick.
            next_update_check: Instant::now() + Duration::from_secs(10),
            tag_filter: None,
            tag_input: None,
            item_menu: None,
            marked: HashSet::new(),
            anchor: None,
            group_tag: None,
            menu_by_key: false,
            keys_sheet: None,
            whats_new_pending: false,
            whats_new_open: false,
            by_key: false,
            taught: HashSet::new(),
            copy_hint: None,
            tag_focus: false,
            background: Background::spawn(ctx.clone()),
            imports_running: 0,
            imported: 0,
            recording: None,
            record_problem: None,
            cfg,
        };
        app.set_backend(app.cfg.search_backend());
        if app.cfg.reads_images() {
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
        if let Err(e) = crate::installed::refresh_version() {
            log::warn!("updating the version in Installed apps: {e:#}");
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
        if let Some(notes) = updated {
            let notes: Vec<String> = notes
                .iter()
                .map(|note| match note {
                    UpdateNote::BuiltInSearchRemoved => t!("toast-built-in-search-removed"),
                    UpdateNote::BuiltInReaderRemoved if cfg!(windows) => {
                        t!("toast-built-in-reader-removed")
                    }
                    UpdateNote::BuiltInReaderRemoved => t!("toast-built-in-reader-removed-off"),
                })
                .collect();
            if !notes.is_empty() {
                app.toast_for(notes.join("\n"), Duration::from_secs(15));
            }
            // Saved in the new form now, so the next start has nothing to update.
            app.save_config();
        }
        // The first start of a newer version says what is new (D47); settings
        // without a version come from a version before 0.6.
        let last = app.cfg.last_version.clone();
        if last != update::VERSION {
            app.whats_new_pending = had_settings && (last.is_empty() || update::newer_than(&last));
            // The checklist is for new installs (D53): a history that already
            // has items when it first could show skips it.
            if last.is_empty() && !app.library.items().is_empty() {
                app.cfg.checklist.closed = true;
            }
            app.cfg.last_version = update::VERSION.to_string();
            app.save_config();
        }
        if let Some(aside) = app.library.store().moved_aside.clone() {
            let message = log_error!(
                "error-history-moved-aside",
                file = aside.display().to_string()
            );
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
        let popup = parse(&self.cfg.popup_hotkey);
        let window = parse(&self.cfg.window_hotkey);
        let ignore = parse(&self.cfg.ignore_next_hotkey);
        let id = |h: Option<HotKey>| h.map_or(NO_HOTKEY, |h| u64::from(h.id()));
        let ids = &self.hotkey_ids;
        ids.popup.store(id(popup), Ordering::SeqCst);
        ids.window.store(id(window), Ordering::SeqCst);
        ids.ignore.store(id(ignore), Ordering::SeqCst);
        let (c, flags, focused, win, ids) = (
            ctx.clone(),
            self.flags.clone(),
            self.focused.clone(),
            self.win.clone(),
            self.hotkey_ids.clone(),
        );
        GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
            if e.state() != HotKeyState::Pressed {
                return;
            }
            let pressed = u64::from(e.id());
            if pressed == ids.popup.load(Ordering::SeqCst) {
                if win.is_popup_open() && win.popup_focused.load(Ordering::SeqCst) {
                    win.hide();
                } else {
                    win.show_popup(paste::window_in_front());
                }
            } else if pressed == ids.window.load(Ordering::SeqCst) {
                if win.is_popup_open() {
                    win.show_full();
                } else if focused.load(Ordering::SeqCst) && !win.hidden.load(Ordering::SeqCst) {
                    win.hide();
                } else {
                    win.show_over(paste::window_in_front());
                }
            } else if pressed == ids.ignore.load(Ordering::SeqCst) {
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
        // Versions before 0.6 saved a minimized window with no size, and
        // Windows then makes it tiny at start: it gets the first size back.
        if !self.start_size_checked {
            if let Some(inner) = ctx.input(|i| i.viewport().inner_rect) {
                self.start_size_checked = true;
                let [width, height] = SMALLEST_WINDOW;
                if inner.width() < width || inner.height() < height {
                    let size = Vec2::from(WINDOW_SIZE);
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                }
            }
        }
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
            self.item_menu = None;
            self.menu_by_key = false;
            self.keys_sheet = None;
            self.clear_marks();
            self.whats_new_open = false;
            // An error from while the window was hidden (such as after starting
            // at sign-in) gets its full time on screen.
            if let Some((_, at)) = &mut self.error {
                *at = Instant::now();
            }
            // Read now, so the main button names it when the window shows.
            let window = self.win.previous.load(Ordering::SeqCst);
            if paste::exists(window) {
                let _ = self.paste_target_name(window);
            }
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

    /// Turns off Send images for the search model in use, unless another
    /// model is in use by now.
    fn stop_sending_images(&mut self, key: &str) {
        if self.ready_key() != Some(key) {
            return;
        }
        let mut d = self.cfg.clone();
        if let Some(search) = d.search_settings_mut() {
            search.images = false;
        }
        // Open Settings applies its draft every frame, which would undo this.
        if let Some(search) = self.draft.as_mut().and_then(Config::search_settings_mut) {
            search.images = false;
        }
        let ctx = self.win.ctx.clone();
        self.apply_settings(d, &ctx, false);
        self.toast(t!("toast-images-turned-off"));
    }

    /// `run` for a key press: its message doesn't name the key.
    fn run_by_key(&mut self, command: Command) {
        self.by_key = true;
        self.run(command);
        self.by_key = false;
    }

    /// The key to name in the message after a mouse action, the first time
    /// in this run; `None` after a key press, once it was named, or when
    /// the action has no key.
    fn key_to_teach(&mut self, keys: &str) -> Option<String> {
        let first = !self.by_key && !keys.trim().is_empty() && self.taught.insert(keys.to_string());
        first.then(|| keys.to_string())
    }

    /// Carries out what the user asked for.
    fn run(&mut self, command: Command) {
        match command {
            Command::Select(id) => {
                self.selected = Some(id);
                self.anchor = Some(id);
                self.clear_marks();
            }
            Command::ToggleMark(id) => self.toggle_mark(id),
            Command::MarkRange(id) => self.mark_range(id),
            Command::PinGroup(ids) => self.pin_group(&ids),
            Command::DeleteGroup(ids) => self.delete_group(&ids),
            Command::CopyTogether(ids, paste) => self.copy_together(&ids, paste),
            Command::Copy(id, paste) => {
                self.selected = Some(id);
                let k = &self.cfg.keys;
                let keys = match paste {
                    Paste::Yes => &k.paste,
                    Paste::Plain => &k.paste_plain,
                    Paste::No => &k.copy_only,
                };
                self.copy_hint = (!self.by_key).then(|| keys.clone());
                self.copy(id, paste);
            }
            Command::Delete(id) => self.delete(id),
            Command::Pin(id) => {
                self.toggle_pin(id);
                // Pinning has no message of its own; this one teaches the key.
                let keys = self.cfg.keys.pin.clone();
                if let Some(keys) = self.key_to_teach(&keys) {
                    let text = if self.library.get(id).is_some_and(|it| it.pinned) {
                        t!("toast-pinned-key", keys = keys)
                    } else {
                        t!("toast-unpinned-key", keys = keys)
                    };
                    self.toast_for(text, KEY_HINT_FOR);
                }
            }
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
            Command::SaveSkipped(capture) => {
                self.record(capture);
                self.toast(t!("toast-saved"));
            }
            Command::StopSendingImages(key) => self.stop_sending_images(&key),
            Command::ShowWeaker => self.show_weaker(),
            Command::ShowWhatsNew => {
                self.win.show_full();
                self.whats_new_open = true;
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
        if kind == Kind::Image && self.cfg.reads_images() {
            jobs.push(Job::Ocr(id));
        }
        if self.ready_key().is_some() {
            jobs.push(Job::Embed(id));
        }
        jobs
    }

    /// A copy from the clipboard: recorded, unless the secret filter or a
    /// rule holds it back.
    fn on_capture(&mut self, capture: Capture) {
        let Capture {
            content,
            app,
            program,
            formats,
        } = capture;
        if let Captured::Skipped { reason, text } = content {
            let message = match reason {
                SkipReason::Private => t!("toast-skipped-private"),
                SkipReason::Secret => t!("toast-skipped-secret"),
            };
            let kept = text.map(|t| Capture {
                content: Captured::Text(t),
                app,
                program,
                formats,
            });
            self.skipped(message, kept);
            return;
        }
        let text = match &content {
            Captured::Text(t) => Some(t.as_str()),
            _ => None,
        };
        if self.rules.skips(app.as_deref(), text) {
            log::info!("copy not saved: a rule matched");
            let kept = Capture {
                content,
                app,
                program,
                formats,
            };
            self.skipped(t!("toast-skipped-rule"), Some(kept));
            return;
        }
        self.record(Capture {
            content,
            app,
            program,
            formats,
        });
    }

    /// Stops recording copies from `app`, with a rule the user can remove
    /// in Settings.
    fn ignore_app(&mut self, app: &str) {
        let mut d = self.cfg.clone();
        d.rules.push(crate::rules::Rule {
            app: app.to_string(),
            ..Default::default()
        });
        let ctx = self.win.ctx.clone();
        self.apply_settings(d, &ctx, false);
        self.toast_for(t!("toast-app-ignored", app = app), Duration::from_secs(6));
    }

    /// The ways to put an item back, the main one first: paste into the
    /// program the window opened over, or only copy when there is none.
    /// Without formatting to leave out, there is no plain-text way.
    fn ways_to_copy(&mut self, it: &Item) -> Vec<CopyWay> {
        let keys = self.cfg.keys.clone();
        let way = |text: String, keys: String, paste: Paste| CopyWay { text, keys, paste };
        let window = self.win.previous.load(Ordering::SeqCst);
        if !paste::exists(window) {
            let mut ways = vec![way(t!("item-copy"), keys.copy_only, Paste::No)];
            if it.formatted {
                ways.push(way(t!("item-copy-plain"), keys.paste_plain, Paste::Plain));
            }
            return ways;
        }
        let paste = match self.paste_target_name(window) {
            Some(name) => t!("item-paste-into", app = name.as_str()),
            None => t!("item-paste"),
        };
        let mut ways = vec![way(paste, keys.paste, Paste::Yes)];
        if it.formatted {
            ways.push(way(t!("item-paste-plain"), keys.paste_plain, Paste::Plain));
        }
        ways.push(way(t!("item-copy-only"), keys.copy_only, Paste::No));
        ways
    }

    /// The own name of the program `window` belongs to, once it is known.
    fn paste_target_name(&mut self, window: paste::Window) -> Option<String> {
        if self.paste_program.0 != window {
            self.paste_program = (window, paste::program(window));
        }
        let path = self.paste_program.1.clone()?;
        self.programs.get(&path).map(|p| p.name.clone())
    }

    /// Pasting goes nowhere once the user is in another program: until a
    /// shortcut opens this app again, pasting only copies.
    fn left_for_another_program(&self) {
        if paste::window_in_front() != paste::NO_WINDOW {
            self.win.previous.store(paste::NO_WINDOW, Ordering::SeqCst);
        }
    }

    /// Says a copy wasn't recorded. `kept` is offered back with "Save anyway".
    fn skipped(&mut self, message: String, kept: Option<Capture>) {
        // Longer when there is a button to reach.
        let how_long = Duration::from_secs(if kept.is_some() { 8 } else { 4 });
        self.toast_for(message, how_long);
        if let Some(toast) = &mut self.toast {
            toast.action = kept.map(|c| (t!("toast-save-anyway"), Command::SaveSkipped(c)));
        }
    }

    /// Adds a copy to the history, or moves its duplicate to the top.
    fn record(&mut self, capture: Capture) {
        let Capture {
            content: c,
            app,
            program,
            formats,
        } = capture;
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
        item.app = app;
        item.program = program;
        match c {
            Captured::Text(t) => {
                // Saved before the item, which then says it has them.
                if let Some(formats) = &formats {
                    match self.library.store().put_formats(id, formats) {
                        Ok(()) => item.formatted = true,
                        Err(e) => log::warn!("keeping the formatting failed: {e:#}"),
                    }
                }
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
            Captured::Skipped { .. } => return,
            Captured::Image(img) => {
                item.kind = Kind::Image;
                item.width = img.width();
                item.height = img.height();
                // Writing the PNG takes a while for a big image: the item joins
                // the history once the file is saved (`Done::ImageSaved`).
                let store = self.library.store().clone();
                self.background.run(move || {
                    let saved = store.images.save(id, &img).map_err(|e| format!("{e:#}"));
                    Done::ImageSaved {
                        item: Box::new(item),
                        saved,
                    }
                });
                return;
            }
        }
        self.add_item(item);
        self.prune();
    }

    /// Deletes what is over the history size and, when it is on, what is
    /// older than the chosen number of days.
    fn prune(&mut self) {
        let days = i64::from(self.cfg.delete_after_days);
        let pruned = self.library.prune(self.cfg.max_items).and_then(|mut ids| {
            if days > 0 {
                let cutoff = crate::store::now() - days * 24 * 60 * 60;
                ids.extend(self.library.prune_older_than(cutoff)?);
            }
            Ok(ids)
        });
        match pruned {
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
                self.add_item(*item);
                self.prune();
            }
            Done::ImageSaved { saved: Err(e), .. } => {
                self.set_error(log_error!("error-saving-image", error = e.to_string()))
            }
            Done::Copied {
                id,
                paste: how,
                result: Ok(()),
            } => {
                if let Err(e) = self.library.record_use(id) {
                    self.set_error(log_error!("error-saving-item", error = e.to_string()));
                }
                self.dirty = true;
                let target = self.win.previous.load(Ordering::SeqCst);
                let hint = self.copy_hint.take();
                if how == Paste::No || !paste::exists(target) {
                    // Named only where the message can be seen.
                    let hint = hint.filter(|keys| {
                        !self.cfg.minimize_after_copy
                            && !keys.trim().is_empty()
                            && self.taught.insert(keys.clone())
                    });
                    match hint {
                        Some(keys) => {
                            self.toast_for(t!("toast-copied-key", keys = keys), KEY_HINT_FOR)
                        }
                        None => self.toast(t!("toast-copied")),
                    }
                    if self.cfg.minimize_after_copy {
                        self.win.hide();
                    }
                } else if paste::is_out_of_reach(target) {
                    // Windows doesn't let this app press keys there: the
                    // window stays to say so, and the user pastes.
                    self.toast_for(t!("toast-copied-admin"), Duration::from_secs(8));
                } else {
                    self.win.hide();
                    if !self.cfg.checklist.pasted {
                        self.cfg.checklist.pasted = true;
                        self.save_config();
                    }
                    std::thread::spawn(move || {
                        if let Err(e) = paste::paste_into(target) {
                            log::warn!("pasting failed, the item is copied: {e:#}");
                        }
                    });
                }
            }
            Done::Copied { result: Err(e), .. } => {
                self.set_error(log_error!("error-copying", error = e.to_string()))
            }
            Done::TextCopied(Ok(())) => self.toast(t!("toast-copied-text")),
            Done::CopiedTogether {
                count,
                pictures,
                paste: how,
                result: Ok(()),
            } => {
                let target = self.win.previous.load(Ordering::SeqCst);
                if how != Paste::No && paste::exists(target) && !paste::is_out_of_reach(target) {
                    self.win.hide();
                    if !self.cfg.checklist.pasted {
                        self.cfg.checklist.pasted = true;
                        self.save_config();
                    }
                    std::thread::spawn(move || {
                        if let Err(e) = paste::paste_into(target) {
                            log::warn!("pasting failed, the items are copied: {e:#}");
                        }
                    });
                } else {
                    let text = if pictures > 0 {
                        t!(
                            "toast-copied-group-pictures",
                            count = count,
                            pictures = pictures
                        )
                    } else {
                        t!("toast-copied-group", count = count)
                    };
                    self.toast_for(text, Duration::from_secs(4));
                }
            }
            Done::CopiedTogether { result: Err(e), .. } => {
                self.set_error(log_error!("error-copying", error = e.to_string()))
            }
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
                Event::ReaderStatus(s) => self.reader_status = s,
                Event::Error(s) => self.set_error(s),
                Event::Queue(n) => {
                    self.queue_total = run_total(self.queue_total, self.queue, n);
                    self.queue = n;
                }
                Event::BackendOff => {
                    self.backend = BackendState::Off;
                    self.status.clear();
                }
                Event::ImagesRefused { key } => {
                    // Images get vectors of their text from now on, so text
                    // read later is indexed too.
                    let current = match &mut self.backend {
                        BackendState::Ready { key: ready, images } if *ready == key => {
                            *images = false;
                            true
                        }
                        _ => false,
                    };
                    if current {
                        self.toast_for(t!("toast-images-refused"), IMAGES_REFUSED_FOR);
                        if let Some(toast) = &mut self.toast {
                            toast.action =
                                Some((t!("toast-images-off"), Command::StopSendingImages(key)));
                        }
                    }
                }
                Event::BackendReady {
                    key,
                    images,
                    stored,
                } => {
                    // Learn the server's default model name for the status bar.
                    if let EmbedBackend::Server(sc) = self.cfg.search_backend() {
                        if sc.model.trim().is_empty()
                            && self.model_list.pending.is_none()
                            && self.model_list.url != ModelList::key(&sc.base_url, &sc.api_key)
                        {
                            self.model_list.fetch(&sc.base_url, &sc.api_key, false, ctx);
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
        // Weaker matches wait under one line, unless shown for this search.
        if self.weaker_shown_for.as_deref() == Some(self.query.trim()) {
            (self.results, self.weaker) = (hits, Vec::new());
        } else {
            (self.results, self.weaker) = hits.into_iter().partition(|h| !h.weak);
        }
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
        self.weaker.retain(|h| !gone.contains(&h.id));
        // With every strong match gone, the weaker ones are all there is.
        if self.results.is_empty() {
            self.results.append(&mut self.weaker);
        }
        for id in ids {
            self.thumbs.remove(id);
            self.thumb_seen.remove(id);
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

    /// Puts the item on the clipboard in the background; `finish` reports it
    /// and pastes it.
    fn copy(&mut self, id: u64, paste: Paste) {
        let Some(item) = self.library.get(id).cloned() else {
            return;
        };
        let flags = self.flags.clone();
        let store = self.library.store().clone();
        self.background.run(move || {
            let formats = if item.formatted && paste != Paste::Plain {
                store.formats(id).unwrap_or_else(|e| {
                    log::warn!("reading the formatting failed: {e:#}");
                    None
                })
            } else {
                None
            };
            let result = write_clipboard(&flags, || {
                clipboard::copy_item(&item, &store.images, formats.as_ref())
            });
            Done::Copied { id, paste, result }
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
            removed: vec![removed],
            since: Instant::now(),
        });
        let keys = self.cfg.keys.delete.clone();
        let text = match self.key_to_teach(&keys) {
            Some(keys) => t!("toast-deleted-key", keys = keys),
            None => t!("toast-deleted"),
        };
        self.toast_for(text, UNDO_FOR);
        if let Some(toast) = &mut self.toast {
            toast.action = Some((t!("toast-undo"), Command::UndoDelete));
        }
    }

    /// Makes the pending delete final: database row, vectors and image file.
    pub(super) fn finish_delete(&mut self) {
        if let Some(pending) = self.pending_delete.take() {
            for removed in pending.removed {
                if let Err(e) = self.library.forget(removed) {
                    self.set_error(log_error!("error-deleting", error = e.to_string()));
                }
            }
        }
    }

    fn undo_delete(&mut self) {
        let Some(pending) = self.pending_delete.take() else {
            return;
        };
        // The first in the list was taken last.
        self.selected = pending.removed.last().map(|r| r.item.id);
        for removed in pending.removed.into_iter().rev() {
            self.library.put_back(removed);
        }
        self.dirty = true;
    }

    /// The items selected together, in list order; empty while only one
    /// item is selected.
    pub(super) fn group(&self) -> Vec<u64> {
        if self.marked.len() < 2 {
            return Vec::new();
        }
        let shown = self.results.iter().map(|h| h.id);
        let group: Vec<u64> = shown.filter(|id| self.marked.contains(id)).collect();
        if group.len() < 2 {
            return Vec::new();
        }
        group
    }

    /// Back to one item selected.
    fn clear_marks(&mut self) {
        self.marked.clear();
        self.group_tag = None;
    }

    fn toggle_mark(&mut self, id: u64) {
        if self.marked.is_empty() {
            self.marked.extend(self.selected);
        }
        if !self.marked.remove(&id) {
            self.marked.insert(id);
        }
        self.selected = Some(id);
        self.anchor = Some(id);
    }

    /// Selects every item from where the range starts to `id`, in list order.
    fn mark_range(&mut self, id: u64) {
        let from = self.anchor.or(self.selected).unwrap_or(id);
        let place = |x: u64| self.results.iter().position(|h| h.id == x);
        if let (Some(a), Some(b)) = (place(from), place(id)) {
            let range = &self.results[a.min(b)..=a.max(b)];
            self.marked = range.iter().map(|h| h.id).collect();
        }
        self.anchor = Some(from);
        self.selected = Some(id);
    }

    /// Ctrl+A: every item shown.
    fn mark_all(&mut self) {
        self.marked = self.results.iter().map(|h| h.id).collect();
        if self.selected.is_none() {
            self.selected = self.results.first().map(|h| h.id);
        }
    }

    /// Pins every item of `ids`, or unpins them all when all are pinned.
    fn pin_group(&mut self, ids: &[u64]) {
        let pinned = |app: &Self, id: u64| app.library.get(id).is_some_and(|it| it.pinned);
        let pin = ids.iter().any(|&id| !pinned(self, id));
        for &id in ids {
            if pinned(self, id) != pin {
                self.toggle_pin(id);
            }
        }
        self.toast(if pin {
            t!("toast-pinned-group", count = ids.len())
        } else {
            t!("toast-unpinned-group", count = ids.len())
        });
    }

    /// Deletes every item of `ids`; one Undo brings them all back.
    fn delete_group(&mut self, ids: &[u64]) {
        // One undo at a time: an earlier delete becomes final now.
        self.finish_delete();
        let place: HashMap<u64, usize> = self
            .library
            .items()
            .iter()
            .enumerate()
            .map(|(i, it)| (it.id, i))
            .collect();
        let mut ids: Vec<u64> = ids
            .iter()
            .copied()
            .filter(|id| place.contains_key(id))
            .collect();
        // Taken from the last to the first, each keeps its place.
        ids.sort_by_key(|id| std::cmp::Reverse(place[id]));
        let row = self.results.iter().position(|h| ids.contains(&h.id));
        let mut removed = Vec::with_capacity(ids.len());
        for &id in &ids {
            if let Some(r) = self.library.take(id) {
                self.loader.forget(id, false);
                removed.push(r);
            }
        }
        self.drop_stale_tag_filter();
        self.drop_removed(&ids);
        self.clear_marks();
        // Keep the selection on the row that takes the first one's place.
        if let Some(i) = row {
            let last = self.results.len().saturating_sub(1);
            self.selected = self.results.get(i.min(last)).map(|h| h.id);
        }
        let count = removed.len();
        self.pending_delete = Some(PendingDelete {
            removed,
            since: Instant::now(),
        });
        self.toast_for(t!("toast-deleted-group", count = count), UNDO_FOR);
        if let Some(toast) = &mut self.toast {
            toast.action = Some((t!("toast-undo"), Command::UndoDelete));
        }
        self.dirty = true;
    }

    /// Puts the texts, links and file paths of `ids` on the clipboard
    /// together, one after another in list order, and maybe pastes them.
    /// Pictures are left out.
    fn copy_together(&mut self, ids: &[u64], paste: Paste) {
        let items = ids.iter().filter_map(|&id| self.library.get(id));
        let (count, text, pictures) = texts_together(items);
        if count == 0 {
            self.toast(t!("toast-pictures-not-together"));
            return;
        }
        let flags = self.flags.clone();
        self.background.run(move || Done::CopiedTogether {
            count,
            pictures,
            paste,
            result: write_clipboard(&flags, || clipboard::copy_text(&text)),
        });
    }

    /// Adds the tags typed in the bar to every item selected together.
    pub(super) fn tag_group(&mut self, tags: &str) {
        let group = self.group();
        for &id in &group {
            self.run(Command::AddTags(id, tags.to_string()));
        }
        let tag = tags.trim().trim_start_matches('#');
        self.toast(t!("toast-tag-added-group", tag = tag, count = group.len()));
        self.group_tag = None;
    }

    /// Shows the weaker matches folded below the results, until the search
    /// changes.
    fn show_weaker(&mut self) {
        self.weaker_shown_for = Some(self.query.trim().to_string());
        self.results.append(&mut self.weaker);
    }

    fn move_selection(&mut self, delta: i32) {
        if self.results.is_empty() {
            return;
        }
        let cur = self
            .selected
            .and_then(|id| self.results.iter().position(|h| h.id == id));
        // Going on from the last result shows the weaker matches below it.
        if delta > 0 && cur == Some(self.results.len() - 1) && !self.weaker.is_empty() {
            self.show_weaker();
        }
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

/// The size of an indexing run when the worker reports `left` items to go
/// (it reported `before` last time). Work added meanwhile adds to the
/// run, so its progress bar never goes back; an empty queue ends it.
fn run_total(total: usize, before: usize, left: usize) -> usize {
    if left == 0 {
        0
    } else {
        total + left.saturating_sub(before)
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
mod programs;
mod settings;
mod shortcut;
mod view;

use background::{Background, Done};
use settings::{PendingSend, ServerEditor, SettingsPage};
use view::{add_fonts, apply_theme, looks_like_code, set_accent, style};

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::Storage as _;

    #[test]
    fn an_indexing_run_grows_with_new_work_and_ends_when_empty() {
        let (mut total, mut left) = (0, 0);
        let mut seen = Vec::new();
        for next in [500, 400, 450, 10, 0, 3] {
            total = run_total(total, left, next);
            left = next;
            seen.push((total - left, total));
        }
        // (done, total) after each report: new work never takes "done" back.
        assert_eq!(
            seen,
            [(0, 500), (100, 500), (100, 550), (540, 550), (0, 0), (0, 3)]
        );
    }

    /// app.ron in memory.
    struct Entries(HashMap<String, String>);

    impl eframe::Storage for Entries {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_string(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }

    #[test]
    fn copy_together_joins_texts_in_order_and_leaves_pictures_out() {
        let item =
            |id: u64, kind: Kind, text: &str| Item::new(id, kind, Origin::Clipboard, text.into());
        let items = [
            item(1, Kind::Text, "first line\n"),
            item(2, Kind::Image, ""),
            item(3, Kind::Link, "https://example.com"),
            item(4, Kind::Files, "C:\\a.txt\nC:\\b.txt"),
        ];
        let joined = "first line\nhttps://example.com\nC:\\a.txt\nC:\\b.txt";
        assert_eq!(texts_together(items.iter()), (3, joined.to_string(), 1));
        assert_eq!(texts_together(items[1..2].iter()), (0, String::new(), 1));
    }

    #[test]
    fn a_minimized_window_keeps_its_place_and_size() {
        let mut storage = Entries(HashMap::new());
        let mut kept = None;
        // What eframe saves before the app's `save`, then what the app leaves.
        let mut save = |entry: &str, minimized: bool| {
            storage.set_string(WINDOW_KEY, entry.to_string());
            keep_window_state(&mut storage, minimized, &mut kept);
            storage.get_string(WINDOW_KEY).unwrap()
        };
        assert_eq!(save("1040 by 680 at 32,32", false), "1040 by 680 at 32,32");
        assert_eq!(save("0 by 0 at -32000", true), "1040 by 680 at 32,32");
        assert_eq!(save("1200 by 900 at 40,40", false), "1200 by 900 at 40,40");
    }
}
