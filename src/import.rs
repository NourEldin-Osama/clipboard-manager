//! Turning files and folders on disk into searchable items.

use std::path::{Path, PathBuf};

use crate::store::{Item, Kind, Origin};

const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff"];
const TEXT_EXT: &[&str] = &[
    "txt", "md", "markdown", "rst", "csv", "tsv", "json", "jsonl", "yaml", "yml", "toml", "xml",
    "html", "htm", "css", "js", "ts", "tsx", "jsx", "py", "rs", "go", "java", "kt", "c", "h",
    "cpp", "hpp", "cs", "rb", "php", "sh", "ps1", "bat", "sql", "log", "ini", "cfg", "tex", "srt",
    "vtt",
];
const MAX_TEXT_BYTES: u64 = 1 << 20;
pub const MAX_FILES: usize = 20_000;
/// A batch is handed over at this many files or bytes of text, so saving one
/// batch stays quick.
const BATCH_FILES: usize = 200;
const BATCH_BYTES: usize = 4 << 20;

/// Expands folders recursively and reads the files, handing the items over in
/// batches as it goes. The items get their ids when they are added.
pub fn read_in_batches(paths: &[PathBuf], mut hand_over: impl FnMut(Vec<Item>)) {
    let mut batch = Vec::new();
    let mut bytes = 0;
    for item in list_files(paths).iter().filter_map(|f| file_item(f)) {
        bytes += item.text.len();
        batch.push(item);
        if batch.len() >= BATCH_FILES || bytes >= BATCH_BYTES {
            hand_over(std::mem::take(&mut batch));
            bytes = 0;
        }
    }
    if !batch.is_empty() {
        hand_over(batch);
    }
}

/// The given files, and the files in the given folders, at most MAX_FILES.
fn list_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for p in paths {
        let room = MAX_FILES - files.len();
        if p.is_dir() {
            files.extend(
                walkdir::WalkDir::new(p)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .map(|e| e.into_path())
                    // Stops early: a dropped drive would otherwise be walked to the end.
                    .take(room),
            );
        } else if p.is_file() && room > 0 {
            files.push(p.clone());
        }
        if files.len() >= MAX_FILES {
            break;
        }
    }
    files
}

fn ext(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn file_item(path: &Path) -> Option<Item> {
    let e = ext(path);
    let path_str = path.to_string_lossy().into_owned();
    let mut item = Item::new(0, Kind::Files, Origin::Imported, path_str.as_str().into());
    item.source = Some(path.to_path_buf());
    if IMAGE_EXT.contains(&e.as_str()) {
        let (w, h) = image::image_dimensions(path).ok()?;
        item.kind = Kind::Image;
        item.image = Some(path.to_path_buf());
        item.text = "".into();
        item.width = w;
        item.height = h;
    } else if TEXT_EXT.contains(&e.as_str()) {
        if std::fs::metadata(path).ok()?.len() > MAX_TEXT_BYTES {
            return None;
        }
        let text = String::from_utf8_lossy(&std::fs::read(path).ok()?).into_owned();
        if text.trim().is_empty() {
            return None;
        }
        item.kind = Kind::Text;
        item.text = text.into();
    }
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_read_into_items() {
        let dir = std::env::temp_dir().join(format!("cbm-import-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("inner")).unwrap();
        std::fs::write(dir.join("notes.txt"), "Meeting at 10").unwrap();
        std::fs::write(dir.join("inner").join("readme.md"), "# Title").unwrap();
        std::fs::write(dir.join("empty.txt"), "  ").unwrap();
        std::fs::write(dir.join("data.bin"), [0u8, 1, 2]).unwrap();
        let mut batches = Vec::new();
        read_in_batches(std::slice::from_ref(&dir), |b| batches.push(b));
        assert_eq!(batches.len(), 1);
        let mut found: Vec<(Kind, String)> = batches[0]
            .iter()
            .map(|i| (i.kind, i.text.to_string()))
            .collect();
        // The empty file is left out; a file of unknown type is kept by its path.
        let bin = dir.join("data.bin").to_string_lossy().into_owned();
        let mut expected = vec![
            (Kind::Text, "# Title".to_string()),
            (Kind::Text, "Meeting at 10".to_string()),
            (Kind::Files, bin),
        ];
        found.sort_by(|a, b| a.1.cmp(&b.1));
        expected.sort_by(|a, b| a.1.cmp(&b.1));
        assert_eq!(found, expected);
        std::fs::remove_dir_all(dir).ok();
    }
}
