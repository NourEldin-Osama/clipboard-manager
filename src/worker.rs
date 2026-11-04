//! Background thread that owns the OCR engine and the embedding model, so the
//! UI never blocks on inference. Searches go first, then model switches, then
//! new copies, then indexing of the existing history.

use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use anyhow::{anyhow, Result};

use crate::config::EmbedBackend;
use crate::config::TextReader;
use crate::embed::{server::ServerEmbedder, Embedder, Embedding, TextRole};
use crate::i18n::{log_server_error, t};
use crate::net::ServerError;
use crate::ocr::Reader;
use crate::store::{Kind, Store};

/// Items embedded per model call while indexing.
const EMBED_BATCH: usize = 16;

pub enum Job {
    Ocr(u64),
    Embed(u64),
    SetBackend(EmbedBackend),
    SetTextReader(TextReader),
}

pub enum Event {
    /// A status line, in the interface language.
    Status(String),
    /// An error to show, in the interface language (the log has it in English).
    Error(String),
    /// A backend is loaded; carries the embeddings already stored for it.
    BackendReady {
        key: String,
        images: bool,
        stored: Vec<(u64, Embedding)>,
    },
    BackendOff,
    /// The search model's server refused an image: images now go to it as
    /// their read text.
    ImagesRefused {
        key: String,
    },
    Ocr {
        id: u64,
        text: String,
    },
    Embedded {
        id: u64,
        key: String,
        embedding: Option<Embedding>,
    },
    QueryVector {
        seq: u64,
        key: String,
        embedding: Embedding,
    },
    /// Pending indexing jobs left.
    Queue(usize),
}

pub struct Worker {
    tx: Sender<Message>,
    pending: Arc<std::sync::atomic::AtomicUsize>,
}

enum Message {
    Query(u64, String),
    Job(Job, Priority),
}

#[derive(Clone, Copy)]
enum Priority {
    /// For an item just copied: the user may search for it right away.
    Fresh,
    /// Indexing the existing history.
    Backfill,
}

impl Worker {
    pub fn spawn(
        store: Arc<Store>,
        text_reader: TextReader,
        events: Sender<Event>,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let (tx, rx) = channel();
        let pending = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let p = pending.clone();
        std::thread::Builder::new()
            .name("indexer".into())
            .spawn(move || {
                let mut st = State {
                    store,
                    events,
                    wake: Box::new(wake),
                    text_reader,
                    ocr: None,
                    ocr_failed: false,
                    ocr_error_shown: false,
                    embedder: None,
                    backend: None,
                    have: HashSet::new(),
                    failed: HashSet::new(),
                    images_refused: false,
                    image_worked: false,
                    index_error_shown: false,
                    pending: p,
                };
                st.run(rx);
            })
            .expect("spawn worker");
        Self { tx, pending }
    }

    /// Work for an item just copied, or a model switch: goes before
    /// background indexing.
    pub fn send(&self, job: Job) {
        self.push(job, Priority::Fresh);
    }

    /// Background work on the existing history (indexing, OCR of old images):
    /// runs when nothing newer is waiting.
    pub fn send_backfill(&self, job: Job) {
        self.push(job, Priority::Backfill);
    }

    fn push(&self, job: Job, priority: Priority) {
        if !matches!(job, Job::SetBackend(_) | Job::SetTextReader(_)) {
            self.pending
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let _ = self.tx.send(Message::Job(job, priority));
    }

    pub fn query(&self, seq: u64, text: String) {
        let _ = self.tx.send(Message::Query(seq, text));
    }
}

/// What the worker has to do, in the order it does it: the newest search
/// (the user is waiting for it), a model switch (queued work belongs to the
/// new model), work for new copies, then background indexing.
#[derive(Default)]
struct Queues {
    query: Option<(u64, String)>,
    backend: Option<EmbedBackend>,
    reader: Option<TextReader>,
    fresh: VecDeque<Job>,
    backfill: VecDeque<Job>,
}

enum Next {
    Query(u64, String),
    Backend(EmbedBackend),
    Reader(TextReader),
    Ocr(u64),
    Embed(Vec<u64>),
}

impl Queues {
    fn push(&mut self, m: Message) {
        match m {
            // Only the newest search and the latest model choice matter.
            Message::Query(seq, text) => self.query = Some((seq, text)),
            Message::Job(Job::SetBackend(b), _) => self.backend = Some(b),
            Message::Job(Job::SetTextReader(r), _) => self.reader = Some(r),
            Message::Job(job, Priority::Fresh) => self.fresh.push_back(job),
            Message::Job(job, Priority::Backfill) => self.backfill.push_back(job),
        }
    }

