//! Slow work the window must not wait for. Saving a copied image as PNG and
//! putting an item back on the clipboard run in order on one thread; each
//! import, and each backup written or restored, runs on a thread of its own,
//! so a long one never holds up a copy. Results come back to `App` as `Done`
//! messages.

use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;

use super::Paste;
use crate::backup::{Problem, Restored, Step};
use crate::i18n::log_error;
use crate::library::Ids;
use crate::store::{Item, Kind, Store};

/// How often a backup's progress redraws the window at most.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// How far a backup's thread is, read by the window as it draws.
#[derive(Default)]
pub(super) struct BackupProgress {
    /// Items written, or restored and skipped, so far.
    pub(super) done: AtomicUsize,
    /// How many items there are; none yet while a restore checks the file.
    pub(super) total: AtomicUsize,
}

/// What a restore needs from the window.
pub(super) struct RestoreRequest {
    pub(super) file: PathBuf,
    pub(super) password: String,
    pub(super) store: Arc<Store>,
    /// The kind and content of every item in the history.
    pub(super) known: HashSet<(Kind, u64)>,
    pub(super) ids: Ids,
}

/// A finished piece of background work.
pub(super) enum Done {
    /// A copied image, with its PNG file written, or why it wasn't.
    ImageSaved {
        item: Box<Item>,
        saved: Result<PathBuf, String>,
    },
    /// An item put back on the clipboard, or why it wasn't.
    Copied {
        id: u64,
        paste: Paste,
        result: Result<(), String>,
    },
    /// Text read from an image put on the clipboard, or why it wasn't.
    TextCopied(Result<(), String>),
    /// A picture written as a PNG for Photos: where, or why it wasn't.
    #[cfg(windows)]
    CopiedForPhotos(Result<PathBuf, String>),
    /// The texts of several items put on the clipboard together, or why
    /// they weren't.
    CopiedTogether {
        /// How many items' texts.
        count: usize,
        /// Pictures left out.
        pictures: usize,
        paste: Paste,
        result: Result<(), String>,
    },
    /// Files read by an import, not saved yet.
    Imported(Vec<Item>),
    /// An import has read all its files.
    ImportFinished,
    /// A backup was written: how many items it holds, and where; or why it
    /// wasn't.
    BackupSaved(Result<(usize, PathBuf), String>),
    /// The password opened the backup being restored.
    BackupOpened,
    /// Items of a backup, with their formatting and pictures saved, to add
    /// to the history.
    Restored(Vec<Item>),
    /// A restore is over: what it added and skipped, or why it stopped.
    RestoreFinished(Result<Restored, Problem>),
    /// A task crashed; the ones after it still run.
    Crashed,
}

type Task = Box<dyn FnOnce() -> Done + Send>;

/// Hands results to the window and wakes it up.
#[derive(Clone)]
struct Reply {
    tx: Sender<Done>,
    ctx: egui::Context,
}

impl Reply {
    fn send(&self, done: Done) {
        let _ = self.tx.send(done);
        self.ctx.request_repaint();
    }
}

pub(super) struct Background {
    tasks: Sender<Task>,
    reply: Reply,
    pub(super) done: Receiver<Done>,
}

impl Background {
    pub(super) fn spawn(ctx: egui::Context) -> Self {
        let (tx, done) = channel();
        let reply = Reply { tx, ctx };
        let (tasks, queue) = channel::<Task>();
        let r = reply.clone();
        std::thread::Builder::new()
            .name("background".into())
            .spawn(move || {
                for task in queue {
                    let done = catch_unwind(AssertUnwindSafe(task)).unwrap_or(Done::Crashed);
                    r.send(done);
                }
            })
            .expect("spawn background thread");
        Self { tasks, reply, done }
    }

    /// Runs `task` after the tasks sent before it.
    pub(super) fn run(&self, task: impl FnOnce() -> Done + Send + 'static) {
        let _ = self.tasks.send(Box::new(task));
    }

    /// Reads the files and folders on a new thread, sending them in batches.
    /// Always ends with `ImportFinished`.
    pub(super) fn import(&self, paths: Vec<PathBuf>) {
        let reply = self.reply.clone();
        let started = std::thread::Builder::new()
            .name("import".into())
            .spawn(move || {
                let read = catch_unwind(AssertUnwindSafe(|| {
                    crate::import::read_in_batches(&paths, |batch| {
                        reply.send(Done::Imported(batch))
                    });
                }));
                if read.is_err() {
                    log::error!("the import stopped early: reading a file crashed");
                }
                reply.send(Done::ImportFinished);
            });
        if let Err(e) = started {
            log::error!("could not start the import: {e}");
            self.reply.send(Done::ImportFinished);
        }
    }

