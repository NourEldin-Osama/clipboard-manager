//! Persistent storage for history items and their embeddings (redb, pure Rust).
//!
//! Images are written as PNG files next to the database; the database keeps
//! metadata, OCR text and one embedding per (model, item).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::embed::Embedding;

const ITEMS: TableDefinition<u64, &[u8]> = TableDefinition::new("items");
const EMBEDDINGS: TableDefinition<(&str, u64), &[u8]> = TableDefinition::new("embeddings");
/// The model keys that have rows in EMBEDDINGS, so deleting an item visits
/// one row per model instead of scanning every stored vector.
const MODELS: TableDefinition<&str, ()> = TableDefinition::new("models");
/// Facts about the file itself, such as `schema_version`.
const META: TableDefinition<&str, u64> = TableDefinition::new("meta");

/// The stored format. `migrate` brings an older file up to it, and a newer
/// file (from a later app version) is refused rather than misread.
/// 1: items and embeddings. 2: adds the models and meta tables.
const SCHEMA_VERSION: u64 = 2;

/// redb's page cache defaults to 1 GiB, on top of the history the app already
/// holds in memory; a clipboard history needs far less.
const CACHE_BYTES: usize = 64 * 1024 * 1024;

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
    /// Times it was copied back from the list ("Most used").
    #[serde(default)]
    pub uses: u32,
}