    fn is_empty(&self) -> bool {
        self.query.is_none()
            && self.backend.is_none()
            && self.reader.is_none()
            && self.fresh.is_empty()
            && self.backfill.is_empty()
    }

    fn next(&mut self) -> Option<Next> {
        if let Some((seq, text)) = self.query.take() {
            return Some(Next::Query(seq, text));
        }
        if let Some(b) = self.backend.take() {
            return Some(Next::Backend(b));
        }
        if let Some(r) = self.reader.take() {
            return Some(Next::Reader(r));
        }
        let queue = if self.fresh.is_empty() {
            &mut self.backfill
        } else {
            &mut self.fresh
        };
        match queue.pop_front()? {
            Job::Ocr(id) => Some(Next::Ocr(id)),
            Job::Embed(id) => {
                // Many inputs per request are much faster, especially over HTTP.
                let mut ids = vec![id];
                while ids.len() < EMBED_BATCH {
                    match queue.front() {
                        Some(Job::Embed(more)) => {
                            ids.push(*more);
                            queue.pop_front();
                        }
                        _ => break,
                    }
                }
                Some(Next::Embed(ids))
            }
            Job::SetBackend(b) => Some(Next::Backend(b)),
            Job::SetTextReader(r) => Some(Next::Reader(r)),
        }
    }
}

impl Next {
    /// The item ids this step works on.
    fn ids(&self) -> Vec<u64> {
        match self {
            Next::Ocr(id) => vec![*id],
            Next::Embed(ids) => ids.clone(),
            Next::Query(..) | Next::Backend(_) | Next::Reader(_) => Vec::new(),
        }
    }
}

struct State {
    store: Arc<Store>,
    events: Sender<Event>,
    wake: Box<dyn Fn() + Send>,
    /// The reader chosen in Settings; `ocr` is that reader once made.
    text_reader: TextReader,
    ocr: Option<Reader>,
    ocr_failed: bool,
    /// A failure to read an image was shown since the reader was chosen.
    ocr_error_shown: bool,
    embedder: Option<Box<dyn Embedder>>,
    /// The model last asked for, to load again if a panic loses it.
    backend: Option<EmbedBackend>,
    /// Items that already have a vector from the current model; a repeated
    /// request for them is skipped.
    have: HashSet<u64>,
    /// Items a model or the OCR library panicked on: skipped until restart.
    failed: HashSet<u64>,
    /// The current model's server refused an image, so images go to it as
    /// their read text.
    images_refused: bool,
    /// The current model's server took an image.
    image_worked: bool,
    /// A failure to index an item was shown since the model loaded.
    index_error_shown: bool,
    pending: Arc<std::sync::atomic::AtomicUsize>,
}

impl State {
    fn emit(&self, e: Event) {
        let _ = self.events.send(e);
        (self.wake)();
    }

