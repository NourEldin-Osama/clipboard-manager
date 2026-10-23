//! Clipboard monitoring thread and helpers for writing back to the clipboard.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::store::fnv1a;

pub enum Captured {
    Text(String),
    Image(image::RgbaImage),
    Files(Vec<PathBuf>),
    /// A copy that was deliberately not recorded, with the reason to show.
    /// `text` is kept only when the user may still choose to save it (the
    /// secret filter can be wrong); a copy marked private by its app is not.
    Skipped {
        reason: &'static str,
        text: Option<String>,
    },
}

impl Captured {
    pub fn hash(&self) -> u64 {
        match self {
            Captured::Text(t) => fnv1a(t.as_bytes()),
            Captured::Image(i) => {
                fnv1a(i.as_raw()) ^ ((i.width() as u64) << 32 | i.height() as u64)
            }
            Captured::Files(f) => fnv1a(join_paths(f).as_bytes()),
            Captured::Skipped { .. } => 0,
        }
    }
}

pub fn join_paths(f: &[PathBuf]) -> String {
    f.iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Default)]
pub struct Flags {
    /// Monitoring on/off.
    pub enabled: AtomicBool,
    /// Skip exactly one upcoming change (the "ignore next copy" hotkey).
    pub ignore_next: AtomicBool,
    /// Don't record text that looks like a password, API key or token.
    pub skip_secrets: AtomicBool,
}

/// How long to keep retrying a clipboard that another program holds open.
/// Browsers and Office can take well over arboard's built-in ~25 ms.
const BUSY_RETRY_FOR: Duration = Duration::from_secs(2);

pub fn spawn_watcher(flags: Arc<Flags>, tx: Sender<Captured>, wake: impl Fn() + Send + 'static) {
    std::thread::Builder::new()
        .name("clipboard".into())
        .spawn(move || {
            let mut cb = loop {
                match arboard::Clipboard::new() {
                    Ok(c) => break c,
                    Err(e) => {
                        log::warn!("clipboard unavailable: {e}");
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
            };
            let mut last_seq = sequence_number();
            // Whatever is on the clipboard at start-up is not a new copy.
            let mut last_hash = read_with_retry(&mut cb).ok().flatten().map(|c| c.hash());
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let seq = sequence_number();
                if seq.is_some() && seq == last_seq {
                    continue;
                }
                let c = match read_with_retry(&mut cb) {
                    Ok(Some(c)) => c,
                    // Empty, or a format we don't record (e.g. only HTML or a custom format).
                    Ok(None) => {
                        last_seq = seq;
                        continue;
                    }
                    Err(e) => {
                        log::warn!(
                            "clipboard stayed busy for {BUSY_RETRY_FOR:?}, skipping this copy: {e}"
                        );
                        last_seq = seq;
                        continue;
                    }
                };
                // Only mark the change as handled once it was actually read.
                last_seq = seq;
                let h = c.hash();
                // Without a change counter (Linux, macOS) the content hash is the only
                // way to tell a new copy from the same one. With it (Windows) every copy
                // counts, so copying the same text again still moves it to the top, and a
                // copy that was skipped earlier is judged again with current settings.
                if seq.is_none() && Some(h) == last_hash {
                    continue;
                }
                last_hash = Some(h);
                if flags.ignore_next.swap(false, Ordering::SeqCst) {
                    log::debug!("skipped one copy (ignore-next or our own copy)");
                    continue;
                }
                if !flags.enabled.load(Ordering::SeqCst) {
                    continue;
                }
                let c = if is_sensitive() {
                    Captured::Skipped {
                        reason: "Not saved: the app you copied from marked it as private",
                        text: None,
                    }
                } else {
                    match c {
                        Captured::Text(t)
                            if flags.skip_secrets.load(Ordering::SeqCst)
                                && crate::privacy::looks_like_secret(&t) =>
                        {
                            Captured::Skipped {
                                reason:
                                    "Not saved: looks like a password or API key (see Settings)",
                                text: Some(t),
                            }
                        }
                        other => other,
                    }
                };
                if let Captured::Skipped { reason, .. } = &c {
                    log::info!("{reason}");
                }
                if tx.send(c).is_err() {
                    return;
                }
                wake();
            }
        })
        .expect("spawn clipboard watcher");
}

/// Reads the clipboard, backing off while another program holds it open.
/// `Ok(None)` means there is nothing we record; `Err` means it stayed busy.
fn read_with_retry(cb: &mut arboard::Clipboard) -> Result<Option<Captured>, arboard::Error> {
    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(20);
    loop {
        match read(cb) {
            Err(arboard::Error::ClipboardOccupied) if start.elapsed() < BUSY_RETRY_FOR => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(250));
            }
            r => return r,
        }
    }
}

