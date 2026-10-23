//! Background thread that owns the OCR engine and the embedding model, so the
//! UI never blocks on inference. Search queries jump ahead of indexing work.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::config::EmbedBackend;
use crate::embed::{server::ServerEmbedder, siglip::SigLip, Embedder, Embedding, TextRole};
use crate::i18n::{tr, trf};
use crate::models;
use crate::ocr::Ocr;
use crate::store::{Kind, Store};

/// Items embedded per model call while indexing.
const EMBED_BATCH: usize = 16;

pub enum Job {
    Ocr(u64),
    Embed(u64),
    SetBackend(EmbedBackend),
}

pub enum Event {
    Status(String),
    Error(String),
    /// A backend is loaded; carries the embeddings already stored for it.
    BackendReady {
        key: String,
        images: bool,
        stored: Vec<(u64, Embedding)>,
    },
    BackendOff,
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
    jobs: Sender<Job>,
    queries: Sender<(u64, String)>,
    pending: Arc<std::sync::atomic::AtomicUsize>,
}

impl Worker {
    pub fn spawn(
        store: Arc<Store>,
        models_dir: PathBuf,
        events: Sender<Event>,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let (jobs, job_rx) = channel();
        let (queries, query_rx) = channel();
        let pending = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let p = pending.clone();
        std::thread::Builder::new()
            .name("indexer".into())
            .spawn(move || {
                let mut st = State {
                    store,
                    models_dir,
                    events,
                    wake: Box::new(wake),
                    ocr: None,
                    ocr_failed: false,
                    embedder: None,
                    pending: p,
                };
                st.run(job_rx, query_rx);
            })
            .expect("spawn worker");
        Self {
            jobs,
            queries,
            pending,
        }
    }

    pub fn send(&self, job: Job) {
        if !matches!(job, Job::SetBackend(_)) {
            self.pending
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let _ = self.jobs.send(job);
    }

    pub fn query(&self, seq: u64, text: String) {
        let _ = self.queries.send((seq, text));
    }
}

struct State {
    store: Arc<Store>,
    models_dir: PathBuf,
    events: Sender<Event>,
    wake: Box<dyn Fn() + Send>,
    ocr: Option<Ocr>,
    ocr_failed: bool,
    embedder: Option<Box<dyn Embedder>>,
    pending: Arc<std::sync::atomic::AtomicUsize>,
}

impl State {
    fn emit(&self, e: Event) {
        let _ = self.events.send(e);
        (self.wake)();
    }

    fn run(&mut self, jobs: Receiver<Job>, queries: Receiver<(u64, String)>) {
        // A job taken off the queue while collecting a batch, to run next.
        let mut held: Option<Job> = None;
        loop {
            // Only the newest query matters.
            if let Some((seq, text)) = queries.try_iter().last() {
                self.run_query(seq, &text);
                continue;
            }
            let job = match held.take() {
                Some(j) => j,
                None => match jobs.recv_timeout(Duration::from_millis(30)) {
                    Ok(j) => j,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => return,
                },
            };
            let done = match job {
                Job::SetBackend(b) => {
                    self.set_backend(b);
                    continue;
                }
                Job::Ocr(id) => {
                    if let Err(e) = self.do_ocr(id) {
                        log::warn!("OCR failed: {e:#}");
                    }
                    1
                }
                Job::Embed(id) => {
                    // Embedding many inputs per request is much faster, especially over HTTP.
                    let mut ids = vec![id];
                    while ids.len() < EMBED_BATCH {
                        match jobs.try_recv() {
                            Ok(Job::Embed(i)) => ids.push(i),
                            Ok(other) => {
                                held = Some(other);
                                break;
                            }
                            Err(_) => break,
                        }
                    }
                    self.embed_batch(&ids);
                    ids.len()
                }
            };
            let left = self
                .pending
                .fetch_sub(done, std::sync::atomic::Ordering::Relaxed)
                .saturating_sub(done);
            self.emit(Event::Queue(left));
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
            Err(e) => self.emit(Event::Error(format!("query embedding failed: {e:#}"))),
        }
    }