    fn run(&mut self, rx: Receiver<Message>) {
        let mut queues = Queues::default();
        loop {
            // Wait only when there is nothing to do, then take everything that
            // has arrived, so a newer search or a model switch goes first.
            if queues.is_empty() {
                match rx.recv() {
                    Ok(m) => queues.push(m),
                    Err(_) => return,
                }
            }
            for m in rx.try_iter() {
                queues.push(m);
            }
            let Some(next) = queues.next() else {
                continue;
            };
            let ids = next.ids();
            let was_backend = matches!(next, Next::Backend(_));
            // A panic in a model or the OCR library must not end indexing
            // for the rest of the session.
            let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run_one(next)));
            if run.is_err() {
                self.recover(&ids, was_backend);
            }
            if !ids.is_empty() {
                let done = ids.len();
                let left = self
                    .pending
                    .fetch_sub(done, std::sync::atomic::Ordering::Relaxed)
                    .saturating_sub(done);
                self.emit(Event::Queue(left));
            }
        }
    }

    fn run_one(&mut self, next: Next) {
        match next {
            Next::Query(seq, text) => self.run_query(seq, &text),
            Next::Backend(b) => self.set_backend(b),
            Next::Reader(r) => {
                self.text_reader = r;
                self.ocr = None;
                self.ocr_failed = false;
                self.ocr_error_shown = false;
            }
            Next::Ocr(id) => {
                if let Err(e) = self.do_ocr(id) {
                    // A server can fail for every image: the first failure shows.
                    if self.ocr_error_shown {
                        log::warn!("reading text in an image failed: {e:#}");
                    } else {
                        self.ocr_error_shown = true;
                        self.emit(Event::Error(log_server_error!("error-ocr-unavailable", &e)));
                    }
                }
            }
            Next::Embed(ids) => self.embed_batch(&ids),
        }
    }

    /// After a panic: the items are skipped until restart, and a model the
    /// panic took with it is loaded again (unless loading was what panicked).
    fn recover(&mut self, ids: &[u64], was_backend: bool) {
        log::error!("indexing panicked on items {ids:?}; they are skipped until the app restarts");
        self.failed.extend(ids);
        self.ocr = None;
        self.emit(Event::Error(t!("error-indexing-skipped")));
        if was_backend {
            self.embedder = None;
            self.emit(Event::BackendOff);
        } else if self.embedder.is_none() {
            if let Some(b) = self.backend.clone() {
                let reload =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.set_backend(b)));
                if reload.is_err() {
                    self.embedder = None;
                    self.emit(Event::BackendOff);
                }
            }
        }
    }

    fn run_query(&mut self, seq: u64, text: &str) {
        let Some(emb) = self.embedder.as_mut() else {
            return;
        };
        match emb.embed_texts(&[text.to_string()], TextRole::Query) {
            Ok(mut v) if !v.is_empty() => {
                let key = emb.key();
                self.emit(Event::QueryVector {
                    seq,
                    key,
                    embedding: v.remove(0),
                });
            }
            Ok(_) => {}
            Err(e) => self.emit(Event::Error(log_server_error!("error-query-embedding", &e))),
        }
    }

    fn set_backend(&mut self, b: EmbedBackend) {
        self.embedder = None;
        self.have.clear();
        self.images_refused = false;
        self.image_worked = false;
        self.index_error_shown = false;
        self.backend = Some(b.clone());
        let loaded: Result<Option<Box<dyn Embedder>>> = match &b {
            EmbedBackend::None => Ok(None),
            EmbedBackend::Server(cfg) => {
                self.emit(Event::Status(if cfg.launch.is_some() {
                    t!("status-starting-llama")
                } else {
                    t!("status-connecting", host = cfg.base_url.as_str())
                }));
                ServerEmbedder::new(cfg.as_ref().clone()).and_then(|mut s| {
                    // Probe once so misconfiguration shows up immediately.
                    s.embed_texts(&["ping".into()], TextRole::Query)?;
                    Ok(Some(Box::new(s) as Box<dyn Embedder>))
                })
            }
        };
        match loaded {
            Ok(Some(e)) => {
                let key = e.key();
                let stored = self.store.embeddings(&key).unwrap_or_default();
                self.have = stored.iter().map(|(id, _)| *id).collect();
                let images = e.supports_images();
                self.embedder = Some(e);
                self.emit(Event::BackendReady {
                    key,
                    images,
                    stored,
                });
            }
            Ok(None) => self.emit(Event::BackendOff),
            Err(e) => {
                self.emit(Event::Error(log_server_error!("error-model", &e)));
                self.emit(Event::BackendOff);
            }
        }
    }

    fn ensure_ocr(&mut self) -> Option<&Reader> {
        if self.ocr.is_none() && !self.ocr_failed {
            let r = match self.text_reader.clone() {
                TextReader::Off => return None,
                #[cfg(windows)]
                TextReader::Windows => crate::ocr::WindowsReader::new().map(Reader::Windows),
                #[cfg(not(windows))]
                TextReader::Windows => Err(anyhow!("Windows' reader works only on Windows")),
                TextReader::Server(cfg) => Ok(Reader::Server(crate::ocr::ServerReader::new(cfg))),
            };
            match r {
                Ok(o) => self.ocr = Some(o),
                Err(e) => {
                    self.ocr_failed = true;
                    self.emit(Event::Error(log_server_error!("error-ocr-unavailable", &e)));
                }
            }
        }
        self.ocr.as_ref()
    }

    fn do_ocr(&mut self, id: u64) -> Result<()> {
        if self.failed.contains(&id) {
            return Ok(());
        }
        let Some(item) = self.store.get(id)? else {
            return Ok(());
        };
        let Some(path) = item.image.clone() else {
            return Ok(());
        };
        if item.ocr.is_some() {
            return Ok(());
        }
        let img = self.store.images.open(id, &path)?;
        let text = match self.ensure_ocr() {
            Some(ocr) => ocr.recognize(&img)?,
            None => return Ok(()),
        };
        // Only the OCR field: a pin or tag set while OCR ran stays, and an item
        // deleted meanwhile stays deleted.
        let saved = self
            .store
            .update(id, |stored| stored.ocr = Some(text.as_str().into()))?;
        if saved.is_some() {
            // Its text changed, so a text vector made before is out of date.
            self.have.remove(&id);
            self.emit(Event::Ocr { id, text });
        }
        Ok(())
    }

    fn embed_batch(&mut self, ids: &[u64]) {
        // Already indexed by this model (a repeated request), or failed before.
        let ids: Vec<u64> = ids
            .iter()
            .copied()
            .filter(|id| !self.have.contains(id) && !self.failed.contains(id))
            .collect();
        if ids.is_empty() {
            return;
        }
        // Taken out so the store and event channel stay usable while it runs,
        // and put back even if it panics, so indexing can go on after it.
        let Some(mut emb) = self.embedder.take() else {
            return;
        };
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.embed_with(&mut emb, &ids)
        }));
        self.embedder = Some(emb);
        if let Err(panic) = run {
            std::panic::resume_unwind(panic);
        }
    }

    fn embed_with(&mut self, emb: &mut Box<dyn Embedder>, ids: &[u64]) {
        let key = emb.key();
        let mut texts: Vec<(u64, String)> = Vec::new();
        for &id in ids {
            let item = match self.store.get(id) {
                Ok(Some(i)) => i,
                _ => continue,
            };
            if item.kind == Kind::Image && emb.supports_images() && !self.images_refused {
                let r = item
                    .image
                    .clone()
                    .ok_or_else(|| anyhow!("image item without file"))
                    .and_then(|p| self.store.images.open(id, &p))
                    .and_then(|img| emb.embed_image(&img));
                if !self.refuses_images(&key, &r) {
                    self.image_worked |= r.is_ok();
                    self.finish(&key, id, r);
                    continue;
                }
            }
            let text: String = item.searchable_text().chars().take(8000).collect();
            if text.trim().is_empty() {
                self.emit(Event::Embedded {
                    id,
                    key: key.clone(),
                    embedding: None,
                });
            } else {
                texts.push((id, text));
            }
        }
        if !texts.is_empty() {
            let inputs: Vec<String> = texts.iter().map(|(_, t)| t.clone()).collect();
            match emb.embed_texts(&inputs, TextRole::Document) {
                Ok(vs) => {
                    let done = texts.iter().map(|(id, _)| *id).zip(vs).collect();
                    self.finish_all(&key, done);
                }
                // One bad input fails the whole request; retry individually to isolate it.
                Err(_) if texts.len() > 1 => {
                    for (id, t) in &texts {
                        let r = emb
                            .embed_texts(std::slice::from_ref(t), TextRole::Document)
                            .and_then(|mut v| {
                                v.pop().ok_or_else(|| anyhow!("no embedding returned"))
                            });
                        self.finish(&key, *id, r);
                    }
                }
                Err(e) => self.finish(&key, texts[0].0, Err(e)),
            }
        }
    }

    /// Whether `result` is the first sign that the model's server takes no
    /// images. Images then go to it as their read text until the model
    /// changes or the app restarts, and the app hears it once. After an image
    /// worked, a refusal is about that image only.
    fn refuses_images(&mut self, key: &str, result: &Result<Embedding>) -> bool {
        let Err(e) = result else {
            return false;
        };
        let refused = matches!(
            e.downcast_ref::<ServerError>(),
            Some(ServerError::NoImages { .. })
        );
        if !refused || self.image_worked {
            return false;
        }
        log::warn!(
            "{e:#}; images go to this model as their read text until the model changes or the app restarts"
        );
        self.images_refused = true;
        self.emit(Event::ImagesRefused {
            key: key.to_string(),
        });
        true
    }

    /// Saves a whole batch's vectors in one transaction, then reports each one.
    fn finish_all(&mut self, key: &str, done: Vec<(u64, Embedding)>) {
        let rows: Vec<(u64, &Embedding)> = done.iter().map(|(id, e)| (*id, e)).collect();
        if let Err(err) = self.store.put_embeddings(key, &rows) {
            log::warn!("saving embeddings failed: {err:#}");
        }
        for (id, embedding) in done {
            self.have.insert(id);
            self.emit(Event::Embedded {
                id,
                key: key.to_string(),
                embedding: Some(embedding),
            });
        }
    }

    fn finish(&mut self, key: &str, id: u64, r: Result<Embedding>) {
        let embedding = match r {
            Ok(e) => {
                if let Err(err) = self.store.put_embedding(key, id, &e) {
                    log::warn!("saving embedding failed: {err:#}");
                }
                self.have.insert(id);
                Some(e)
            }
            // A server that stopped fails every item: the first failure
            // shows, the others go only to a debug log.
            Err(e) if self.index_error_shown => {
                log::debug!("embedding item {id} failed: {e:#}");
                None
            }
            Err(e) => {
                self.index_error_shown = true;
                self.emit(Event::Error(log_server_error!("error-indexing", &e)));
                None
            }
        };
        self.emit(Event::Embedded {
            id,
            key: key.to_string(),
            embedding,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(job: Job, priority: Priority) -> Message {
        Message::Job(job, priority)
    }

    fn order(queues: &mut Queues) -> Vec<String> {
        std::iter::from_fn(|| queues.next())
            .map(|next| match next {
                Next::Query(seq, _) => format!("query {seq}"),
                Next::Backend(_) => "backend".into(),
                Next::Reader(_) => "reader".into(),
                Next::Ocr(id) => format!("ocr {id}"),
                Next::Embed(ids) => format!("embed {ids:?}"),
            })
            .collect()
    }

    #[test]
    fn the_latest_reader_choice_comes_before_the_images_waiting() {
        let mut queues = Queues::default();
        queues.push(job(Job::Ocr(1), Priority::Fresh));
        queues.push(job(
            Job::SetTextReader(TextReader::Windows),
            Priority::Fresh,
        ));
        queues.push(job(
            Job::SetTextReader(TextReader::Server(Default::default())),
            Priority::Fresh,
        ));
        assert_eq!(order(&mut queues), ["reader", "ocr 1"]);
    }

    #[test]
    fn searches_then_model_switch_then_new_copies_then_backfill() {
        let mut queues = Queues::default();
        queues.push(job(Job::Embed(1), Priority::Backfill));
        queues.push(job(Job::Embed(2), Priority::Backfill));
        queues.push(job(Job::Ocr(9), Priority::Fresh));
        queues.push(job(Job::SetBackend(EmbedBackend::None), Priority::Fresh));
        queues.push(Message::Query(1, "old".into()));
        queues.push(Message::Query(2, "newest".into()));
        queues.push(job(Job::Embed(9), Priority::Fresh));
        assert_eq!(
            order(&mut queues),
            ["query 2", "backend", "ocr 9", "embed [9]", "embed [1, 2]"]
        );
        assert!(queues.is_empty());
    }

    /// A model that panics on any text containing "boom".
    struct Fragile;

    impl Embedder for Fragile {
        fn key(&self) -> String {
            "fragile".into()
        }
        fn supports_images(&self) -> bool {
            false
        }
        fn embed_texts(&mut self, texts: &[String], _: TextRole) -> Result<Vec<Embedding>> {
            assert!(
                !texts.iter().any(|t| t.contains("boom")),
                "the model crashed"
            );
            Ok(texts.iter().map(|_| Embedding::Dense(vec![1.0])).collect())
        }
        fn embed_image(&mut self, _: &image::DynamicImage) -> Result<Embedding> {
            unreachable!("text only")
        }
    }

    /// The worker's state as `spawn` makes it, without the thread.
    fn state(store: Arc<Store>, events: Sender<Event>) -> State {
        State {
            store,
            events,
            wake: Box::new(|| {}),
            text_reader: TextReader::Windows,
            ocr: None,
            ocr_failed: false,
            ocr_error_shown: false,
            embedder: None,
            backend: None,
            have: HashSet::new(),
            failed: HashSet::new(),
            images_refused: false,
            image_worked: false,
            index_error_shown: false,
            pending: Arc::default(),
        }
    }

    /// A model whose server stopped.
    struct Stopped;

    impl Embedder for Stopped {
        fn key(&self) -> String {
            "stopped".into()
        }
        fn supports_images(&self) -> bool {
            false
        }
        fn embed_texts(&mut self, _: &[String], _: TextRole) -> Result<Vec<Embedding>> {
            Err(ServerError::Unreachable {
                url: "http://127.0.0.1:8888/v1/embeddings".into(),
                reason: "connection refused".into(),
            }
            .into())
        }
        fn embed_image(&mut self, _: &image::DynamicImage) -> Result<Embedding> {
            unreachable!("text only")
        }
    }

    #[test]
    fn a_server_that_stopped_gets_one_message() {
        let dir = std::env::temp_dir().join(format!("cbm-worker-stopped-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("t.redb"), &dir).unwrap());
        for id in 1..=3 {
            let it = crate::store::Item::new(
                id,
                Kind::Text,
                crate::store::Origin::Clipboard,
                format!("text {id}").into(),
            );
            store.put(&it).unwrap();
        }
        let (events, rx) = channel();
        let mut st = state(store.clone(), events);
        st.embedder = Some(Box::new(Stopped));
        st.run_one(Next::Embed(vec![1, 2]));
        st.run_one(Next::Embed(vec![3]));
        let errors = rx
            .try_iter()
            .filter(|event| matches!(event, Event::Error(_)))
            .count();
        assert_eq!(errors, 1);
        drop((st, store));
        std::fs::remove_dir_all(dir).ok();
    }

    /// A model on a server that takes only text, with Send images on.
    struct TextOnly;

    impl Embedder for TextOnly {
        fn key(&self) -> String {
            "text-only".into()
        }
        fn supports_images(&self) -> bool {
            true
        }
        fn embed_texts(&mut self, texts: &[String], _: TextRole) -> Result<Vec<Embedding>> {
            Ok(texts.iter().map(|_| Embedding::Dense(vec![1.0])).collect())
        }
        fn embed_image(&mut self, _: &image::DynamicImage) -> Result<Embedding> {
            Err(ServerError::NoImages {
                url: "http://127.0.0.1:8888/v1/embeddings".into(),
                said: "'input' is required for embeddings".into(),
            }
            .into())
        }
    }

    #[test]
    fn a_server_that_refuses_images_gets_their_read_text() {
        let dir = std::env::temp_dir().join(format!("cbm-worker-images-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("t.redb"), &dir).unwrap());
        for id in [1, 2] {
            let mut it = crate::store::Item::new(
                id,
                Kind::Image,
                crate::store::Origin::Clipboard,
                "".into(),
            );
            it.image = Some(store.images.save(id, &image::RgbaImage::new(4, 4)).unwrap());
            it.ocr = Some(format!("read text {id}").into());
            store.put(&it).unwrap();
        }
        let (events, rx) = channel();
        let mut st = state(store.clone(), events);
        st.embedder = Some(Box::new(TextOnly));
        st.run_one(Next::Embed(vec![1, 2]));
        assert!(st.have.contains(&1) && st.have.contains(&2));
        assert_eq!(store.embeddings("text-only").unwrap().len(), 2);
        let refused = rx
            .try_iter()
            .filter(|event| matches!(event, Event::ImagesRefused { .. }))
            .count();
        assert_eq!(refused, 1, "the app hears it once");
        drop((st, store));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_server_it_cannot_reach_turns_meaning_search_off_with_an_error() {
        let dir = std::env::temp_dir().join(format!("cbm-worker-server-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("t.redb"), &dir).unwrap());
        let (events, rx) = channel();
        let mut st = state(store.clone(), events);
        st.embedder = Some(Box::new(Fragile));
        // A port on this PC that nothing listens on: the connection is refused.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let server = crate::config::ServerConfig {
            base_url: format!("http://127.0.0.1:{port}/v1"),
            ..Default::default()
        };
        st.set_backend(EmbedBackend::Server(Box::new(server)));
        assert!(st.embedder.is_none(), "the previous model is not kept");
        let seen: Vec<&str> = rx
            .try_iter()
            .map(|event| match event {
                Event::Status(_) => "status",
                Event::Error(_) => "error",
                Event::BackendOff => "off",
                _ => "other",
            })
            .collect();
        assert_eq!(seen, ["status", "error", "off"]);
        drop((st, store));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_panic_skips_its_items_and_indexing_goes_on() {
        let dir = std::env::temp_dir().join(format!("cbm-worker-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("t.redb"), &dir).unwrap());
        for (id, text) in [(1, "fine"), (2, "boom"), (3, "also fine")] {
            let mut it = crate::store::Item::new(
                id,
                Kind::Text,
                crate::store::Origin::Clipboard,
                text.into(),
            );
            it.created = id as i64;
            store.put(&it).unwrap();
        }
        let (events, _rx) = channel();
        let mut st = state(store.clone(), events);
        st.embedder = Some(Box::new(Fragile));
        // What `run` does for each step, without the thread.
        for next in [Next::Embed(vec![1, 2]), Next::Embed(vec![3])] {
            let ids = next.ids();
            let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| st.run_one(next)));
            if run.is_err() {
                st.recover(&ids, false);
            }
        }
        assert!(
            st.failed.contains(&2),
            "the item that crashed the model is skipped"
        );
        assert!(st.embedder.is_some(), "the model survives the panic");
        assert!(st.have.contains(&3), "indexing went on");
        assert_eq!(store.embeddings("fragile").unwrap().len(), 1);
        drop((st, store));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn embeds_are_batched_up_to_the_limit() {
        let mut queues = Queues::default();
        for id in 0..20 {
            queues.push(job(Job::Embed(id), Priority::Backfill));
        }
        let first = order(&mut queues);
        assert_eq!(first.len(), 2);
        assert!(first[0].starts_with("embed [0, 1,") && first[0].ends_with("15]"));
        assert_eq!(first[1], "embed [16, 17, 18, 19]");
    }
}
