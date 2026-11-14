//! Decodes thumbnails, previews and pictures shown full screen on background
//! threads so scrolling and selection never wait for a large PNG or JPEG (or
//! for decrypting one).

use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui::ColorImage;

use crate::store::ImageFiles;

/// Large enough for grid cards; list rows scale it down.
pub const THUMB_PX: u32 = 280;
pub const PREVIEW_PX: u32 = 1600;
/// A picture full screen keeps its own size up to this, which graphics
/// cards take as one texture.
pub const FULL_PX: u32 = 8192;

/// What a decode is for, which sets how big it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Size {
    /// A list row or a grid card.
    Thumb,
    /// The details pane.
    Preview,
    /// The picture full screen.
    Full,
}

impl Size {
    /// The longest side it is decoded to.
    fn most_px(self) -> u32 {
        match self {
            Size::Thumb => THUMB_PX,
            Size::Preview => PREVIEW_PX,
            Size::Full => FULL_PX,
        }
    }
}

pub struct Loaded {
    pub id: u64,
    pub size: Size,
    /// `None` when the file is missing or unreadable.
    pub image: Option<ColorImage>,
    /// Not decoded because a newer request of its size superseded it.
    pub skipped: bool,
}

pub struct ImageLoader {
    thumbs: Sender<(u64, PathBuf)>,
    previews: Sender<(u64, PathBuf)>,
    full: Sender<(u64, PathBuf)>,
    done: Receiver<Loaded>,
    requested: HashSet<(u64, Size)>,
}

impl ImageLoader {
    pub fn new(files: ImageFiles, wake: impl Fn() + Send + Clone + 'static) -> Self {
        let (done_tx, done) = channel();
        // Separate threads, so a long queue of thumbnails never delays the
        // preview or the picture full screen.
        let thumbs = spawn(
            "thumbnails",
            Size::Thumb,
            files.clone(),
            done_tx.clone(),
            wake.clone(),
        );
        let previews = spawn(
            "preview",
            Size::Preview,
            files.clone(),
            done_tx.clone(),
            wake.clone(),
        );
        let full = spawn("full picture", Size::Full, files, done_tx, wake);
        Self {
            thumbs,
            previews,
            full,
            done,
            requested: HashSet::new(),
        }
    }

    /// Queues a decode unless one was already requested for this item.
    pub fn request(&mut self, id: u64, path: &Path, size: Size) {
        if self.requested.insert((id, size)) {
            let tx = match size {
                Size::Thumb => &self.thumbs,
                Size::Preview => &self.previews,
                Size::Full => &self.full,
            };
            let _ = tx.send((id, path.to_path_buf()));
        }
    }

    pub fn forget(&mut self, id: u64, size: Size) {
        self.requested.remove(&(id, size));
    }

    pub fn finished(&self) -> impl Iterator<Item = Loaded> + '_ {
        self.done.try_iter()
    }
}

fn spawn(
    name: &str,
    size: Size,
    files: ImageFiles,
    done: Sender<Loaded>,
    wake: impl Fn() + Send + 'static,
) -> Sender<(u64, PathBuf)> {
    let (tx, rx) = channel::<(u64, PathBuf)>();
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            while let Ok((mut id, mut path)) = rx.recv() {
                if size != Size::Thumb {
                    // Only the latest selection matters; skip ones the user moved past.
                    while let Ok((i, p)) = rx.try_recv() {
                        let _ = done.send(Loaded {
                            id,
                            size,
                            image: None,
                            skipped: true,
                        });
                        id = i;
                        path = p;
                    }
                }
                // A file that crashes the decoder shows as missing; the
                // thread goes on with the next one.
                let max = size.most_px();
                let image = catch_unwind(AssertUnwindSafe(|| decode(&files, id, &path, max)))
                    .unwrap_or_else(|_| {
                        log::warn!("decoding the image of item {id} crashed");
                        None
                    });
                if done
                    .send(Loaded {
                        id,
                        size,
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

fn decode(files: &ImageFiles, id: u64, path: &Path, max: u32) -> Option<ColorImage> {
    let img = files.open(id, path).ok()?;
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
