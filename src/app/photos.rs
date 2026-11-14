//! Opening a picture in Windows' own Photos app. Pictures are stored
//! encrypted, so Photos gets a copy as PNG in a folder of the app's among
//! Windows' temporary files. The copies go at start and at quit; one that
//! Photos still holds open stays, and goes the next time.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::store::ImageFiles;

/// Where the copies go: a folder of the app's among Windows' temporary
/// files, which stay on this PC.
fn copies_folder() -> PathBuf {
    std::env::temp_dir()
        .join("clipboard-manager")
        .join("photos")
}

/// The name of the `n`th copy of item `id`'s picture: clip-12.png, then
/// clip-12-2.png and so on while Photos still holds the earlier ones.
fn copy_name(id: u64, n: u32) -> String {
    if n <= 1 {
        format!("clip-{id}.png")
    } else {
        format!("clip-{id}-{n}.png")
    }
}

/// Whether `name` is the name of a copy, so only copies are deleted.
fn is_copy_name(name: &str) -> bool {
    let Some(middle) = name
        .strip_prefix("clip-")
        .and_then(|rest| rest.strip_suffix(".png"))
    else {
        return false;
    };
    let numbers: Vec<&str> = middle.split('-').collect();
    numbers.len() <= 2
        && numbers
            .iter()
            .all(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

/// A path in `folder` that no file has yet, for a copy of item `id`.
fn free_copy_path(folder: &Path, id: u64) -> PathBuf {
    (1..)
        .map(|n| folder.join(copy_name(id, n)))
        .find(|path| !path.exists())
        .expect("one of the names is free")
}

/// Writes item `id`'s picture, stored at `stored`, as a PNG copy for
/// Photos. Returns where it went.
pub(super) fn write_copy(images: &ImageFiles, id: u64, stored: &Path) -> anyhow::Result<PathBuf> {
    write_copy_in(&copies_folder(), images, id, stored)
}

fn write_copy_in(
    folder: &Path,
    images: &ImageFiles,
    id: u64,
    stored: &Path,
) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(folder).with_context(|| format!("making {}", folder.display()))?;
    let path = free_copy_path(folder, id);
    images
        .open(id, stored)?
        .save(&path)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// Deletes the copies made before. One that Photos still holds open stays
/// until the next time; that is no error.
pub(super) fn delete_old_copies() {
    let left = delete_copies_in(&copies_folder());
    if left > 0 {
        log::info!("{left} copies for Photos are still open; they go next time");
    }
}

/// Deletes the copies in `folder`, and nothing else there. How many could
/// not be deleted.
fn delete_copies_in(folder: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().is_some_and(is_copy_name))
        .filter(|entry| std::fs::remove_file(entry.path()).is_err())
        .count()
}

/// The link that opens the picture at `path` in Photos. The characters
/// that would end or split the link's value are written as their codes;
/// the others stay as they are, as Photos reads a path.
fn photos_link(path: &Path) -> String {
    let mut link = String::from("ms-photos:viewer?fileName=");
    for c in path.to_string_lossy().chars() {
        match c {
            '%' => link.push_str("%25"),
            '&' => link.push_str("%26"),
            '#' => link.push_str("%23"),
            '+' => link.push_str("%2B"),
            c => link.push(c),
        }
    }
    link
}

/// Opens the picture at `path` in Photos, or, when Photos doesn't start,
/// in the app Windows opens pictures with.
pub(super) fn show(path: &Path) -> std::io::Result<()> {
    let link = photos_link(path);
    match shell_open(OsStr::new(&link), true) {
        Ok(()) => Ok(()),
        Err(e) => {
            log::info!("Photos didn't open the picture, so the app for pictures does: {e}");
            shell_open(path.as_os_str(), false)
        }
    }
}

/// Asks Windows to open `target`, a file or a link, as a double-click on it
/// would. With `is_quiet`, a failure shows no message of Windows' own, so
/// another way can be tried.
fn shell_open(target: &OsStr, is_quiet: bool) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SHELLEXECUTEINFOW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide = |text: &OsStr| -> Vec<u16> { text.encode_wide().chain(Some(0)).collect() };
    let (verb, file) = (wide(OsStr::new("open")), wide(target));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: if is_quiet { SEE_MASK_FLAG_NO_UI } else { 0 },
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        nShow: SW_SHOWNORMAL,
        ..Default::default()
    };
    // SAFETY: `info` is set up as ShellExecuteExW needs; its strings end
    // with a null and live until after the call.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new, empty folder for one test.
    fn empty_folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("cbm-photos-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&folder).ok();
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn copies_have_names_of_their_own() {
        assert_eq!(copy_name(12, 1), "clip-12.png");
        assert_eq!(copy_name(12, 2), "clip-12-2.png");
        for name in ["clip-12.png", "clip-12-2.png", "clip-0.png"] {
            assert!(is_copy_name(name), "{name}");
        }
        let others = [
            "clip-.png",
            "clip-12-.png",
            "clip-a.png",
            "clip-1-2-3.png",
            "clip-12.png.part",
            "clip-12.jpg",
            "photo.png",
            "notes.txt",
        ];
        for name in others {
            assert!(!is_copy_name(name), "{name}");
        }
    }

    /// A copy takes the first free name. Deleting the copies leaves other
    /// files alone, and one still held open stays until the next time.
    #[test]
    fn copies_take_a_free_name_and_go_later() {
        use std::os::windows::fs::OpenOptionsExt;
        let folder = empty_folder("names");
        assert_eq!(free_copy_path(&folder, 5), folder.join("clip-5.png"));
        std::fs::write(folder.join("clip-5.png"), b"first").unwrap();
        assert_eq!(free_copy_path(&folder, 5), folder.join("clip-5-2.png"));
        std::fs::write(folder.join("clip-5-2.png"), b"second").unwrap();
        std::fs::write(folder.join("notes.txt"), b"not a copy").unwrap();
        // Held open with no sharing, as an app showing it could.
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(folder.join("clip-5-2.png"))
            .unwrap();
        assert_eq!(delete_copies_in(&folder), 1);
        assert!(!folder.join("clip-5.png").exists());
        assert!(folder.join("clip-5-2.png").exists());
        assert!(folder.join("notes.txt").exists());
        drop(held);
        assert_eq!(delete_copies_in(&folder), 0);
        assert!(!folder.join("clip-5-2.png").exists());
        assert!(folder.join("notes.txt").exists());
        // No folder yet: nothing to delete.
        assert_eq!(delete_copies_in(&folder.join("missing")), 0);
        std::fs::remove_dir_all(folder).ok();
    }

    /// The copy of a picture stored encrypted is a plain PNG with the same
    /// pixels, and a second copy takes a name of its own.
    #[test]
    fn a_copy_is_the_picture_as_plain_png() {
        let folder = empty_folder("copy");
        let store = crate::store::Store::open(&folder.join("history.redb"), &folder).unwrap();
        let picture = image::RgbaImage::from_fn(5, 3, |x, y| {
            image::Rgba([x as u8 * 40, y as u8 * 80, 7, 255])
        });
        let stored = store.images.save(9, &picture).unwrap();
        let copies = folder.join("copies");
        let copy = write_copy_in(&copies, &store.images, 9, &stored).unwrap();
        assert_eq!(copy, copies.join("clip-9.png"));
        let bytes = std::fs::read(&copy).unwrap();
        assert!(bytes.starts_with(b"\x89PNG"), "a plain PNG");
        let read = image::load_from_memory(&bytes).unwrap().into_rgba8();
        assert_eq!(read, picture);
        let again = write_copy_in(&copies, &store.images, 9, &stored).unwrap();
        assert_eq!(again, copies.join("clip-9-2.png"));
        drop(store);
        std::fs::remove_dir_all(folder).ok();
    }

    /// The path goes in the link as it is, but for the characters that
    /// would end or split its value.
    #[test]
    fn the_photos_link_keeps_the_whole_path() {
        let plain =
            Path::new(r"C:\Users\Sam Lee\AppData\Local\Temp\clipboard-manager\photos\clip-3.png");
        assert_eq!(
            photos_link(plain),
            r"ms-photos:viewer?fileName=C:\Users\Sam Lee\AppData\Local\Temp\clipboard-manager\photos\clip-3.png"
        );
        let odd = Path::new(r"C:\A&B #1 +50%\clip-3.png");
        assert_eq!(
            photos_link(odd),
            r"ms-photos:viewer?fileName=C:\A%26B %231 %2B50%25\clip-3.png"
        );
    }
}
