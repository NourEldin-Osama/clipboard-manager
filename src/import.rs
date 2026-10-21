//! Turning files and folders on disk into searchable items.

use std::path::{Path, PathBuf};

use crate::store::{fnv1a, now, Item, Kind, Origin};

const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff"];
const TEXT_EXT: &[&str] = &[
    "txt", "md", "markdown", "rst", "csv", "tsv", "json", "jsonl", "yaml", "yml", "toml", "xml", "html", "htm",
    "css", "js", "ts", "tsx", "jsx", "py", "rs", "go", "java", "kt", "c", "h", "cpp", "hpp", "cs", "rb", "php",
    "sh", "ps1", "bat", "sql", "log", "ini", "cfg", "tex", "srt", "vtt",
];
const MAX_TEXT_BYTES: u64 = 1 << 20;
pub const MAX_FILES: usize = 20_000;

/// Expands folders recursively. Items get ids from `next_id`.
pub fn collect(paths: &[PathBuf], next_id: &mut u64) -> Vec<Item> {
    let mut files: Vec<PathBuf> = Vec::new();
    for p in paths {
        if p.is_dir() {
            files.extend(
                walkdir::WalkDir::new(p)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .map(|e| e.into_path()),
            );
        } else if p.is_file() {
            files.push(p.clone());
        }
        if files.len() >= MAX_FILES {
            break;
        }
    }
    files.truncate(MAX_FILES);
    files.into_iter().filter_map(|f| file_item(&f, next_id)).collect()
}

fn ext(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn file_item(path: &Path, next_id: &mut u64) -> Option<Item> {
    let e = ext(path);
    let path_str = path.to_string_lossy().into_owned();
    let mut item = Item {
        id: 0,
        kind: Kind::Files,
        origin: Origin::Imported,
        text: path_str.as_str().into(),
        image: None,
        width: 0,
        height: 0,
        ocr: None,
        pinned: false,
        created: now(),
        hash: fnv1a(path_str.as_bytes()),
        source: Some(path.to_path_buf()),
        tags: Vec::new(),
    };
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
    item.id = *next_id;
    *next_id += 1;
    Some(item)
}