    fn set_backend(&mut self, b: EmbedBackend) {
        self.embedder = None;
        let loaded: Result<Option<Box<dyn Embedder>>> = match &b {
            EmbedBackend::None => Ok(None),
            EmbedBackend::SigLip { repo } => {
                if !models::siglip_ready(&self.models_dir, repo) {
                    Err(anyhow!(
                        "{repo} is not downloaded yet (Settings → Search model)"
                    ))
                } else {
                    self.emit(Event::Status(trf("Loading {repo}…", &[("repo", &repo)])));
                    SigLip::load(
                        &models::siglip_dir(&self.models_dir, repo),
                        &format!("siglip:{repo}"),
                    )
                    .map(|m| Some(Box::new(m) as Box<dyn Embedder>))
                }
            }
            EmbedBackend::Server(cfg) => {
                self.emit(Event::Status(if cfg.launch.is_some() {
                    tr("Starting llama-server…").into()
                } else {
                    trf("Connecting to {host}…", &[("host", &cfg.base_url)])
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
                self.emit(Event::Error(format!("{e:#}")));
                self.emit(Event::BackendOff);
            }
        }
    }

    fn ensure_ocr(&mut self) -> Option<&Ocr> {
        if self.ocr.is_none() && !self.ocr_failed {
            let r = (|| -> Result<Ocr> {
                if !models::ocr_ready(&self.models_dir) {
                    self.emit(Event::Status(
                        tr("Downloading OCR models (12 MB, one time)…").into(),
                    ));
                    models::download_ocr(&self.models_dir, &Arc::default())?;
                }
                Ocr::load(&models::ocr_dir(&self.models_dir))
            })();
            match r {
                Ok(o) => self.ocr = Some(o),
                Err(e) => {
                    self.ocr_failed = true;
                    self.emit(Event::Error(format!("OCR unavailable: {e:#}")));
                }
            }
        }
        self.ocr.as_ref()
    }

    fn do_ocr(&mut self, id: u64) -> Result<()> {
        let Some(item) = self.store.get(id)? else {
            return Ok(());
        };
        let Some(path) = item.image.clone() else {
            return Ok(());
        };
        if item.ocr.is_some() {
            return Ok(());
        }
        let img = image::open(&path)?;
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
            self.emit(Event::Ocr { id, text });
        }
        Ok(())
    }

    fn embed_batch(&mut self, ids: &[u64]) {
        // Taken out so the store and event channel stay usable while it runs.
        let Some(mut emb) = self.embedder.take() else {
            return;
        };
        let key = emb.key();
        let mut texts: Vec<(u64, String)> = Vec::new();
        for &id in ids {
            let item = match self.store.get(id) {
                Ok(Some(i)) => i,
                _ => continue,
            };
            if item.kind == Kind::Image && emb.supports_images() {
                let r = item
                    .image
                    .clone()
                    .ok_or_else(|| anyhow!("image item without file"))
                    .and_then(|p| Ok(image::open(p)?))
                    .and_then(|img| emb.embed_image(&img));
                self.finish(&key, id, r);
            } else {
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
        }
        if !texts.is_empty() {
            let inputs: Vec<String> = texts.iter().map(|(_, t)| t.clone()).collect();
            match emb.embed_texts(&inputs, TextRole::Document) {
                Ok(vs) => {
                    for ((id, _), e) in texts.iter().zip(vs) {
                        self.finish(&key, *id, Ok(e));
                    }
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
        self.embedder = Some(emb);
    }

    fn finish(&self, key: &str, id: u64, r: Result<Embedding>) {
        let embedding = match r {
            Ok(e) => {
                if let Err(err) = self.store.put_embedding(key, id, &e) {
                    log::warn!("saving embedding failed: {err:#}");
                }
                Some(e)
            }
            Err(e) => {
                log::warn!("embedding item {id} failed: {e:#}");
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
