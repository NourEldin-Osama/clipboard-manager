//! The egui user interface.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
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
    Config, EmbedBackend, LaunchConfig, Paths, SearchEngine, SearchMode, SortOrder, Theme, ViewMode,
};
use crate::download::Progress;
use crate::embed::Embedding;
use crate::i18n::{self, log_error, t};
use crate::images::{ImageLoader, Size};
use crate::library::{Library, Removed};
use crate::paste;
use crate::search::{self, Filter, Hit, Searcher};
use crate::store::{looks_like_link, Item, Kind, Origin, Store};
use crate::tray::{Tray, TrayCmd, TrayLook};
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

/// Model servers found on this PC (D66), asked again at most every 30
/// seconds while a screen that offers them shows.
#[derive(Default)]
struct LocalServers {
    found: Vec<crate::embed::local::LocalServer>,
    asked: Option<Instant>,
    pending: Option<Receiver<Vec<crate::embed::local::LocalServer>>>,
}

impl LocalServers {
    const AGAIN: Duration = Duration::from_secs(30);

    /// Looks for them, unless that is running or was done just now.
    fn look(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() || self.asked.is_some_and(|t| t.elapsed() < Self::AGAIN) {
            return;
        }
        let (tx, rx) = channel();
        let wake = ctx.clone();
        let started = std::thread::Builder::new()
            .name("local-servers".into())
            .spawn(move || {
                let _ = tx.send(crate::embed::local::find());
                wake.request_repaint();
            });
        if started.is_ok() {
            self.asked = Some(Instant::now());
            self.pending = Some(rx);
        }
    }

    fn poll(&mut self) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(found) => {
                    self.found = found;
                    self.pending = None;
                }
                Err(TryRecvError::Disconnected) => self.pending = None,
                Err(TryRecvError::Empty) => {}
            }
        }
    }
}

/// The models each saved server lists, for the model menus (D67): asked once
/// each time Settings opens.
#[derive(Default)]
struct ServerLists {
    /// By the server's address and key.
    found: HashMap<String, Vec<String>>,
    pending: HashMap<String, Receiver<Vec<String>>>,
}

impl ServerLists {
    fn key(server: &crate::config::Server) -> String {
        format!("{}|{}", server.base_url.trim(), server.api_key.trim())
    }

    /// Asks each server not asked yet.
    fn ask(&mut self, servers: &[crate::config::Server], ctx: &egui::Context) {
        for server in servers {
            let key = Self::key(server);
            if self.found.contains_key(&key) || self.pending.contains_key(&key) {
                continue;
            }
            let (tx, rx) = channel();
            let (url, api_key, kind) =
                (server.base_url.clone(), server.api_key.clone(), server.kind);
            let wake = ctx.clone();
            let started = std::thread::Builder::new()
                .name("server-models".into())
                .spawn(move || {
                    let models = match crate::embed::local::models_at(&url, &api_key, kind) {
                        crate::embed::local::Listed::Models(models) => models,
                        _ => Vec::new(),
                    };
                    let _ = tx.send(models);
                    wake.request_repaint();
                });
            if started.is_ok() {
                self.pending.insert(key, rx);
            }
        }
    }

    fn poll(&mut self) {
        let mut done = Vec::new();
        for (key, rx) in &self.pending {
            match rx.try_recv() {
                Ok(models) => done.push((key.clone(), models)),
                Err(TryRecvError::Disconnected) => done.push((key.clone(), Vec::new())),
                Err(TryRecvError::Empty) => {}
            }
        }
        for (key, models) in done {
            self.pending.remove(&key);
            self.found.insert(key, models);
        }
    }

    /// What `server` listed, once it answered.
    fn models(&self, server: &crate::config::Server) -> &[String] {
        self.found
            .get(&Self::key(server))
            .map_or(&[], |models| models.as_slice())
    }
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
    /// Set by `show_full` on any thread: the next frame shows the full
    /// window from the app's own thread, as the tray's Show does.
    show_wanted: Arc<AtomicBool>,
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
        // Shown by the next frame's `window_logic`, on the app's own thread:
        // shown straight from another thread (a second start, a shortcut),
        // the window sometimes stayed black until the tray's Show.
        self.show_wanted.store(true, Ordering::SeqCst);
        self.focus_search.store(true, Ordering::SeqCst);
        self.ctx.request_repaint();
    }

    /// Shows the full window now; only on the app's own thread, in a frame.
    fn show_now(&self) {
        self.main_window(egui::ViewportCommand::Visible(true));
        self.main_window(egui::ViewportCommand::Minimized(false));
        self.main_window(egui::ViewportCommand::Focus);
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

/// How many bytes of an update's download are done, and of how many: the
/// release's size until the server names it.
fn download_amounts(r: &Release, p: &Progress) -> (u64, u64) {
    let total = match p.total.load(Ordering::Relaxed) {
        0 => r.size,
        total => total,
    };
    (p.done.load(Ordering::Relaxed), total.max(1))
}

/// What a button of the update's message, or of the Version row in
/// Settings, About, asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum UpdateAction {
    /// Hides the message until the update's next step.
    Close,
    Skip,
    Download,
    Cancel,
    Restart,
    /// Downloads again after a failed download, or checks again after a
    /// failed check.
    TryAgain,
    WhatsNew,
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

/// A short message at the bottom of the window (D55), with an optional
/// button. Up to three show at once, the newest in front.
struct Toast {
    text: String,
    /// A second, quieter line.
    detail: Option<String>,
    icon: Option<icons::Icon>,
    /// How long it shows. Its time runs only while a window shows it and
    /// the pointer is not on the messages.
    how_long: Duration,
    shown_for: Duration,
    /// The button's text and what it does.
    action: Option<(String, Command)>,
    /// The keys that do the same as the button, shown in it.
    keys: Option<String>,
    /// The whole text of a copy that wasn't saved, shown while Show is held.
    secret: Option<String>,
    /// Shown in the note at the screen's corner, not in the window (D56).
    on_screen: bool,
    /// Drawn in the window at least once: it has slid in.
    drawn: bool,
    /// Tells the messages apart, for egui.
    id: u64,
    /// Makes this the update's message (D65): its text, buttons and progress
    /// follow `App::update`, and it lasts until the update is done or closed.
    update: Option<view::UpdateParts>,
    /// Makes this a question that waits for its answer: its button, or
    /// Cancel, Esc or its window closing, which answer no.
    question: Option<view::Question>,
}

impl Toast {
    /// The number of the delete its button brings back, if it has one.
    fn undoes(&self) -> Option<u64> {
        match self.action {
            Some((_, Command::UndoDelete(number))) => Some(number),
            _ => None,
        }
    }
}

/// At most this many messages show at once; a newer one pushes out the
/// oldest.
const MAX_TOASTS: usize = 3;

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
    /// Brings back the items of one delete, by its number in the undo
    /// history: its message's Undo.
    UndoDelete(u64),
    /// Brings back the items of the newest delete that can still be undone:
    /// Ctrl+Z.
    UndoNewestDelete,
    /// Records a copy that the secret filter or a rule held back.
    SaveSkipped(Capture),
    /// Copies text read from an image; it is not recorded as a new copy.
    CopyText(String),
    /// Turns off Send images for the search model with this key, after its
    /// server refused an image.
    StopSendingImages(String),
    /// Opens the weaker matches below their link, under the strong ones.
    ShowWeaker,
    /// Folds the weaker matches back under their link.
    HideWeaker,
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
    /// Starts the clipboard watcher again after it stopped on crashes.
    StartRecordingAgain,
    /// Asks the search server again now, instead of at the next retry.
    TrySearchAgain,
    /// Reads an image's text again after it failed.
    ReadAgain(u64),
    /// Opens Settings at a page.
    OpenSettings(settings::SettingsPage),
    /// Records copies from a program again: Undo after its rule was added.
    RecordAppAgain(String),
    /// Stops recording copies from a program (its file name, and its name
    /// as shown) with a rule: Never record, the answer to the question
    /// the item's menu asks.
    IgnoreApp(String, String),
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
    Details,
}

