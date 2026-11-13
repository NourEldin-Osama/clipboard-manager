//! The programs copies come from, as Windows shows them: each one's own
//! name and icon. They are read on a background thread, once per program,
//! and kept for the run.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

/// What the window shows for a program.
pub(super) struct Program {
    /// Its own name, such as "Snipping Tool", or else its file name.
    pub(super) name: String,
    pub(super) icon: Option<TextureHandle>,
}

/// What Windows says about a program, read on the background thread.
struct Description {
    name: String,
    icon: Option<ColorImage>,
}

pub(super) struct Programs {
    ctx: egui::Context,
    ask: Sender<PathBuf>,
    answers: Receiver<(PathBuf, Description)>,
    /// `None` while it is being read.
    known: HashMap<PathBuf, Option<Program>>,
}

impl Programs {
    pub(super) fn new(ctx: egui::Context) -> Self {
        let (ask, questions) = channel::<PathBuf>();
        let (tell, answers) = channel();
        let wake = ctx.clone();
        std::thread::Builder::new()
            .name("programs".into())
            .spawn(move || {
                for path in questions {
                    // A program that crashes the reading gets its file name
                    // and no icon; the thread goes on with the next one.
                    let description =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| describe(&path)))
                            .unwrap_or_else(|_| {
                                log::warn!("reading {} crashed", path.display());
                                Description {
                                    name: file_name(&path),
                                    icon: None,
                                }
                            });
                    if tell.send((path, description)).is_err() {
                        return;
                    }
                    wake.request_repaint();
                }
            })
            .expect("spawn the programs thread");
        Self {
            ctx,
            ask,
            answers,
            known: HashMap::new(),
        }
    }

    /// The program at `path`, once it has been read. The first time it is
    /// asked for, it is read in the background.
    pub(super) fn get(&mut self, path: &Path) -> Option<&Program> {
        for (path, description) in self.answers.try_iter() {
            let icon = description.icon.map(|image| {
                let name = format!("program:{}", path.display());
                self.ctx.load_texture(name, image, TextureOptions::LINEAR)
            });
            let name = description.name;
            self.known.insert(path, Some(Program { name, icon }));
        }
        if !self.known.contains_key(path) {
            self.known.insert(path.to_path_buf(), None);
            let _ = self.ask.send(path.to_path_buf());
        }
        self.known.get(path)?.as_ref()
    }
}

/// The program's own name from its file description, or its file name; and
/// its icon.
fn describe(path: &Path) -> Description {
    Description {
        name: file_description(path).unwrap_or_else(|| file_name(path)),
        icon: icon(path),
    }
}

/// The program's file name as a name, as `tidy` gives it.
fn file_name(path: &Path) -> String {
    path.file_stem()
        .map(|name| tidy(&name.to_string_lossy()))
        .unwrap_or_default()
}

/// The name a copy's program shows: its own name once it has been read,
/// else its file name (`app`) as `tidy` gives it.
pub(super) fn shown_name(app: &str, program: Option<&Program>) -> String {
    program.map_or_else(|| tidy(app), |p| p.name.clone())
}

/// The own names of well-known programs whose file names say less, for
/// copies whose program file can't be read (made before 0.6, or a program
/// Windows keeps out of reach). Names, the same in every language.
const KNOWN_PROGRAMS: [(&str, &str); 26] = [
    ("acrobat", "Adobe Acrobat"),
    ("acrord32", "Adobe Acrobat Reader"),
    ("chrome", "Google Chrome"),
    ("cmd", "Windows Command Processor"),
    ("code", "Visual Studio Code"),
    ("devenv", "Visual Studio"),
    ("discord", "Discord"),
    ("excel", "Microsoft Excel"),
    ("explorer", "Windows Explorer"),
    ("figma", "Figma"),
    ("firefox", "Firefox"),
    ("ms-teams", "Microsoft Teams"),
    ("msedge", "Microsoft Edge"),
    ("mspaint", "Paint"),
    ("notepad", "Notepad"),
    ("obsidian", "Obsidian"),
    ("olk", "Outlook"),
    ("onenote", "Microsoft OneNote"),
    ("outlook", "Microsoft Outlook"),
    ("powerpnt", "Microsoft PowerPoint"),
    ("slack", "Slack"),
    ("snippingtool", "Snipping Tool"),
    ("teams", "Microsoft Teams"),
    ("telegram", "Telegram"),
    ("windowsterminal", "Windows Terminal"),
    ("winword", "Microsoft Word"),
];

