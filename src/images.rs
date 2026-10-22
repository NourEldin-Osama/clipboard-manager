//! Decodes thumbnails and previews on background threads so scrolling and
//! selection never wait for a large PNG or JPEG.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui::ColorImage;

/// Large enough for grid cards; list rows scale it down.
pub const THUMB_PX: u32 = 280;
pub const PREVIEW_PX: u32 = 1600;

pub struct Loaded {
    pub id: u64,
    pub preview: bool,
    /// `None` when the file is missing or unreadable.
    pub image: Option<ColorImage>,
    /// Not decoded because a newer preview request superseded it.
    pub skipped: bool,
}

pub struct ImageLoader {
    thumbs: Sender<(u64, PathBuf)>,
    previews: Sender<(u64, PathBuf)>,
    done: Receiver<Loaded>,
    requested: HashSet<(u64, bool)>,
}

impl ImageLoader {
    pub fn new(wake: impl Fn() + Send + Clone + 'static) -> Self {
        let (done_tx, done) = channel();
        // Separate threads, so a long queue of thumbnails never delays the preview.
        let thumbs = spawn("thumbnails", THUMB_PX, false, done_tx.clone(), wake.clone());
        let previews = spawn("preview", PREVIEW_PX, true, done_tx, wake);
        Self {
            thumbs,
            previews,
            done,
            requested: HashSet::new(),
        }
    }

    /// Queues a decode unless one was already requested for this item.
    pub fn request(&mut self, id: u64, path: &Path, preview: bool) {
        if self.requested.insert((id, preview)) {
            let tx = if preview {
                &self.previews
            } else {
                &self.thumbs
            };
            let _ = tx.send((id, path.to_path_buf()));
        }
    }

    pub fn forget(&mut self, id: u64, preview: bool) {
        self.requested.remove(&(id, preview));
    }

    pub fn finished(&self) -> impl Iterator<Item = Loaded> + '_ {
        self.done.try_iter()
    }
}

fn spawn(
    name: &str,
    max: u32,
    preview: bool,
    done: Sender<Loaded>,
    wake: impl Fn() + Send + 'static,
) -> Sender<(u64, PathBuf)> {
    let (tx, rx) = channel::<(u64, PathBuf)>();
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            while let Ok((mut id, mut path)) = rx.recv() {
                if preview {
                    // Only the latest selection matters; skip ones the user moved past.
                    while let Ok((i, p)) = rx.try_recv() {
                        let _ = done.send(Loaded {
                            id,
                            preview,
                            image: None,
                            skipped: true,
                        });
                        id = i;
                        path = p;
                    }
                }
                let image = decode(&path, max);
                if done
                    .send(Loaded {
                        id,
                        preview,
                        image,
                        skipped: false,
                    })
                    .is_err()
                {
                    return;
                }
                wake();
            }
        })
        .expect("spawn image loader");
    tx
}

fn decode(path: &Path, max: u32) -> Option<ColorImage> {
    let img = image::open(path).ok()?;
    let img = if img.width() > max || img.height() > max {
        img.thumbnail(max, max)
    } else {
        img
    };
    let rgba = img.into_rgba8();
    Some(ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}