    /// Writes a backup of `items` to `file`, locked with `password`, on a
    /// thread of its own. Always ends with `BackupSaved`.
    pub(super) fn export_backup(
        &self,
        store: Arc<Store>,
        items: Vec<Arc<Item>>,
        file: PathBuf,
        password: String,
        progress: Arc<BackupProgress>,
    ) {
        let reply = self.reply.clone();
        let started = std::thread::Builder::new()
            .name("backup".into())
            .spawn(move || {
                let wake = reply.ctx.clone();
                let written = catch_unwind(AssertUnwindSafe(|| {
                    crate::backup::export(&store, &items, &file, &password, |done| {
                        progress.done.store(done, Ordering::Relaxed);
                        wake.request_repaint_after(PROGRESS_EVERY);
                    })
                }));
                let saved = match written {
                    Ok(Ok(count)) => Ok((count, file)),
                    Ok(Err(e)) => Err(format!("{e:#}")),
                    Err(_) => Err(log_error!("error-background-crash")),
                };
                reply.send(Done::BackupSaved(saved));
            });
        if let Err(e) = started {
            self.reply.send(Done::BackupSaved(Err(e.to_string())));
        }
    }

    /// Restores a backup on a thread of its own: `BackupOpened` once the
    /// password opens it, its items in batches of `Restored`, and always
    /// `RestoreFinished` at the end.
    pub(super) fn restore_backup(&self, request: RestoreRequest, progress: Arc<BackupProgress>) {
        let reply = self.reply.clone();
        let started = std::thread::Builder::new()
            .name("restore".into())
            .spawn(move || {
                let RestoreRequest {
                    file,
                    password,
                    store,
                    known,
                    ids,
                } = request;
                let wake = reply.ctx.clone();
                let on_step = |step| match step {
                    Step::Opened => reply.send(Done::BackupOpened),
                    Step::Checked(total) => {
                        progress.total.store(total, Ordering::Relaxed);
                        wake.request_repaint();
                    }
                    Step::Done(done) => {
                        progress.done.store(done, Ordering::Relaxed);
                        wake.request_repaint_after(PROGRESS_EVERY);
                    }
                };
                let restored = catch_unwind(AssertUnwindSafe(|| {
                    crate::backup::restore(
                        &file,
                        &password,
                        &store,
                        &known,
                        || ids.next(),
                        on_step,
                        |batch| reply.send(Done::Restored(batch)),
                    )
                }));
                let finished = restored.unwrap_or_else(|_| {
                    let crashed = log_error!("error-background-crash");
                    Err(Problem::Failed(anyhow::anyhow!(crashed)))
                });
                reply.send(Done::RestoreFinished(finished));
            });
        if let Err(e) = started {
            let e = Problem::Failed(e.into());
            self.reply.send(Done::RestoreFinished(Err(e)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn next(background: &Background) -> Done {
        background
            .done
            .recv_timeout(Duration::from_secs(10))
            .expect("a result within 10 s")
    }

    #[test]
    fn tasks_run_in_order_and_a_crash_does_not_stop_the_next_ones() {
        let background = Background::spawn(egui::Context::default());
        background.run(|| Done::Copied {
            id: 1,
            paste: Paste::No,
            result: Ok(()),
        });
        background.run(|| panic!("a task that crashes"));
        background.run(|| Done::Copied {
            id: 2,
            paste: Paste::No,
            result: Ok(()),
        });
        assert!(matches!(next(&background), Done::Copied { id: 1, .. }));
        assert!(matches!(next(&background), Done::Crashed));
        assert!(matches!(next(&background), Done::Copied { id: 2, .. }));
    }

    #[test]
    fn an_import_sends_its_files_then_finishes() {
        let dir = std::env::temp_dir().join(format!("cbm-background-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "alpha").unwrap();
        let background = Background::spawn(egui::Context::default());
        background.import(vec![dir.clone()]);
        assert!(matches!(next(&background), Done::Imported(batch) if batch.len() == 1));
        assert!(matches!(next(&background), Done::ImportFinished));
        // Nothing to read still finishes, so the status bar never waits forever.
        background.import(vec![dir.join("missing")]);
        assert!(matches!(next(&background), Done::ImportFinished));
        std::fs::remove_dir_all(dir).ok();
    }
}