/// A program's file name as a name: a well-known program's own name, or
/// else the file name without `.exe`, starting with a capital, and in small
/// letters after it when the whole name is in capitals. "WINWORD.EXE"
/// becomes "Microsoft Word" and "tool.exe" "Tool".
pub(super) fn tidy(file: &str) -> String {
    let stem = match file.len().checked_sub(4) {
        Some(cut) if file.is_char_boundary(cut) && file[cut..].eq_ignore_ascii_case(".exe") => {
            &file[..cut]
        }
        _ => file,
    };
    if let Some((_, name)) = KNOWN_PROGRAMS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(stem))
    {
        return (*name).to_string();
    }
    // Short names in capitals are usually letters said one by one.
    let letters = stem.chars().filter(|c| c.is_alphabetic()).count();
    let in_capitals = letters > 3 && !stem.chars().any(char::is_lowercase);
    let mut chars = stem.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let rest: String = chars.collect();
    let rest = if in_capitals {
        rest.to_lowercase()
    } else {
        rest
    };
    first.to_uppercase().chain(rest.chars()).collect()
}

#[cfg(windows)]
fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain([0]).collect()
}

/// The description in the program's version information, as File Explorer
/// shows it: "Snipping Tool" for SnippingTool.exe.
#[cfg(windows)]
fn file_description(path: &Path) -> Option<String> {
    use std::ffi::c_void;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    };
    let file = wide(path);
    // SAFETY: the buffer is the size Windows asked for, and every value read
    // from it lies inside it, where VerQueryValueW points.
    unsafe {
        let size = GetFileVersionInfoSizeW(file.as_ptr(), std::ptr::null_mut());
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(file.as_ptr(), 0, size, data.as_mut_ptr().cast()) == 0 {
            return None;
        }
        let query = |what: &str| -> Option<(*mut c_void, u32)> {
            let what: Vec<u16> = what.encode_utf16().chain([0]).collect();
            let (mut value, mut len) = (std::ptr::null_mut(), 0);
            let found = VerQueryValueW(data.as_ptr().cast(), what.as_ptr(), &mut value, &mut len);
            (found != 0 && !value.is_null() && len > 0).then_some((value, len))
        };
        // The first language and code page the file has its texts in.
        let (codes, _) = query("\\VarFileInfo\\Translation")?;
        let codes = codes.cast::<u16>();
        let (language, code_page) = (*codes, *codes.add(1));
        let (text, len) = query(&format!(
            "\\StringFileInfo\\{language:04x}{code_page:04x}\\FileDescription"
        ))?;
        let text = std::slice::from_raw_parts(text.cast::<u16>(), len as usize);
        let name = String::from_utf16_lossy(text);
        let name = name.trim_end_matches('\0').trim();
        (!name.is_empty()).then(|| name.to_string())
    }
}

#[cfg(not(windows))]
fn file_description(_path: &Path) -> Option<String> {
    None
}