fn read(cb: &mut arboard::Clipboard) -> Result<Option<Captured>, arboard::Error> {
    // A busy clipboard must be told apart from "this format isn't there".
    fn check<T>(r: Result<T, arboard::Error>) -> Result<Option<T>, arboard::Error> {
        match r {
            Ok(v) => Ok(Some(v)),
            Err(arboard::Error::ClipboardOccupied) => Err(arboard::Error::ClipboardOccupied),
            Err(_) => Ok(None),
        }
    }
    if let Some(files) = check(cb.get().file_list())? {
        if !files.is_empty() {
            return Ok(Some(Captured::Files(files)));
        }
    }
    if let Some(t) = check(cb.get_text())? {
        if !t.trim().is_empty() {
            return Ok(Some(Captured::Text(t)));
        }
    }
    if let Some(img) = check(cb.get_image())? {
        let (w, h) = (img.width as u32, img.height as u32);
        if let Some(buf) = image::RgbaImage::from_raw(w, h, img.bytes.into_owned()) {
            return Ok(Some(Captured::Image(buf)));
        }
    }
    Ok(None)
}

/// Cheap change counter on Windows, so we only read the clipboard when it changed.
#[cfg(windows)]
fn sequence_number() -> Option<u32> {
    clipboard_win::raw::seq_num().map(|n| n.get())
}

#[cfg(not(windows))]
fn sequence_number() -> Option<u32> {
    None
}

/// Password managers mark secrets with these formats; never record them.
#[cfg(windows)]
fn is_sensitive() -> bool {
    use clipboard_win::raw;
    let _guard = match clipboard_win::Clipboard::new_attempts(5) {
        Ok(g) => g,
        Err(_) => return false,
    };
    [
        "ExcludeClipboardContentFromMonitorProcessing",
        "Clipboard Viewer Ignore",
    ]
    .iter()
    .filter_map(|n| raw::register_format(n))
    .any(|f| raw::is_format_avail(f.get()))
}

#[cfg(not(windows))]
fn is_sensitive() -> bool {
    false
}

/// One long-lived handle for writing. On Linux the copied data is served by
/// this handle, so dropping it right after a copy would make the data vanish.
static WRITER: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// Writes to the clipboard, retrying while another program holds it open.
fn write(f: impl Fn(&mut arboard::Clipboard) -> Result<(), arboard::Error>) -> anyhow::Result<()> {
    let mut guard = WRITER.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = Some(arboard::Clipboard::new()?);
    }
    let cb = guard.as_mut().unwrap();
    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(20);
    loop {
        match f(cb) {
            Ok(()) => return Ok(()),
            Err(arboard::Error::ClipboardOccupied) if start.elapsed() < BUSY_RETRY_FOR => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(250));
            }
            Err(arboard::Error::ClipboardOccupied) => {
                anyhow::bail!("the clipboard is busy (another program is holding it open)")
            }
            Err(e) => return Err(e.into()),
        }
    }
}

pub fn copy_text(text: &str) -> anyhow::Result<()> {
    write(|cb| cb.set_text(text))
}

pub fn copy_image(img: &image::RgbaImage) -> anyhow::Result<()> {
    write(|cb| {
        cb.set_image(arboard::ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: std::borrow::Cow::Borrowed(img.as_raw()),
        })
    })
}

pub fn copy_files(files: &[PathBuf]) -> anyhow::Result<()> {
    write(|cb| cb.set().file_list(files))
}
