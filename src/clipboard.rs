//! Clipboard monitoring thread and helpers for writing back to the clipboard.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::store::{fnv1a, Formats, ImageFiles, Item, Kind};

/// Formatting bigger than this (HTML and RTF together) isn't kept: the
/// copy pastes as plain text instead.
const MAX_FORMATS_BYTES: usize = 2 * 1024 * 1024;

/// A copy, with the program it came from.
pub struct Capture {
    pub content: Captured,
    /// The program's file name, as in `chrome.exe`, when Windows says.
    pub app: Option<String>,
    /// The HTML and RTF versions of a text copy, if it had any.
    pub formats: Option<Formats>,
}

pub enum Captured {
    Text(String),
    Image(image::RgbaImage),
    Files(Vec<PathBuf>),
    /// A copy that was deliberately not recorded, and why.
    /// `text` is kept only when the user may still choose to save it (the
    /// secret filter can be wrong); a copy marked private by its app is not.
    Skipped {
        reason: SkipReason,
        text: Option<String>,
    },
}

/// Why a copy was not recorded.
#[derive(Clone, Copy, Debug)]
pub enum SkipReason {
    /// The app it came from marked it as private (password managers do).
    Private,
    /// It looks like a password or an API key.
    Secret,
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

pub fn spawn_watcher(flags: Arc<Flags>, tx: Sender<Capture>, wake: impl Fn() + Send + 'static) {
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
                        reason: SkipReason::Private,
                        text: None,
                    }
                } else {
                    match c {
                        Captured::Text(t)
                            if flags.skip_secrets.load(Ordering::SeqCst)
                                && crate::privacy::looks_like_secret(&t) =>
                        {
                            Captured::Skipped {
                                reason: SkipReason::Secret,
                                text: Some(t),
                            }
                        }
                        other => other,
                    }
                };
                if let Captured::Skipped { reason, .. } = &c {
                    log::info!("copy not saved: {reason:?}");
                }
                let formats = match &c {
                    // Only kept when the clipboard didn't change meanwhile.
                    Captured::Text(_) => read_formats().filter(|_| sequence_number() == seq),
                    _ => None,
                };
                let capture = Capture {
                    content: c,
                    app: source_app(),
                    formats,
                };
                if tx.send(capture).is_err() {
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

/// The file name of the program that put the content on the clipboard:
/// the clipboard's owner, or the window in front when there is none.
#[cfg(windows)]
fn source_app() -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::DataExchange::GetClipboardOwner;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    // SAFETY: plain Win32 calls; the process handle is closed before returning,
    // and `name` holds `size` UTF-16 units, as QueryFullProcessImageNameW is told.
    unsafe {
        let mut window = GetClipboardOwner();
        if window.is_null() {
            window = GetForegroundWindow();
        }
        let mut process_id = 0;
        GetWindowThreadProcessId(window, &mut process_id);
        if process_id == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return None;
        }
        let mut name = [0u16; 1024];
        let mut size = name.len() as u32;
        let found = QueryFullProcessImageNameW(process, 0, name.as_mut_ptr(), &mut size);
        CloseHandle(process);
        if found == 0 {
            return None;
        }
        let path = PathBuf::from(String::from_utf16_lossy(&name[..size as usize]));
        Some(path.file_name()?.to_string_lossy().into_owned())
    }
}

#[cfg(not(windows))]
fn source_app() -> Option<String> {
    None
}

/// The clipboard's names for the formatted versions of text.
#[cfg(windows)]
const FORMAT_NAMES: [&str; 2] = ["HTML Format", "Rich Text Format"];

/// The HTML and RTF on the clipboard, as raw bytes, if there are any and
/// they fit in `MAX_FORMATS_BYTES` together.
#[cfg(windows)]
fn read_formats() -> Option<Formats> {
    use clipboard_win::raw;
    let _open = clipboard_win::Clipboard::new_attempts(10).ok()?;
    let mut found = [None, None];
    let mut total = 0;
    for (name, slot) in FORMAT_NAMES.iter().zip(&mut found) {
        let Some(format) = raw::register_format(name) else {
            continue;
        };
        let Some(size) = raw::size(format.get()) else {
            continue;
        };
        total += size.get();
        if total > MAX_FORMATS_BYTES {
            return None;
        }
        let mut data = Vec::new();
        if raw::get_vec(format.get(), &mut data).is_ok() {
            *slot = Some(data);
        }
    }
    let [html, rtf] = found;
    (html.is_some() || rtf.is_some()).then_some(Formats { html, rtf })
}

#[cfg(not(windows))]
fn read_formats() -> Option<Formats> {
    None
}

/// Puts text and its formatted versions on the clipboard together, so the
/// app pasted into picks the richest one it understands.
#[cfg(windows)]
fn copy_formatted(text: &str, formats: &Formats) -> anyhow::Result<()> {
    use clipboard_win::{options::NoClear, raw};
    let start = std::time::Instant::now();
    let _open = loop {
        match clipboard_win::Clipboard::new_attempts(10) {
            Ok(open) => break open,
            Err(_) if start.elapsed() < BUSY_RETRY_FOR => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(_) => anyhow::bail!("the clipboard is busy (another program is holding it open)"),
        }
    };
    let failed = |e: clipboard_win::ErrorCode| anyhow::anyhow!("writing to the clipboard: {e}");
    raw::empty().map_err(failed)?;
    raw::set_string_with(text, NoClear).map_err(failed)?;
    for (name, data) in FORMAT_NAMES.iter().zip([&formats.html, &formats.rtf]) {
        if let (Some(format), Some(data)) = (raw::register_format(name), data) {
            raw::set_without_clear(format.get(), data).map_err(failed)?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn copy_formatted(text: &str, _formats: &Formats) -> anyhow::Result<()> {
    copy_text(text)
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

/// Puts an item from the history back on the clipboard. Slow for a big image
/// (it is read from its file), and it waits while another program holds
/// the clipboard open, so the window calls it from a background thread.
/// `formats` are the text's formatted versions, if it goes back with them.
pub fn copy_item(
    item: &Item,
    images: &ImageFiles,
    formats: Option<&Formats>,
) -> anyhow::Result<()> {
    match item.kind {
        Kind::Text | Kind::Link => match formats {
            Some(formats) => copy_formatted(&item.text, formats),
            None => copy_text(&item.text),
        },
        Kind::Files => copy_files(&item.text.lines().map(PathBuf::from).collect::<Vec<_>>()),
        Kind::Image => {
            let path = item
                .image
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing image"))?;
            copy_image(&images.open(item.id, path)?.into_rgba8())
        }
    }
}

pub fn copy_text(text: &str) -> anyhow::Result<()> {
    write(|cb| cb.set_text(text))
}

fn copy_image(img: &image::RgbaImage) -> anyhow::Result<()> {
    write(|cb| {
        cb.set_image(arboard::ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: std::borrow::Cow::Borrowed(img.as_raw()),
        })
    })
}

fn copy_files(files: &[PathBuf]) -> anyhow::Result<()> {
    write(|cb| cb.set().file_list(files))
}