/// The program's icon at 48 pixels.
#[cfg(windows)]
fn icon(path: &Path) -> Option<ColorImage> {
    use windows_sys::Win32::Graphics::Gdi::DeleteObject;
    use windows_sys::Win32::UI::Shell::SHDefExtractIconW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};
    const SIZE: u32 = 48;
    let file = wide(path);
    // SAFETY: plain Win32 calls; the icon and its two bitmaps are freed
    // before returning.
    unsafe {
        let mut icon = std::ptr::null_mut();
        let found = SHDefExtractIconW(file.as_ptr(), 0, 0, &mut icon, std::ptr::null_mut(), SIZE);
        if found != 0 || icon.is_null() {
            return None;
        }
        let mut info: ICONINFO = std::mem::zeroed();
        let has_info = GetIconInfo(icon, &mut info) != 0;
        let color = (has_info && !info.hbmColor.is_null())
            .then(|| bitmap_pixels(info.hbmColor))
            .flatten();
        let mask = (has_info && !info.hbmMask.is_null())
            .then(|| bitmap_pixels(info.hbmMask))
            .flatten();
        if !info.hbmColor.is_null() {
            DeleteObject(info.hbmColor);
        }
        if !info.hbmMask.is_null() {
            DeleteObject(info.hbmMask);
        }
        DestroyIcon(icon);
        let (width, height, mut bgra) = color?;
        // An icon without alpha shows where its mask is black.
        if bgra.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0) {
            let mask = mask.filter(|(w, h, _)| (*w, *h) == (width, height));
            for (i, pixel) in bgra.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let hidden = mask.as_ref().is_some_and(|(_, _, m)| m[i * 4] > 127);
                pixel[3] = if hidden { 0 } else { 255 };
            }
        }
        for pixel in bgra.as_chunks_mut::<4>().0.iter_mut() {
            pixel.swap(0, 2);
        }
        Some(ColorImage::from_rgba_unmultiplied([width, height], &bgra))
    }
}

#[cfg(not(windows))]
fn icon(_path: &Path) -> Option<ColorImage> {
    None
}

/// A bitmap's pixels as 32-bit BGRA, top row first.
///
/// # Safety
/// `bitmap` must be a valid bitmap handle.
#[cfg(windows)]
unsafe fn bitmap_pixels(
    bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
) -> Option<(usize, usize, Vec<u8>)> {
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS,
    };
    // SAFETY: the caller passes a valid bitmap; GetDIBits writes `height`
    // rows of `width` 4-byte pixels, which is the buffer's size.
    unsafe {
        let mut found: BITMAP = std::mem::zeroed();
        let size = std::mem::size_of::<BITMAP>() as i32;
        if GetObjectW(bitmap, size, (&mut found as *mut BITMAP).cast()) == 0 {
            return None;
        }
        let (width, height) = (found.bmWidth, found.bmHeight);
        if width <= 0 || height <= 0 {
            return None;
        }
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            // Negative: top row first.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        let rows = GetDIBits(
            dc,
            bitmap,
            0,
            height as u32,
            pixels.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        );
        DeleteDC(dc);
        (rows == height).then_some((width as usize, height as usize, pixels))
    }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn file_names_read_as_names() {
        // Well-known programs by their own names.
        assert_eq!(tidy("WINWORD.EXE"), "Microsoft Word");
        assert_eq!(tidy("Code.exe"), "Visual Studio Code");
        assert_eq!(tidy("SnippingTool.exe"), "Snipping Tool");
        // Others by their file names.
        assert_eq!(tidy("MYNOTES.EXE"), "Mynotes");
        assert_eq!(tidy("tool.exe"), "Tool");
        // Letters said one by one keep their capitals.
        assert_eq!(tidy("ABC.EXE"), "ABC");
        // Only a last `.exe` goes.
        assert_eq!(tidy("app.exe.config"), "App.exe.config");
        assert_eq!(tidy(""), "");
    }

    #[test]
    fn known_programs_are_in_order_and_once() {
        let files: Vec<&str> = KNOWN_PROGRAMS.iter().map(|(f, _)| *f).collect();
        let mut sorted = files.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(files, sorted);
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn a_program_has_its_own_name_and_an_icon() {
        let windows = std::env::var_os("SystemRoot").expect("SystemRoot");
        let explorer = PathBuf::from(windows).join("explorer.exe");
        let found = describe(&explorer);
        // Its description, not its file name: "Windows Explorer" in English.
        assert_ne!(found.name, "explorer");
        assert!(!found.name.is_empty());
        let icon = found.icon.expect("an icon");
        assert_eq!(icon.size, [48, 48]);
        assert!(icon.pixels.iter().any(|pixel| pixel.a() > 0));
    }
}
