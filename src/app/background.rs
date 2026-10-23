//! Slow work the window must not wait for. Saving a copied image as PNG and
//! putting an item back on the clipboard run in order on one thread; each
//! import reads its files on a thread of its own, so a long import never holds
//! up a copy. Results come back to `App` as `Done` messages.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui;

use crate::store::Item;

/// A finished piece of background work.
pub(super) enum Done {
    /// A copied image, with its PNG file written, or why it wasn't.
    ImageSaved {
        item: Item,
        saved: Result<PathBuf, String>,
    },
    /// An item put back on the clipboard, or why it wasn't.
    Copied { id: u64, result: Result<(), String> },
    /// Text read from an image put on the clipboard, or why it wasn't.
    TextCopied(Result<(), String>),
    /// Files read by an import, not saved yet.
    Imported(Vec<Item>),
    /// An import has read all its files.
    ImportFinished,
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
            result: Ok(()),
        });
        background.run(|| panic!("a task that crashes"));
        background.run(|| Done::Copied {
            id: 2,
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
