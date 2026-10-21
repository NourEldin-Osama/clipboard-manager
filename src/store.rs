//! Persistent storage for history items and their embeddings (redb, pure Rust).
//!
//! Images are written as PNG files next to the database; the database keeps
//! metadata, OCR text and one embedding per (model, item).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::embed::Embedding;

const ITEMS: TableDefinition<u64, &[u8]> = TableDefinition::new("items");
const EMBEDDINGS: TableDefinition<(&str, u64), &[u8]> = TableDefinition::new("embeddings");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Text,
    Link,
    Image,
    Files,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Text => "Text",
            Kind::Link => "Link",
            Kind::Image => "Image",
            Kind::Files => "Files",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    Clipboard,
    /// Added from disk via import / drag-and-drop; never auto-pruned.
    Imported,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub kind: Kind,
    pub origin: Origin,
    /// Text content, or the newline-separated paths for `Files`.
    /// Shared so that cloning an item never copies large text.
    pub text: Arc<str>,
    /// PNG on disk for `Image` items.
    pub image: Option<PathBuf>,
    pub width: u32,
    pub height: u32,
    /// Text recognised in the image; `None` = not processed yet.
    pub ocr: Option<Arc<str>>,
    pub pinned: bool,
    /// Unix seconds.
    pub created: i64,
    pub hash: u64,
    /// File this item was imported from.
    #[serde(default)]
    pub source: Option<PathBuf>,
    /// User labels, as typed; compared case-insensitively.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Item {
    /// Everything keyword search looks at.
    pub fn searchable_text(&self) -> String {
        let mut s = String::from(&*self.text);
        if let Some(o) = self.ocr.as_deref().filter(|o| !o.is_empty()) {
            s.push('\n');
            s.push_str(o);
        }
        if let Some(p) = &self.source {
            s.push('\n');
            s.push_str(&p.to_string_lossy());
        }
        // "#work" and plain "work" both find a tagged item.
        for t in &self.tags {
            s.push_str("\n#");
            s.push_str(t);
        }
        s
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }

    /// Kept by pruning and "Clear history": the user marked it as worth keeping.
    pub fn is_kept(&self) -> bool {
        self.pinned || !self.tags.is_empty()
    }

    /// First non-empty line, shortened; never copies the whole text.
    pub fn title(&self) -> String {
        let src: &str = match self.kind {
            Kind::Image => match self.ocr.as_deref() {
                Some(o) if !o.trim().is_empty() => o,
                _ => {
                    return match &self.source {
                        Some(p) => p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(),
                        None => crate::i18n::trf("Image {w}×{h}", &[("w", &self.width), ("h", &self.height)]),
                    }
                }
            },
            _ => &self.text,
        };
        let line = src.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
        let mut out: String = line.chars().take(120).collect();
        if line.chars().nth(120).is_some() {
            out.push('…');
        }
        out
    }
}

pub struct Store {
    db: Database,
    pub images_dir: PathBuf,
}

impl Store {
    pub fn open(db_path: &Path, images_dir: &Path) -> Result<Self> {
        let db = Database::create(db_path).context("opening database")?;
        let txn = db.begin_write()?;
        txn.open_table(ITEMS)?;
        txn.open_table(EMBEDDINGS)?;
        txn.commit()?;
        Ok(Self { db, images_dir: images_dir.to_path_buf() })
    }

