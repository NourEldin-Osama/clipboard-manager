//! The egui user interface.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, FontId, Key, RichText, Sense, Stroke, TextureHandle, Vec2};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::clipboard::{self, Captured, Flags};
use crate::config::{Config, EmbedBackend, LaunchConfig, Paths, SearchMode, ServerConfig, Theme};
use crate::embed::Embedding;
use crate::images::ImageLoader;
use crate::import;
use crate::models::{self, Progress, SIGLIP_MODELS};
use crate::search::{self, Filter, Hit};
use crate::store::{looks_like_link, now, Item, Kind, Origin, Store};
use crate::worker::{Event, Job, Worker};

const QUERY_DEBOUNCE: Duration = Duration::from_millis(180);
/// While indexing, re-rank at most this often instead of after every vector.
const RESEARCH_EVERY: Duration = Duration::from_millis(400);
/// Thumbnail textures kept before the cache is dropped and refilled from what is visible.
const MAX_THUMBS: usize = 600;

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
            let _ = tx.send(crate::embed::server::list_models(&u, &k).map_err(|e| format!("{e:#}")));
            c.request_repaint();
        });
        self.url = Self::key(s);
        self.pending = Some(rx);
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
    toast: Option<(String, Instant, Duration)>,

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
    hotkey_error: Option<String>,
    model_list: ModelList,
    settings_page: SettingsPage,
    /// A destructive action waiting for a second click.
    confirm: Option<&'static str>,
    /// Scroll offset and visible height of the list, from the last frame.
    list_view: (f32, f32),
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Paths) -> anyhow::Result<Self> {
        let ctx = cc.egui_ctx.clone();
        let cfg = Config::load(&paths.config);
        set_accent(cfg.accent);
        style(&ctx);
        apply_theme(&ctx, cfg.theme);

        let store = Arc::new(Store::open(&paths.db, &paths.images)?);
        let items = store.all_items()?;
        let next_id = store.next_id()?;

        let (ev_tx, events) = channel();
        let c = ctx.clone();
        let worker = Worker::spawn(store.clone(), paths.models.clone(), ev_tx, move || c.request_repaint());

        let flags = Arc::new(Flags::default());
        flags.enabled.store(cfg.monitor_clipboard, Ordering::SeqCst);
        flags.skip_secrets.store(cfg.skip_secrets, Ordering::SeqCst);
        let (cap_tx, captures) = channel();
        let c = ctx.clone();
        clipboard::spawn_watcher(flags.clone(), cap_tx, move || c.request_repaint());

        let mut app = Self {
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
            focus_search: Arc::new(AtomicBool::new(true)),
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
            hotkey_error: None,
            model_list: ModelList::default(),
            settings_page: SettingsPage::Search,
            confirm: None,
            list_view: (0.0, 0.0),
            cfg,
        };
        app.lower = app.items.iter().map(|i| (i.id, i.searchable_text().to_lowercase())).collect();
        app.set_backend(app.cfg.backend.clone());
        if app.cfg.ocr_images {
            for it in &app.items {
                if it.kind == Kind::Image && it.ocr.is_none() {
                    app.worker.send(Job::Ocr(it.id));
                }
            }
        }
        app.register_hotkeys(&ctx);
        Ok(app)
    }

    // ----- state changes -------------------------------------------------

    fn set_backend(&mut self, b: EmbedBackend) {
        self.embeddings.clear();
        self.query_vec = None;
        self.backend = if b == EmbedBackend::None { BackendState::Off } else { BackendState::Loading };
        self.worker.send(Job::SetBackend(b));
        self.dirty = true;
    }

    fn register_hotkeys(&mut self, ctx: &egui::Context) {
        self.hotkeys = None;
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
        let (c, flags, focus, focused) = (ctx.clone(), self.flags.clone(), self.focus_search.clone(), self.focused.clone());
        GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
            if e.state() != HotKeyState::Pressed {
                return;
            }
            if Some(e.id()) == show_id {
                if focused.load(Ordering::SeqCst) {
                    c.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                } else {
                    c.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    c.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    c.send_viewport_cmd(egui::ViewportCommand::Focus);
                    focus.store(true, Ordering::SeqCst);
                }
            } else if Some(e.id()) == ignore_id {
                flags.ignore_next.store(true, Ordering::SeqCst);
            }
            c.request_repaint();
        }));
        self.hotkeys = Some(Hotkeys { _manager: manager });
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

    fn toast_for(&mut self, s: impl Into<String>, how_long: Duration) {
        self.toast = Some((s.into(), Instant::now(), how_long));
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
        self.lower.insert(item.id, item.searchable_text().to_lowercase());
        self.items.push(item);
        self.dirty = true;
    }

    fn on_capture(&mut self, c: Captured) {
        if let Captured::Skipped(reason) = c {
            self.toast_for(reason, Duration::from_secs(4));
            return;
        }
        let hash = c.hash();
        if self.cfg.dedupe {
            if let Some(it) = self.items.iter_mut().find(|i| i.hash == hash && i.origin == Origin::Clipboard) {
                it.created = now();
                let it = it.clone();
                let _ = self.store.put(&it);
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
        };
        match c {
            Captured::Text(t) => {
                item.kind = if looks_like_link(&t) { Kind::Link } else { Kind::Text };
                item.text = t.into();
            }
            Captured::Files(f) => {
                item.kind = Kind::Files;
                item.text = clipboard::join_paths(&f).into();
            }
            Captured::Skipped(_) => unreachable!(),
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
            .filter(|i| i.origin == Origin::Clipboard && !i.pinned)
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
        let known: HashSet<u64> = self.items.iter().filter(|i| i.origin == Origin::Imported).map(|i| i.hash).collect();
        let new: Vec<Item> = import::collect(&paths, &mut next).into_iter().filter(|i| !known.contains(&i.hash)).collect();
        self.next_id = next;
        let n = new.len();
        for it in new {
            self.add_item(it);
        }
        self.toast(format!("Imported {n} file{}", if n == 1 { "" } else { "s" }));
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
                Event::BackendReady { key, images, stored } => {
                    // Learn the server's default model name for the status bar.
                    if let EmbedBackend::Server(sc) = &self.cfg.backend {
                        if sc.model.trim().is_empty() && self.model_list.pending.is_none() && self.model_list.url != ModelList::key(sc) {
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
                    let reembed = matches!(self.backend, BackendState::Ready { images: false, .. }) && !text.is_empty();
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
                Event::QueryVector { seq, key, embedding } => {
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
            let tex = l.image.map(|ci| ctx.load_texture(format!("{}{}", if l.preview { "preview" } else { "thumb" }, l.id), ci, egui::TextureOptions::LINEAR));
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
                        self.toast(format!("Downloaded {}", d.repo));
                        if matches!(&self.cfg.backend, EmbedBackend::SigLip { repo } if *repo == d.repo) {
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
                if self.ready_key().is_some() && self.cfg.search_mode != SearchMode::Keyword && !self.query.trim().is_empty() {
                    self.query_seq += 1;
                    self.worker.query(self.query_seq, self.query.trim().to_string());
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
        let vector = self.query_vec.as_ref().filter(|(q, _)| *q == self.query).map(|(_, v)| v);
        self.results = search::search(
            &self.items,
            &self.lower,
            &self.embeddings,
            &search::Query {
                text: &self.query,
                vector,
                mode: self.cfg.search_mode,
                filter: self.filter,
                min_similarity: self.cfg.min_similarity,
                image_vectors: matches!(self.backend, BackendState::Ready { images: true, .. }),
            },
        );
        let still_there = self.selected.map(|id| self.results.iter().any(|h| self.items[h.idx].id == id)).unwrap_or(false);
        if !still_there {
            self.selected = self.results.first().map(|h| self.items[h.idx].id);
        }
    }

    fn item(&self, id: u64) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    fn copy(&mut self, id: u64, ctx: &egui::Context) {
        let Some(it) = self.item(id).cloned() else { return };
        self.flags.ignore_next.store(true, Ordering::SeqCst);
        let r = match it.kind {
            Kind::Text | Kind::Link => clipboard::copy_text(&it.text),
            Kind::Files => clipboard::copy_files(&it.text.lines().map(PathBuf::from).collect::<Vec<_>>()),
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
                    if x.origin == Origin::Clipboard {
                        x.created = now();
                        let x = x.clone();
                        let _ = self.store.put(&x);
                        self.dirty = true;
                    }
                }
                self.toast("Copied");
                if self.cfg.minimize_after_copy {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
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
            let it = it.clone();
            let _ = self.store.put(&it);
            self.dirty = true;
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
            // Keep the selection on a neighbour.
            let idx = self.results.iter().position(|h| h.idx == pos);
            self.dirty = true;
            self.refresh_results();
            if let Some(i) = idx {
                self.selected = self.results.get(i.min(self.results.len().saturating_sub(1))).map(|h| self.items[h.idx].id);
            }
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.results.is_empty() {
            return;
        }
        let cur = self.selected.and_then(|id| self.results.iter().position(|h| self.items[h.idx].id == id));
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

    /// Deletes unpinned clipboard items; pinned and imported items stay.
    fn clear_history(&mut self) {
        let doomed: Vec<Item> = self.items.iter().filter(|i| i.origin == Origin::Clipboard && !i.pinned).cloned().collect();
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
        self.toast(format!("Cleared {} items", gone.len()));
    }
}

mod settings_page;
mod view;

use settings_page::SettingsPage;
use view::{apply_theme, looks_like_code, set_accent, style};