/// How long the message after a delete shows. Its delete can be undone
/// for longer (`undo::UNDO_WINDOW`).
const DELETED_MESSAGE_FOR: Duration = Duration::from_secs(8);
/// How long rows and cards glide after a delete, and the time each glide
/// takes.
const GLIDE_FOR: Duration = Duration::from_millis(400);
const GLIDE_TIME: f32 = 0.2;
/// The key that undoes the newest delete, named on its message's Undo.
const UNDO_KEYS: &str = "Ctrl+Z";
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

/// `cfg` with a rule that stops recording copies from the program `app`.
fn ignoring_app(cfg: &Config, app: &str) -> Config {
    let mut d = cfg.clone();
    d.rules.push(crate::rules::Rule {
        app: app.to_string(),
        ..Default::default()
    });
    d
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
/// open it tiny, in a corner of the screen. A window showing a picture full
/// screen would start full screen. So while the window's state is one of
/// these temporary ones, the entry saved before goes back in; otherwise the
/// new entry is kept.
fn keep_window_state(
    storage: &mut dyn eframe::Storage,
    is_temporary: bool,
    kept: &mut Option<String>,
) {
    if !is_temporary {
        *kept = storage.get_string(WINDOW_KEY);
    } else if let Some(entry) = kept {
        storage.set_string(WINDOW_KEY, entry.clone());
    }
}

/// How long the message about a server that refused an image stays.
const IMAGES_REFUSED_FOR: Duration = Duration::from_secs(15);
/// How often deleting by age runs while it is on.
const AGE_PRUNE_EVERY: Duration = Duration::from_secs(60 * 60);
/// How long a shown window that hasn't drawn yet waits before it is shown
/// again, and for how long after it was first shown.
const SHOW_AGAIN_AFTER: Duration = Duration::from_millis(250);
const SHOW_CHECK_FOR: Duration = Duration::from_secs(3);

/// A search server that stopped answering is asked again after this long,
/// then twice as long each time, up to `RETRY_AT_MOST`.
const RETRY_FIRST: Duration = Duration::from_secs(30);
const RETRY_AT_MOST: Duration = Duration::from_secs(10 * 60);

/// When to ask a search server that stopped answering again.
struct Retry {
    at: Instant,
    wait: Duration,
}

impl Retry {
    fn new(now: Instant) -> Self {
        Retry {
            at: now + RETRY_FIRST,
            wait: RETRY_FIRST,
        }
    }

    /// After a try at `now`: the next one comes twice as late.
    fn again(&mut self, now: Instant) {
        self.wait = (self.wait * 2).min(RETRY_AT_MOST);
        self.at = now + self.wait;
    }
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
    /// The clipboard watcher stopped after crashing again and again; the
    /// status bar offers to start it again.
    recording_stopped: bool,
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
    /// The messages showing, the oldest first.
    toasts: Vec<Toast>,
    /// The last message's id.
    toast_seq: u64,
    /// The pointer is on the messages: they open up and their time stops.
    toasts_hovered: bool,
    /// When the messages' time last ran.
    toasts_tick: Option<Instant>,
    /// The message whose Show is held down: its copy's whole text shows.
    toast_showing: Option<u64>,
    /// The pointer is on the note at the screen's corner: its time stops.
    note_hovered: bool,
    /// The note's height for its message, learned as it draws.
    note_height: f32,
    /// Where the note's window is, from when it appeared.
    note_at: Option<egui::Pos2>,
    /// The note's window, as Windows' handle for it, once found: its
    /// style is kept in every frame.
    note_window_handle: Option<isize>,
    /// The app icon, for the note.
    app_icon: Option<egui::TextureHandle>,
    /// Alt+C was pressed and its copy hasn't come yet.
    skip_next_armed: bool,
    /// Until when rows and cards glide to their new places: after a delete
    /// or its Undo, so the gap closes instead of jumping.
    glide_until: Option<Instant>,
    /// The deletes that can still be undone, with their items.
    undo_history: UndoHistory,
    /// When deleting by age runs next; `None` runs it at the next frame.
    next_age_prune: Option<Instant>,
    /// The search server stopped answering: when the app asks it again.
    search_retry: Option<Retry>,
    /// Images whose text couldn't be read; they offer Try again.
    ocr_failed: HashSet<u64>,

    query: String,
    query_changed: Option<Instant>,
    query_seq: u64,
    query_vec: Option<(String, Arc<Embedding>)>,
    filter: Filter,
    results: Vec<Hit>,
    /// While searching, how many results each filter and tag would show.
    search_counts: Option<search::Counts>,
    /// Weaker matches, folded under their link below the results.
    weaker: Vec<Hit>,
    /// The item the app selected by itself for the results: until another
    /// is picked, a better ranking that comes later moves the selection to
    /// the best match.
    auto_selected: Option<u64>,
    /// For which search text the weaker matches are open below their link.
    weaker_link: WeakerLink,
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
    /// The picture shown full screen, while it is.
    viewer: Option<view::Viewer>,
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
    /// Model servers found on this PC (D66).
    local_servers: LocalServers,
    /// What the saved servers list, for the model menus (D67).
    server_lists: ServerLists,
    settings_page: SettingsPage,
    confirm: Option<Confirm>,
    /// The server being added or edited in Settings.
    server_editor: Option<ServerEditor>,
    /// Servers' editors left with changes not saved, by server id (D61):
    /// kept until Save, Discard or quitting the app.
    server_drafts: HashMap<u32, ServerEditor>,
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
    /// With cards as tall as their content, or with weaker matches on rows
    /// of their own, where each card was drawn last, from the grid's top
    /// left, for the arrow keys. Empty otherwise.
    grid_places: Vec<(u64, egui::Rect)>,
    /// The heights of cards as tall as their content, measured once for
    /// each layout of the grid.
    card_heights: view::CardHeights,
    /// Touchpad scrolling between frames: when it last came, and a step
    /// that may be a swipe's late coasting.
    touchpad: view::Touchpad,
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
    /// A shown window that hasn't drawn a frame yet: when it was first
    /// shown, and when it was last asked to show.
    show_check: Option<(Instant, Instant)>,
    /// The shortcut field listening for keys in Settings, and why the last try was refused.
    recording: Option<ShortcutField>,
    record_problem: Option<String>,
    update: UpdateState,
    update_rx: Option<Receiver<UpdateMsg>>,
    /// The close button hides the update's message until the update's next
    /// step.
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
        let (cfg, settings_problem) = Config::load(&paths.config);
        i18n::set(cfg.language, cfg.time_format);
        add_fonts(&ctx);
        icons::register(&ctx);
        set_accent(cfg.accent);
        style(&ctx, cfg.dark_panels);
        apply_theme(&ctx, cfg.theme);

        let store = Arc::new(Store::open(&paths.db, &paths.images)?);
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
        let captures = watch_clipboard(&ctx, &flags);

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
            show_wanted: Arc::new(AtomicBool::new(false)),
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
            show_check: None,
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
            recording_stopped: false,
            rules: crate::rules::Rules::new(&cfg.rules),
            flags,
            library,
            backend: BackendState::Off,
            queue: 0,
            queue_total: 0,
            status: String::new(),
            error: None,
            toasts: Vec::new(),
            toast_seq: 0,
            toasts_hovered: false,
            toasts_tick: None,
            toast_showing: None,
            note_hovered: false,
            note_height: 64.0,
            note_at: None,
            note_window_handle: None,
            app_icon: None,
            skip_next_armed: false,
            glide_until: None,
            undo_history: UndoHistory::default(),
            next_age_prune: None,
            search_retry: None,
            ocr_failed: HashSet::new(),
            query: String::new(),
            query_changed: None,
            query_seq: 0,
            query_vec: None,
            filter: Filter::All,
            results: Vec::new(),
            search_counts: None,
            weaker: Vec::new(),
            auto_selected: None,
            weaker_link: WeakerLink::default(),
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
            viewer: None,
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
            local_servers: LocalServers::default(),
            server_lists: ServerLists::default(),
            settings_page: SettingsPage::Search,
            confirm: None,
            server_editor: None,
            server_drafts: HashMap::new(),
            pending_send: None,
            list_view: (0.0, 0.0),
            popup_list_view: (0.0, 0.0),
            popup_at: None,
            popup_hidden_last_frame: false,
            popup_moving: false,
            details_open: false,
            grid_cols: 1,
            grid_places: Vec::new(),
            card_heights: view::CardHeights::default(),
            touchpad: view::Touchpad::default(),
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
            let file = aside.display().to_string();
            // The log gets the path as it is; the window sets it apart, so
            // it reads left to right inside an Arabic sentence too.
            log_error!("error-history-moved-aside", file = file.as_str());
            let file = bidi::left_to_right(&file);
            app.set_error(t!("error-history-moved-aside", file = file));
        }
        #[cfg(windows)]
        photos::delete_old_copies();
        let win = app.win.clone();
        app.instance.on_show_request(move || win.show());
        Ok(app)
    }

    // ----- state changes -------------------------------------------------

    fn set_backend(&mut self, b: EmbedBackend) {
        self.library.clear_embeddings();
        self.query_vec = None;
        // A new choice starts fresh; the worker says if this one fails too.
        self.search_retry = None;
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
                flags.skip_asked.store(true, Ordering::SeqCst);
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

    /// Shows the full window when asked, and asks again while it has drawn
    /// nothing: a window shown but never drawn stays black.
    fn show_logic(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if self.win.show_wanted.swap(false, Ordering::SeqCst) {
            self.win.show_now();
            self.show_check = Some((now, now));
        }
        let Some((first, last)) = self.show_check else {
            return;
        };
        if self.win.hidden.load(Ordering::SeqCst) || now - first > SHOW_CHECK_FOR {
            self.show_check = None;
            return;
        }
        if now - last >= SHOW_AGAIN_AFTER {
            log::info!("the shown window hasn't drawn yet; showing it again");
            self.win.show_now();
            self.show_check = Some((first, now));
        }
        ctx.request_repaint_after(SHOW_AGAIN_AFTER);
    }

    /// Hides to the tray on close or minimise, and handles tray commands.
    /// Runs every frame, including while the window is hidden.
    fn window_logic(&mut self, ctx: &egui::Context) {
        self.update_logic(ctx);
        self.sync_update_message();
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
        self.show_logic(ctx);
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
        // Alt+C: a short note, and a badge on the tray icon until the copy
        // it skips (D59).
        if self.flags.skip_asked.swap(false, Ordering::SeqCst) {
            self.skip_next_armed = true;
            let on_screen = self.note_wanted();
            let toast = self.toast_with(t!("toast-skip-next"), Duration::from_secs(2));
            toast.icon = Some(icons::Icon::EyeOff);
            toast.on_screen = on_screen;
        }
        if self.skip_next_armed && !self.flags.ignore_next.load(Ordering::SeqCst) {
            self.skip_next_armed = false;
        }
        let paused = !self.flags.enabled.load(Ordering::SeqCst);
        let look = if self.skip_next_armed {
            TrayLook::SkipNext
        } else if paused {
            TrayLook::Paused
        } else {
            TrayLook::Recording
        };
        if let Some(t) = &mut self.tray {
            t.set_paused(paused);
            t.set_look(look);
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
            self.close_viewer();
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
                match &self.update {
                    UpdateState::Downloading(..) => return,
                    // A release newer than the one waiting takes its place.
                    // The one waiting stays on the disk until the new one is
                    // verified, so it still installs at the next start if
                    // this download fails.
                    UpdateState::Ready(waiting, _)
                        if !r.is_newer_than(waiting)
                            || (!manual && r.key() == self.cfg.skipped_update) =>
                    {
                        return;
                    }
                    _ => {}
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

    /// Carries out a button of the update's message or of the Version row in
    /// Settings, About.
    fn update_action(&mut self, action: UpdateAction, ctx: &egui::Context) {
        match action {
            UpdateAction::Close => self.update_hidden = true,
            UpdateAction::Skip => {
                if let UpdateState::Available(r) = &self.update {
                    let key = r.key();
                    // Open Settings applies its draft every frame, which
                    // would undo this.
                    if let Some(draft) = &mut self.draft {
                        draft.skipped_update = key.clone();
                    }
                    self.cfg.skipped_update = key;
                    self.save_config();
                }
                self.update = UpdateState::Idle;
            }
            UpdateAction::Download | UpdateAction::TryAgain => match &self.update {
                UpdateState::Available(r)
                | UpdateState::Failed {
                    release: Some(r), ..
                } => {
                    let r = r.clone();
                    self.download_update(r, ctx);
                }
                // A check that failed is made again.
                UpdateState::Failed { release: None, .. } => {
                    self.update_hidden = false;
                    self.check_for_update(true, ctx);
                }
                _ => {}
            },
            UpdateAction::Cancel => {
                if let UpdateState::Downloading(_, p) = &self.update {
                    p.cancel.store(true, Ordering::Relaxed);
                }
            }
            UpdateAction::Restart => self.restart_to_update(ctx),
            UpdateAction::WhatsNew => {
                let notes = match &self.update {
                    UpdateState::Available(r)
                    | UpdateState::Downloading(r, _)
                    | UpdateState::Ready(r, _)
                    | UpdateState::Failed {
                        release: Some(r), ..
                    } => Some(r.notes_url.clone()),
                    _ => None,
                };
                if let Some(url) = notes {
                    ctx.open_url(egui::OpenUrl::new_tab(url));
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
        self.toast_with(s, how_long);
    }

    /// Adds a message in front of the others, to be given a button or a
    /// second line. With three showing, the oldest goes, never the update's;
    /// a delete it could undo stays in the undo history for Ctrl+Z.
    fn toast_with(&mut self, s: impl Into<String>, how_long: Duration) -> &mut Toast {
        if self.toasts.len() >= MAX_TOASTS {
            if let Some(at) = self.toasts.iter().position(|t| t.update.is_none()) {
                self.toasts.remove(at);
            }
        }
        self.toast_seq += 1;
        self.toasts.push(Toast {
            text: s.into(),
            detail: None,
            icon: None,
            how_long,
            shown_for: Duration::ZERO,
            action: None,
            keys: None,
            secret: None,
            on_screen: false,
            drawn: false,
            id: self.toast_seq,
            update: None,
            question: None,
        });
        self.toasts.last_mut().expect("a message was just added")
    }

    /// Runs the messages' time: only while a window shows them and the
    /// pointer is not on them. A message whose time is up goes. The time in
    /// which a delete can be undone runs the same way, also after its
    /// message is gone; when it is over, the delete becomes final.
    fn tick_toasts(&mut self, ctx: &egui::Context, shown: bool) {
        let now = Instant::now();
        let last = self.toasts_tick.replace(now);
        if self.toasts.is_empty() && self.undo_history.is_empty() {
            return;
        }
        let step = last.map_or(Duration::ZERO, |last| now - last);
        // A long gap means no window drew meanwhile: that time doesn't count.
        if step < Duration::from_secs(1) {
            for toast in &mut self.toasts {
                let runs = if toast.on_screen {
                    !self.note_hovered
                } else {
                    shown && !self.toasts_hovered
                };
                if runs {
                    toast.shown_for += step;
                }
            }
            if shown && !self.toasts_hovered {
                self.undo_history.tick(step);
            }
        }
        self.toasts.retain(|t| t.shown_for < t.how_long);
        self.finish_expired_deletes();
        if shown || self.toasts.iter().any(|t| t.on_screen) {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
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
            Command::UndoDelete(number) => self.undo_delete(number),
            Command::UndoNewestDelete => {
                if let Some(number) = self.undo_history.newest() {
                    self.undo_delete(number);
                }
            }
            Command::RecordAppAgain(app) => {
                let mut d = self.cfg.clone();
                d.rules.retain(|r| !r.skips_app(&app));
                let ctx = self.win.ctx.clone();
                self.apply_settings(d, &ctx, false);
            }
            Command::IgnoreApp(app, name) => self.ignore_app(&app, &name),
            Command::SaveSkipped(capture) => {
                // Said first, so a warning the copy brings (the history 90%
                // full) comes in front of it.
                self.toast(t!("toast-saved"));
                self.record(capture);
            }
            Command::StopSendingImages(key) => self.stop_sending_images(&key),
            Command::ShowWeaker => self.show_weaker(),
            Command::HideWeaker => self.hide_weaker(),
            Command::ShowWhatsNew => {
                self.win.show_full();
                self.whats_new_open = true;
            }
            Command::StartRecordingAgain => {
                log::info!("starting the clipboard watcher again");
                self.captures = watch_clipboard(&self.win.ctx, &self.flags);
                self.recording_stopped = false;
            }
            Command::TrySearchAgain => {
                // Asked now; if it still fails, the waits start over.
                self.worker.send(Job::Probe);
                self.search_retry = Some(Retry::new(Instant::now()));
            }
            Command::OpenSettings(page) => {
                self.win.show_full();
                self.open_settings(page);
            }
            Command::ReadAgain(id) => {
                self.ocr_failed.remove(&id);
                self.worker.send(Job::OcrAgain(id));
                self.dirty = true;
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
            let from = self.program_name(app.as_deref(), program.as_deref());
            let found = text.as_deref().and_then(crate::privacy::find_secret);
            let (title, from) = match (reason, &found) {
                // Its program asked for it not to be kept: only its name.
                (SkipReason::Private, _) => (
                    t!("toast-skipped-private"),
                    from.map(|p| t!("toast-skipped-by", program = bidi::isolate(&p))),
                ),
                (SkipReason::Secret, Some(secret)) => (
                    secret_title(secret.kind),
                    from.map(|p| t!("toast-skipped-from", program = bidi::isolate(&p))),
                ),
                (SkipReason::Secret, None) => (
                    secret_title(crate::privacy::SecretKind::Key),
                    from.map(|p| t!("toast-skipped-from", program = bidi::isolate(&p))),
                ),
            };
            let hint = found.map(|s| hint_text(&s.hint));
            let detail = [from, hint].into_iter().flatten().collect::<Vec<_>>();
            let kept = text.map(|t| Capture {
                content: Captured::Text(t),
                app,
                program,
                formats,
            });
            self.skipped(title, detail, kept);
            return;
        }
        let text = match &content {
            Captured::Text(t) => Some(t.as_str()),
            _ => None,
        };
        if self.rules.skips(app.as_deref(), text) {
            log::info!("copy not saved: a rule matched");
            let from = self.program_name(app.as_deref(), program.as_deref());
            let detail = from
                .map(|p| t!("toast-skipped-from", program = bidi::isolate(&p)))
                .into_iter()
                .collect();
            let kept = Capture {
                content,
                app,
                program,
                formats,
            };
            self.skipped(t!("toast-skipped-rule"), detail, Some(kept));
            return;
        }
        self.record(Capture {
            content,
            app,
            program,
            formats,
        });
    }

    /// Stops recording copies from `app`, shown as `name`, with a rule; the
    /// message offers Undo, and the rule stays in Settings, Privacy.
    fn ignore_app(&mut self, app: &str, name: &str) {
        let d = ignoring_app(&self.cfg, app);
        let ctx = self.win.ctx.clone();
        self.apply_settings(d, &ctx, false);
        let toast = self.toast_with(
            t!("toast-app-ignored", app = bidi::isolate(name)),
            Duration::from_secs(8),
        );
        toast.action = Some((t!("toast-undo"), Command::RecordAppAgain(app.to_string())));
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
            Some(name) => t!("item-paste-into", app = bidi::isolate(&name)),
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
    /// Says a copy wasn't recorded (D57): what it was, where it came from
    /// and a hint (`detail`). `kept` is offered back with "Save anyway"; a
    /// text also shows whole while Show is held. Both leave memory with the
    /// message.
    fn skipped(&mut self, title: String, detail: Vec<String>, kept: Option<Capture>) {
        // Longer when there is a button to reach.
        let how_long = Duration::from_secs(if kept.is_some() { 8 } else { 4 });
        let secret = kept.as_ref().and_then(|c| match &c.content {
            Captured::Text(t) => Some(t.clone()),
            _ => None,
        });
        let toast = self.toast_with(title, how_long);
        toast.icon = Some(icons::Icon::Lock);
        // Each part in its own direction, so in Arabic a program's name
        // and a hint after it don't run together.
        let parts: Vec<String> = detail.iter().map(|part| bidi::isolate(part)).collect();
        toast.detail = (!parts.is_empty()).then(|| parts.join(" · "));
        toast.secret = secret;
        toast.action = kept.map(|c| (t!("toast-save-anyway"), Command::SaveSkipped(c)));
        let on_screen = self.note_wanted();
        if let Some(toast) = self.toasts.last_mut() {
            toast.on_screen = on_screen;
        }
    }

    /// The program a copy came from: its own name once it is known, else
    /// its file name.
    fn program_name(&mut self, app: Option<&str>, program: Option<&Path>) -> Option<String> {
        program
            .and_then(|p| self.programs.get(p))
            .map(|p| p.name.clone())
            .or_else(|| app.map(programs::tidy))
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
        self.prune(false);
    }

    /// Deletes what the history limit and delete by age remove, then warns
    /// when the history is 90% full (D64).
    fn prune(&mut self, report: bool) {
        self.prune_now(report);
        self.check_history_fill();
    }

    /// Deletes what is over the history size and, when it is on, what is
    /// older than the chosen number of days. With `report`, it says so,
    /// with Undo, and the items wait for the undo history's time (D60). A new
    /// copy pushing the oldest item out of a full history is not reported.
    fn prune_now(&mut self, report: bool) {
        let days = i64::from(self.cfg.delete_after_days);
        let cutoff = crate::store::now() - days * 24 * 60 * 60;
        if !report {
            let pruned = self.library.prune(self.cfg.max_items).and_then(|mut ids| {
                if days > 0 {
                    ids.extend(self.library.prune_older_than(cutoff)?);
                }
                Ok(ids)
            });
            match pruned {
                Ok(removed) => self.drop_removed(&removed),
                Err(e) => self.set_error(log_error!("error-deleting", error = e.to_string())),
            }
            return;
        }
        let mut ids = self.library.over_limit(self.cfg.max_items);
        let over_limit = ids.len();
        if days > 0 {
            let more: Vec<u64> = self
                .library
                .older_than(cutoff)
                .into_iter()
                .filter(|id| !ids.contains(id))
                .collect();
            ids.extend(more);
        }
        if ids.is_empty() {
            return;
        }
        let removed = self.take_items(&ids);
        self.drop_stale_tag_filter();
        self.drop_removed(&ids);
        let count = removed.len();
        log::info!("deleted {count} items by age or the history limit");
        let text = if count == over_limit {
            t!("toast-pruned-limit", count = count)
        } else {
            let age = t!("settings-age-days", count = days);
            t!("toast-pruned-age", count = count, age = age)
        };
        self.keep_for_undo(text, removed);
        self.dirty = true;
    }

    /// Warns once when the items the history limit counts reach 90% of it
    /// (D64), and again only after they drop below that: a higher limit,
    /// or items deleted.
    fn check_history_fill(&mut self) {
        let (used, limit) = (self.library.clearable_count(), self.cfg.max_items);
        let full = limit > 0 && used * 10 >= limit * 9;
        if full == self.cfg.history_warned {
            return;
        }
        self.cfg.history_warned = full;
        self.save_config();
        if !full {
            return;
        }
        log::info!("the history is 90% full: {used} of {limit} items");
        let text = t!(
            "toast-history-almost-full",
            used = thousands(used),
            limit = t!("settings-history-items", size = thousands(limit))
        );
        let toast = self.toast_with(text, Duration::from_secs(10));
        toast.icon = Some(icons::Icon::Warning);
        toast.action = Some((
            t!("toast-history-change-limit"),
            Command::OpenSettings(settings::SettingsPage::Clipboard),
        ));
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
                self.prune(false);
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
            #[cfg(windows)]
            Done::CopiedForPhotos(copy) => {
                // Started from the window's thread, which has COM set up, as
                // ShellExecuteExW needs.
                let shown = copy.and_then(|path| photos::show(&path).map_err(|e| e.to_string()));
                if let Err(e) = shown {
                    self.set_error(log_error!("error-opening-in-photos", error = e));
                }
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
        self.local_servers.poll();
        self.server_lists.poll();
        loop {
            match self.captures.try_recv() {
                Ok(c) => self.on_capture(c),
                Err(TryRecvError::Empty) => break,
                // The watcher gave up after crashing again and again.
                Err(TryRecvError::Disconnected) => {
                    self.recording_stopped = true;
                    break;
                }
            }
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
                        self.toast_with(t!("toast-images-refused"), IMAGES_REFUSED_FOR)
                            .action =
                            Some((t!("toast-images-off"), Command::StopSendingImages(key)));
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
                    // Loaded on a retry: this already sends what is missing.
                    self.search_retry = None;
                    self.index_missing();
                    self.query_changed = Some(Instant::now() - QUERY_DEBOUNCE);
                    self.dirty = true;
                }
                Event::OcrFailed { id } => {
                    self.ocr_failed.insert(id);
                    self.dirty = true;
                }
                Event::SearchServer { answering: false } => {
                    if self.search_retry.is_none() {
                        self.search_retry = Some(Retry::new(Instant::now()));
                    }
                }
                Event::SearchServer { answering: true } => {
                    // Items that failed or waited meanwhile go again.
                    if self.search_retry.take().is_some() {
                        self.index_missing();
                    }
                }
                Event::Ocr { id, text } => {
                    self.ocr_failed.remove(&id);
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
        let max_side = ctx.input(|i| i.max_texture_side);
        for l in loaded {
            if l.skipped {
                self.loader.forget(l.id, l.size);
                continue;
            }
            // Mipmaps keep thumbnails smooth when shrunk into a list row, and
            // a picture full screen when shrunk to fit the screen.
            let opts = if l.size == Size::Preview {
                egui::TextureOptions::LINEAR
            } else {
                egui::TextureOptions {
                    mipmap_mode: Some(egui::TextureFilter::Linear),
                    ..egui::TextureOptions::LINEAR
                }
            };
            let name = match l.size {
                Size::Thumb => "thumb",
                Size::Preview => "preview",
                Size::Full => "full",
            };
            // A picture bigger than the graphics card takes can't be a
            // texture: full screen, its preview stands in.
            let image = l
                .image
                .filter(|image| image.width().max(image.height()) <= max_side);
            let tex = image.map(|ci| ctx.load_texture(format!("{name}{}", l.id), ci, opts));
            match l.size {
                Size::Thumb => {
                    self.thumbs.insert(l.id, tex);
                }
                Size::Preview => {
                    // A preview for an item the user already moved past is dropped.
                    if self.selected == Some(l.id) {
                        self.preview = Some((l.id, tex));
                    }
                    self.loader.forget(l.id, Size::Preview);
                }
                Size::Full => {
                    if let Some(viewer) = &mut self.viewer {
                        viewer.picture_came(l.id, tex);
                    }
                    self.loader.forget(l.id, Size::Full);
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

    fn show_results(&mut self, (hits, counts): search::Found) {
        self.searching = false;
        self.search_counts = counts;
        // Weaker matches wait under their link, unless it was opened for
        // this search text.
        (self.results, self.weaker) = self.weaker_link.fold(self.query.trim(), hits);
        let still_there = self
            .selected
            .is_some_and(|id| self.results.iter().any(|h| h.id == id));
        // Until another item is picked, the selection follows the best match,
        // also when a ranking by meaning comes after the one by words.
        let searching = !self.query.trim().is_empty();
        let untouched = searching && self.selected.is_some() && self.selected == self.auto_selected;
        if !still_there || untouched {
            let best = self.results.first().map(|h| h.id);
            if best != self.selected {
                self.scroll_to_selected = true;
            }
            self.selected = best;
            self.auto_selected = best;
        }
    }

    /// Whether the weaker matches of this search wait folded under their
    /// link.
    fn weaker_folded(&self) -> bool {
        !self.weaker_link.is_open(self.query.trim())
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
    /// delete can no longer be undone (see `finish_expired_deletes`).
    fn delete(&mut self, id: u64) {
        let Some(removed) = self.library.take(id) else {
            return;
        };
        self.glide();
        self.loader.forget(id, Size::Thumb);
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
        let keys = self.cfg.keys.delete.clone();
        let text = match self.key_to_teach(&keys) {
            Some(keys) => t!("toast-deleted-key", keys = keys),
            None => t!("toast-deleted"),
        };
        self.keep_for_undo(text, vec![removed]);
    }

    /// Keeps the items of a delete in the undo history, and says `text` in
    /// a message with Undo.
    fn keep_for_undo(&mut self, text: String, removed: Vec<Removed>) {
        let number = self.undo_history.add(removed);
        let toast = self.toast_with(text, DELETED_MESSAGE_FOR);
        toast.icon = Some(icons::Icon::Delete);
        toast.action = Some((t!("toast-undo"), Command::UndoDelete(number)));
        self.name_undo_keys();
    }

    /// Names Ctrl+Z only on the Undo of the delete it would undo: the
    /// newest that can still be undone.
    fn name_undo_keys(&mut self) {
        let newest = self.undo_history.newest();
        for toast in &mut self.toasts {
            if let Some(number) = toast.undoes() {
                toast.keys = (Some(number) == newest).then(|| UNDO_KEYS.to_string());
            }
        }
    }

    /// Makes the deletes whose time to undo is over final, and takes their
    /// messages away.
    fn finish_expired_deletes(&mut self) {
        let (numbers, removed) = self.undo_history.take_expired();
        if numbers.is_empty() {
            return;
        }
        self.toasts
            .retain(|t| t.undoes().is_none_or(|number| !numbers.contains(&number)));
        self.forget(removed);
    }

    /// Makes every delete final, as the app quits.
    pub(super) fn finish_deletes(&mut self) {
        let removed = self.undo_history.take_all();
        self.forget(removed);
    }

    /// Deletes taken items from the disk: database rows, vectors and image
    /// files.
    fn forget(&mut self, removed: Vec<Removed>) {
        if let Err(e) = self.library.forget_many(removed) {
            self.set_error(log_error!("error-deleting", error = e.to_string()));
        }
    }

    /// Brings back the items of the delete `number`, while it can still be
    /// undone.
    fn undo_delete(&mut self, number: u64) {
        self.toasts.retain(|t| t.undoes() != Some(number));
        let Some(ids) = self.undo_history.undo(number, &mut self.library) else {
            return;
        };
        self.glide();
        // The first in the list was taken last, so it went back first.
        self.selected = ids.first().copied();
        self.name_undo_keys();
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
    /// Takes `ids` out of the history in memory, the last first, so each
    /// comes back in its place on Undo.
    /// Rows and cards glide to their new places for a moment.
    fn glide(&mut self) {
        self.glide_until = Some(Instant::now() + GLIDE_FOR);
    }

    fn take_items(&mut self, ids: &[u64]) -> Vec<Removed> {
        self.glide();
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
        ids.sort_by_key(|id| std::cmp::Reverse(place[id]));
        let mut removed = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(r) = self.library.take(id) {
                self.loader.forget(id, Size::Thumb);
                removed.push(r);
            }
        }
        removed
    }

    fn delete_group(&mut self, ids: &[u64]) {
        let row = self.results.iter().position(|h| ids.contains(&h.id));
        let removed = self.take_items(ids);
        self.drop_stale_tag_filter();
        self.drop_removed(ids);
        self.clear_marks();
        // Keep the selection on the row that takes the first one's place.
        if let Some(i) = row {
            let last = self.results.len().saturating_sub(1);
            self.selected = self.results.get(i.min(last)).map(|h| h.id);
        }
        let count = removed.len();
        self.keep_for_undo(t!("toast-deleted-group", count = count), removed);
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

    /// Opens the weaker matches below their link, while the search text
    /// stays the same.
    fn show_weaker(&mut self) {
        self.weaker_link.choose(self.query.trim(), true);
        self.results.append(&mut self.weaker);
    }

    /// Folds the weaker matches back under their link, while the search
    /// text stays the same. A weaker match that was selected gives the
    /// selection to the last strong one, beside the link.
    fn hide_weaker(&mut self) {
        self.weaker_link.choose(self.query.trim(), false);
        let (strong, weak): (Vec<Hit>, Vec<Hit>) = std::mem::take(&mut self.results)
            .into_iter()
            .partition(|h| !h.weak);
        (self.results, self.weaker) = (strong, weak);
        let hidden = self
            .selected
            .is_some_and(|id| self.weaker.iter().any(|h| h.id == id));
        if hidden {
            self.selected = self.results.last().map(|h| h.id);
            self.scroll_to_selected = true;
        }
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

    /// With cards stacked in columns, selects the card below the selected
    /// one in its column, or above it.
    fn step_in_column(&mut self, down: bool) {
        let Some(at) = self.selected_place() else {
            return self.move_selection(0);
        };
        let next = self
            .grid_places
            .iter()
            .filter(|(_, p)| (p.left() - at.left()).abs() < 1.0)
            .filter(|(_, p)| {
                if down {
                    p.top() > at.top()
                } else {
                    p.top() < at.top()
                }
            })
            .min_by(|(_, a), (_, b)| {
                (a.top() - at.top())
                    .abs()
                    .total_cmp(&(b.top() - at.top()).abs())
            })
            .map(|(id, _)| *id);
        // Going on from the end shows the weaker matches below.
        if next.is_none() && down && !self.weaker.is_empty() {
            self.show_weaker();
        }
        if let Some(id) = next {
            self.selected = Some(id);
            self.scroll_to_selected = true;
        }
    }

    /// With cards stacked in columns, selects the card in the column
    /// `columns` away whose middle is nearest the selected one's.
    fn step_across(&mut self, columns: i32) {
        let Some(at) = self.selected_place() else {
            return self.move_selection(0);
        };
        let target = at.left() + columns as f32 * (at.width() + view::GRID_GAP);
        let next = self
            .grid_places
            .iter()
            .filter(|(_, p)| (p.left() - target).abs() < 1.0)
            .min_by(|(_, a), (_, b)| {
                let (a, b) = (a.center().y - at.center().y, b.center().y - at.center().y);
                a.abs().total_cmp(&b.abs())
            })
            .map(|(id, _)| *id);
        if let Some(id) = next {
            self.selected = Some(id);
            self.scroll_to_selected = true;
        }
    }

    /// Where the selected card was drawn, with cards stacked in columns.
    fn selected_place(&self) -> Option<egui::Rect> {
        let id = self.selected?;
        self.grid_places
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, p)| *p)
    }

    fn preview(&mut self, it: &Item) -> Preview {
        match &self.preview {
            Some((id, Some(t))) if *id == it.id => return Preview::Ready(t.clone()),
            Some((id, None)) if *id == it.id => return Preview::Missing,
            _ => {}
        }
        match &it.image {
            Some(p) => {
                self.loader.request(it.id, p, Size::Preview);
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

/// `n` with commas between its thousands: "5,000".
fn thousands(n: usize) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|c| std::str::from_utf8(c).expect("digits"))
        .collect::<Vec<_>>()
        .join(",")
}

/// "Not saved: an API key" and the like.
fn secret_title(kind: crate::privacy::SecretKind) -> String {
    use crate::privacy::SecretKind;
    match kind {
        SecretKind::Password => t!("toast-skipped-password"),
        SecretKind::ApiKey => t!("toast-skipped-api-key"),
        SecretKind::SignIn => t!("toast-skipped-sign-in"),
        SecretKind::PrivateKey => t!("toast-skipped-private-key"),
        SecretKind::Link => t!("toast-skipped-link"),
        SecretKind::Key => t!("toast-skipped-key"),
    }
}

/// A hint that tells one secret from another without showing it.
fn hint_text(hint: &crate::privacy::Hint) -> String {
    use crate::privacy::Hint;
    match hint {
        Hint::Ends(start, end) => format!("{start}…{end}"),
        Hint::Host(host) => host.clone(),
        Hint::Length(n) => t!("toast-hint-characters", count = *n),
    }
}

/// Starts the clipboard watcher; its copies arrive on the receiver, which
/// closes if the watcher stops.
fn watch_clipboard(ctx: &egui::Context, flags: &Arc<Flags>) -> Receiver<Capture> {
    let (tx, captures) = channel();
    let ctx = ctx.clone();
    clipboard::spawn_watcher(flags.clone(), tx, move || ctx.request_repaint());
    captures
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
pub(crate) mod bidi;
mod icons;
#[cfg(windows)]
mod photos;
mod programs;
mod settings;
mod shortcut;
mod undo;
mod view;

use background::{Background, Done};
use settings::{PendingSend, ServerEditor, SettingsPage};
use undo::UndoHistory;
use view::{add_fonts, apply_theme, looks_like_code, set_accent, style};

/// Whether the link between the strong and the weaker matches is open, so
/// the weaker ones show below it. It starts closed for every search text,
/// however few the strong matches are. A click opens or closes it, and
/// that choice holds while the search text stays the same.
#[derive(Default)]
struct WeakerLink {
    /// The search text it is open for; `None` while it is closed.
    open_for: Option<String>,
}

impl WeakerLink {
    /// New results `hits` for `query`: the ones to show, and the weaker
    /// ones that wait under the closed link. Another search text closes
    /// it, also when the one it was opened for comes back later.
    fn fold(&mut self, query: &str, hits: Vec<Hit>) -> (Vec<Hit>, Vec<Hit>) {
        if self.is_open(query) {
            return (hits, Vec::new());
        }
        self.open_for = None;
        hits.into_iter().partition(|h| !h.weak)
    }

    fn is_open(&self, query: &str) -> bool {
        self.open_for.as_deref() == Some(query)
    }

    /// The user opens the link for `query`, or closes it.
    fn choose(&mut self, query: &str, open: bool) {
        self.open_for = open.then(|| query.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::Storage as _;

    /// Results with `strong` strong matches, then `weak` weaker ones.
    fn hits(strong: usize, weak: usize) -> Vec<Hit> {
        (0..strong + weak)
            .map(|i| Hit {
                id: i as u64,
                closeness: None,
                weak: i >= strong,
            })
            .collect()
    }

    /// How many results show and how many wait under the link.
    fn shown_and_folded((shown, folded): (Vec<Hit>, Vec<Hit>)) -> (usize, usize) {
        (shown.len(), folded.len())
    }

    /// Two strong matches and 14 weaker ones, fewer strong ones than a row
    /// of 4 cards holds, start closed: the two show, the 14 wait under the
    /// link.
    #[test]
    fn two_strong_and_14_weaker_matches_start_closed() {
        let mut link = WeakerLink::default();
        let (shown, folded) = link.fold("total", hits(2, 14));
        assert!(shown.iter().all(|h| !h.weak) && folded.iter().all(|h| h.weak));
        assert_eq!((shown.len(), folded.len()), (2, 14));
        assert!(!link.is_open("total"));
    }

    #[test]
    fn the_weaker_matches_link_keeps_the_choice_for_one_search_text() {
        let mut link = WeakerLink::default();
        // A click opens it; more results for the same text show their
        // weaker matches too.
        link.fold("cat", hits(2, 14));
        link.choose("cat", true);
        assert_eq!(shown_and_folded(link.fold("cat", hits(1, 14))), (15, 0));
        // A new search text starts closed, also when the old one comes
        // back.
        assert_eq!(shown_and_folded(link.fold("cats", hits(2, 14))), (2, 14));
        assert_eq!(shown_and_folded(link.fold("cat", hits(2, 14))), (2, 14));
        // A click closes an open link; more results for the same text
        // leave it closed.
        link.choose("cat", true);
        link.choose("cat", false);
        assert_eq!(shown_and_folded(link.fold("cat", hits(2, 14))), (2, 14));
        // The choice is for its own search text only.
        link.choose("dog", true);
        assert!(link.is_open("dog"));
        assert!(!link.is_open("dogs"));
    }

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

    /// Minimized, or full screen for a picture, the window keeps the place
    /// and size it had before.
    #[test]
    fn a_minimized_window_keeps_its_place_and_size() {
        let mut storage = Entries(HashMap::new());
        let mut kept = None;
        // What eframe saves before the app's `save`, then what the app leaves.
        let mut save = |entry: &str, is_temporary: bool| {
            storage.set_string(WINDOW_KEY, entry.to_string());
            keep_window_state(&mut storage, is_temporary, &mut kept);
            storage.get_string(WINDOW_KEY).unwrap()
        };
        assert_eq!(save("1040 by 680 at 32,32", false), "1040 by 680 at 32,32");
        assert_eq!(save("0 by 0 at -32000", true), "1040 by 680 at 32,32");
        assert_eq!(save("1200 by 900 at 40,40", false), "1200 by 900 at 40,40");
        assert_eq!(save("full screen", true), "1200 by 900 at 40,40");
    }
}