    pub fn all_items(&self) -> Result<Vec<Item>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ITEMS)?;
        let mut out = Vec::new();
        for row in table.iter()? {
            let (_, v) = row?;
            match serde_json::from_slice::<Item>(v.value()) {
                Ok(item) => out.push(item),
                Err(e) => log::warn!("skipping unreadable item: {e}"),
            }
        }
        Ok(out)
    }

    pub fn get(&self, id: u64) -> Result<Option<Item>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ITEMS)?;
        Ok(match table.get(id)? {
            Some(v) => Some(serde_json::from_slice(v.value())?),
            None => None,
        })
    }

    pub fn next_id(&self) -> Result<u64> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ITEMS)?;
        let next = table.last()?.map(|(k, _)| k.value() + 1).unwrap_or(1);
        Ok(next)
    }

    pub fn put(&self, item: &Item) -> Result<()> {
        let bytes = serde_json::to_vec(item)?;
        let txn = self.db.begin_write()?;
        txn.open_table(ITEMS)?.insert(item.id, bytes.as_slice())?;
        txn.commit()?;
        Ok(())
    }

    pub fn delete(&self, item: &Item) -> Result<()> {
        let txn = self.db.begin_write()?;
        txn.open_table(ITEMS)?.remove(item.id)?;
        {
            let mut emb = txn.open_table(EMBEDDINGS)?;
            let keys: Vec<String> = emb
                .iter()?
                .filter_map(|r| r.ok())
                .filter(|(k, _)| k.value().1 == item.id)
                .map(|(k, _)| k.value().0.to_string())
                .collect();
            for model in keys {
                emb.remove((model.as_str(), item.id))?;
            }
        }
        txn.commit()?;
        if let Some(p) = &item.image {
            if item.origin == Origin::Clipboard && p.starts_with(&self.images_dir) {
                let _ = std::fs::remove_file(p);
            }
        }
        Ok(())
    }

    pub fn put_embedding(&self, model: &str, id: u64, e: &Embedding) -> Result<()> {
        let bytes = e.to_bytes();
        let txn = self.db.begin_write()?;
        txn.open_table(EMBEDDINGS)?.insert((model, id), bytes.as_slice())?;
        txn.commit()?;
        Ok(())
    }

    /// All embeddings stored for `model`.
    pub fn embeddings(&self, model: &str) -> Result<Vec<(u64, Embedding)>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(EMBEDDINGS)?;
        let mut out = Vec::new();
        for row in table.range((model, 0u64)..=(model, u64::MAX))? {
            let (k, v) = row?;
            if let Some(e) = Embedding::from_bytes(v.value()) {
                out.push((k.value().1, e));
            }
        }
        Ok(out)
    }

    pub fn save_png(&self, id: u64, rgba: &image::RgbaImage) -> Result<PathBuf> {
        let path = self.images_dir.join(format!("{id}.png"));
        rgba.save(&path)?;
        Ok(path)
    }
}

/// FNV-1a: a stable hash for de-duplication (std's hasher may change between releases).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn looks_like_link(s: &str) -> bool {
    let t = s.trim();
    !t.contains(char::is_whitespace)
        && (t.starts_with("http://") || t.starts_with("https://") || t.starts_with("www."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u64, text: &str) -> Item {
        Item {
            id,
            kind: Kind::Text,
            origin: Origin::Clipboard,
            text: text.into(),
            image: None,
            width: 0,
            height: 0,
            ocr: None,
            pinned: false,
            created: now(),
            hash: fnv1a(text.as_bytes()),
            source: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn roundtrip_items_and_embeddings() {
        let dir = std::env::temp_dir().join(format!("cbm-store-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.redb"), &dir).unwrap();
        assert_eq!(store.next_id().unwrap(), 1);
        store.put(&item(1, "hello")).unwrap();
        store.put(&item(2, "world")).unwrap();
        assert_eq!(store.next_id().unwrap(), 3);
        store.put_embedding("m", 1, &Embedding::Dense(vec![1.0, 0.0])).unwrap();
        store.put_embedding("m", 2, &Embedding::Multi(vec![vec![0.0, 1.0]; 3])).unwrap();
        store.put_embedding("other", 2, &Embedding::Dense(vec![0.5])).unwrap();
        assert_eq!(store.embeddings("m").unwrap().len(), 2);
        store.delete(&item(2, "world")).unwrap();
        assert_eq!(store.all_items().unwrap().len(), 1);
        assert_eq!(store.embeddings("m").unwrap().len(), 1);
        assert!(store.embeddings("other").unwrap().is_empty());
        std::fs::remove_dir_all(dir).ok();
    }
}