impl Item {
    /// A new item copied now, with nothing optional set yet.
    pub fn new(id: u64, kind: Kind, origin: Origin, text: Arc<str>) -> Self {
        Self {
            id,
            kind,
            origin,
            hash: fnv1a(text.as_bytes()),
            text,
            image: None,
            width: 0,
            height: 0,
            ocr: None,
            pinned: false,
            created: now(),
            source: None,
            tags: Vec::new(),
            uses: 0,
        }
    }

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
                        Some(p) => p
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        None => crate::i18n::trf(
                            "Image {w}×{h}",
                            &[("w", &self.width), ("h", &self.height)],
                        ),
                    }
                }
            },
            _ => &self.text,
        };
        let line = src
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("");
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
        let db = redb::Builder::new()
            .set_cache_size(CACHE_BYTES)
            .create(db_path)
            .context("opening database")?;
        let store = Self {
            db,
            images_dir: images_dir.to_path_buf(),
        };
        store.migrate()?;
        Ok(store)
    }

    /// Creates missing tables and brings an older file up to SCHEMA_VERSION,
    /// in one transaction: an interrupted migration leaves the old file as it was.
    fn migrate(&self) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            txn.open_table(ITEMS)?;
            let embeddings = txn.open_table(EMBEDDINGS)?;
            let mut models = txn.open_table(MODELS)?;
            let mut meta = txn.open_table(META)?;
            let version = meta.get("schema_version")?.map_or(1, |v| v.value());
            if version > SCHEMA_VERSION {
                bail!("the history was saved by a newer version of the app (format {version}); update the app");
            }
            if version < 2 {
                // One scan to list the models; put_embeddings keeps the list after this.
                let keys: std::collections::BTreeSet<String> = embeddings
                    .iter()?
                    .filter_map(|row| row.ok())
                    .map(|(k, _)| k.value().0.to_string())
                    .collect();
                for key in &keys {
                    models.insert(key.as_str(), ())?;
                }
            }
            meta.insert("schema_version", SCHEMA_VERSION)?;
        }
        txn.commit()?;
        Ok(())
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
        self.put_many(std::slice::from_ref(item))
    }

    /// Saves items in one transaction: one disk sync for a whole import batch
    /// instead of one per file.
    pub fn put_many(&self, items: &[Item]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(ITEMS)?;
            for item in items {
                table.insert(item.id, serde_json::to_vec(item)?.as_slice())?;
            }
        }
        txn.commit()?;
        Ok(())
    }

    /// Changes fields of a stored item in one transaction. The UI and the OCR
    /// worker both write items; with whole-item writes, one could undo the
    /// other's change (a pin lost to an OCR result). Returns the saved item, or
    /// `None` if it was deleted meanwhile, which then stays deleted.
    pub fn update(&self, id: u64, change: impl FnOnce(&mut Item)) -> Result<Option<Item>> {
        let txn = self.db.begin_write()?;
        let saved = {
            let mut table = txn.open_table(ITEMS)?;
            let stored: Option<Item> = table
                .get(id)?
                .map(|v| serde_json::from_slice(v.value()))
                .transpose()?;
            let Some(mut item) = stored else {
                return Ok(None);
            };
            change(&mut item);
            table.insert(id, serde_json::to_vec(&item)?.as_slice())?;
            item
        };
        txn.commit()?;
        Ok(Some(saved))
    }

    pub fn delete(&self, item: &Item) -> Result<()> {
        self.delete_many(std::slice::from_ref(item))
    }

    /// Removes items with their vectors under every model, in one transaction
    /// (one disk sync however many there are), then their image files.
    pub fn delete_many(&self, items: &[Item]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let txn = self.db.begin_write()?;
        {
            let models: Vec<String> = txn
                .open_table(MODELS)?
                .iter()?
                .filter_map(|row| row.ok())
                .map(|(k, _)| k.value().to_string())
                .collect();
            let mut table = txn.open_table(ITEMS)?;
            let mut vectors = txn.open_table(EMBEDDINGS)?;
            for item in items {
                table.remove(item.id)?;
                for model in &models {
                    vectors.remove((model.as_str(), item.id))?;
                }
            }
        }
        txn.commit()?;
        for item in items {
            if let Some(p) = &item.image {
                if item.origin == Origin::Clipboard && p.starts_with(&self.images_dir) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        Ok(())
    }

    pub fn put_embedding(&self, model: &str, id: u64, e: &Embedding) -> Result<()> {
        self.put_embeddings(model, &[(id, e)])
    }

    /// Saves vectors for one model in one transaction: one disk sync for a
    /// whole indexing batch instead of one per item.
    pub fn put_embeddings(&self, model: &str, rows: &[(u64, &Embedding)]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(EMBEDDINGS)?;
            for (id, e) in rows {
                table.insert((model, *id), e.to_bytes().as_slice())?;
            }
            txn.open_table(MODELS)?.insert(model, ())?;
        }
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
        Item::new(id, Kind::Text, Origin::Clipboard, text.into())
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cbm-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_old_file_is_migrated_and_deletes_reach_every_model() {
        let dir = temp_dir("migrate");
        let path = dir.join("old.redb");
        // A format-1 file: only the items and embeddings tables.
        {
            let db = Database::create(&path).unwrap();
            let txn = db.begin_write().unwrap();
            {
                let mut items = txn.open_table(ITEMS).unwrap();
                let bytes = serde_json::to_vec(&item(1, "a")).unwrap();
                items.insert(1, bytes.as_slice()).unwrap();
                let mut vectors = txn.open_table(EMBEDDINGS).unwrap();
                for model in ["m1", "m2"] {
                    let v = Embedding::Dense(vec![1.0]).to_bytes();
                    vectors.insert((model, 1u64), v.as_slice()).unwrap();
                }
            }
            txn.commit().unwrap();
        }
        let store = Store::open(&path, &dir).unwrap();
        assert_eq!(store.all_items().unwrap().len(), 1);
        store.delete(&item(1, "a")).unwrap();
        assert!(store.embeddings("m1").unwrap().is_empty());
        assert!(store.embeddings("m2").unwrap().is_empty());
        drop(store);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_file_from_a_newer_version_is_refused() {
        let dir = temp_dir("newer");
        let path = dir.join("t.redb");
        drop(Store::open(&path, &dir).unwrap());
        {
            let db = Database::create(&path).unwrap();
            let txn = db.begin_write().unwrap();
            txn.open_table(META)
                .unwrap()
                .insert("schema_version", 99)
                .unwrap();
            txn.commit().unwrap();
        }
        let refused = Store::open(&path, &dir)
            .err()
            .expect("a newer format must be refused");
        assert!(format!("{refused:#}").contains("newer version"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn batches_are_saved_and_deleted_together() {
        let dir = temp_dir("batch");
        let store = Store::open(&dir.join("t.redb"), &dir).unwrap();
        let items = [item(1, "one"), item(2, "two"), item(3, "three")];
        store.put_many(&items).unwrap();
        assert_eq!(store.all_items().unwrap().len(), 3);
        let v = Embedding::Dense(vec![0.5, 0.5]);
        store
            .put_embeddings("m", &[(1, &v), (2, &v), (3, &v)])
            .unwrap();
        assert_eq!(store.embeddings("m").unwrap().len(), 3);
        store.delete_many(&items[..2]).unwrap();
        assert_eq!(store.all_items().unwrap().len(), 1);
        assert_eq!(store.embeddings("m").unwrap().len(), 1);
        drop(store);
        std::fs::remove_dir_all(dir).ok();
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
        store
            .put_embedding("m", 1, &Embedding::Dense(vec![1.0, 0.0]))
            .unwrap();
        store
            .put_embedding("m", 2, &Embedding::Multi(vec![vec![0.0, 1.0]; 3]))
            .unwrap();
        store
            .put_embedding("other", 2, &Embedding::Dense(vec![0.5]))
            .unwrap();
        assert_eq!(store.embeddings("m").unwrap().len(), 2);
        store.delete(&item(2, "world")).unwrap();
        assert_eq!(store.all_items().unwrap().len(), 1);
        assert_eq!(store.embeddings("m").unwrap().len(), 1);
        assert!(store.embeddings("other").unwrap().is_empty());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn updates_change_only_their_own_fields() {
        let dir = std::env::temp_dir().join(format!("cbm-update-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.redb"), &dir).unwrap();
        store.put(&item(1, "screenshot")).unwrap();
        // The UI pins while OCR runs; the OCR result then saves only its text.
        store.update(1, |it| it.pinned = true).unwrap();
        store
            .update(1, |it| it.ocr = Some("found text".into()))
            .unwrap();
        let it = store.get(1).unwrap().unwrap();
        assert!(it.pinned);
        assert_eq!(it.ocr.as_deref(), Some("found text"));
        // An item deleted meanwhile is not brought back.
        assert!(store.update(2, |it| it.pinned = true).unwrap().is_none());
        assert!(store.get(2).unwrap().is_none());
        drop(store);
        std::fs::remove_dir_all(dir).ok();
    }
}
