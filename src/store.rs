//! Persistent storage for history items and their embeddings (redb, pure Rust).
//!
//! Images are written as PNG files next to the database; the database keeps
//! metadata, OCR text and one embedding per (model, item).
//!
//! On Windows the items and the copied images' files are encrypted with the
//! history's key (see `vault`); the embeddings are not.

use std::borrow::Cow;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::embed::Embedding;
use crate::i18n::t;
use crate::vault::{self, Vault};

const ITEMS: TableDefinition<u64, &[u8]> = TableDefinition::new("items");
const EMBEDDINGS: TableDefinition<(&str, u64), &[u8]> = TableDefinition::new("embeddings");
/// The model keys that have rows in EMBEDDINGS, so deleting an item visits
/// one row per model instead of scanning every stored vector.
const MODELS: TableDefinition<&str, ()> = TableDefinition::new("models");
/// Facts about the file itself, such as `schema_version`.
const META: TableDefinition<&str, u64> = TableDefinition::new("meta");
/// The history's key, sealed for the Windows account (`HISTORY_KEY`).
const SECRETS: TableDefinition<&str, &[u8]> = TableDefinition::new("secrets");
/// The formatted versions of text items (`Formats`), read only to paste.
const FORMATS: TableDefinition<u64, &[u8]> = TableDefinition::new("formats");
const HISTORY_KEY: &str = "history_key";

/// The stored format. A file from a later app version is refused rather
/// than misread. Where the secrets table holds a key, item rows and the
/// copied images' files are encrypted, and those images' paths are
/// relative to the images folder.
const SCHEMA_VERSION: u64 = 3;

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
    /// Its name in the interface language.
    pub fn label(self) -> String {
        match self {
            Kind::Text => t!("kind-text"),
            Kind::Link => t!("kind-link"),
            Kind::Image => t!("kind-image"),
            Kind::Files => t!("kind-files"),
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
    /// The program it was copied from, as in `chrome.exe`, when known.
    #[serde(default)]
    pub app: Option<String>,
    /// It has formatted versions in the store (`Store::formats`).
    #[serde(default)]
    pub formatted: bool,
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
            app: None,
            formatted: false,
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
                        None => t!("item-image-size", width = self.width, height = self.height),
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

/// The formatted versions of a text copy, as the clipboard had them, so a
/// paste can give them back unchanged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Formats {
    /// The clipboard's "HTML Format" data, header included.
    pub html: Option<Vec<u8>>,
    /// The clipboard's "Rich Text Format" data.
    pub rtf: Option<Vec<u8>>,
}

impl Formats {
    /// Each version as a presence byte, a length and the bytes.
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for part in [&self.html, &self.rtf] {
            match part {
                Some(bytes) => {
                    out.push(1);
                    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    out.extend_from_slice(bytes);
                }
                None => out.push(0),
            }
        }
        out
    }

    fn decode(mut bytes: &[u8]) -> Result<Self> {
        let mut part = || -> Result<Option<Vec<u8>>> {
            let (&present, rest) = bytes.split_first().context("formatting is cut short")?;
            bytes = rest;
            if present == 0 {
                return Ok(None);
            }
            let length = bytes.get(..4).context("formatting is cut short")?;
            let length = u32::from_le_bytes(length.try_into()?) as usize;
            let data = bytes
                .get(4..4 + length)
                .context("formatting is cut short")?;
            bytes = &bytes[4 + length..];
            Ok(Some(data.to_vec()))
        };
        let html = part()?;
        let rtf = part()?;
        Ok(Self { html, rtf })
    }
}

pub struct Store {
    db: Database,
    vault: Option<Arc<Vault>>,
    pub images: ImageFiles,
    /// Where the history went when this Windows account couldn't open it; a
    /// new, empty one took its place, and the app says so once.
    pub moved_aside: Option<PathBuf>,
}

impl Store {
    /// Opens the history, creating it if needed. On Windows it is encrypted:
    /// one whose key this account can't open, or a plain one an older version
    /// wrote, is moved aside for a new, empty one.
    pub fn open(db_path: &Path, images_dir: &Path) -> Result<Self> {
        let mut db = open_database(db_path)?;
        let mut moved_aside = None;
        let vault = match stored_key(&db)? {
            Some(sealed) => match Vault::open(&sealed) {
                Ok(vault) => Some(vault),
                Err(e) => {
                    log::error!("the history's key can't be opened on this account: {e:#}");
                    drop(db);
                    moved_aside = Some(move_aside(db_path, images_dir)?);
                    db = open_database(db_path)?;
                    new_key(&db)?
                }
            },
            // Without DPAPI a history is always plain; on Windows a plain one
            // with items was written by an older version, which isn't read.
            None if cfg!(windows) && has_items(&db)? => {
                log::error!("the history was written by an older version; starting a new one");
                drop(db);
                moved_aside = Some(move_aside(db_path, images_dir)?);
                db = open_database(db_path)?;
                new_key(&db)?
            }
            None => new_key(&db)?,
        };
        let vault = vault.map(Arc::new);
        Ok(Self {
            db,
            images: ImageFiles {
                dir: images_dir.to_path_buf(),
                vault: vault.clone(),
            },
            vault,
            moved_aside,
        })
    }

    /// Whether the items and images are encrypted.
    pub fn is_encrypted(&self) -> bool {
        self.vault.is_some()
    }

    /// The stored form of an item: encrypted when there is a key.
    fn encode(&self, item: &Item) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(item)?;
        match &self.vault {
            Some(vault) => vault.seal(&vault::item_context(item.id), &json),
            None => Ok(json),
        }
    }

    fn decode(&self, id: u64, stored: &[u8]) -> Result<Item> {
        let json = if vault::is_sealed(stored) {
            let vault = self
                .vault
                .as_ref()
                .context("an encrypted item, but no key")?;
            Cow::Owned(vault.unseal(&vault::item_context(id), stored)?)
        } else {
            Cow::Borrowed(stored)
        };
        Ok(serde_json::from_slice(&json)?)
    }

    pub fn all_items(&self) -> Result<Vec<Item>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ITEMS)?;
        let mut out = Vec::new();
        for row in table.iter()? {
            let (k, v) = row?;
            match self.decode(k.value(), v.value()) {
                Ok(item) => out.push(item),
                Err(e) => log::warn!("skipping unreadable item {}: {e:#}", k.value()),
            }
        }
        Ok(out)
    }

    pub fn get(&self, id: u64) -> Result<Option<Item>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ITEMS)?;
        Ok(match table.get(id)? {
            Some(v) => Some(self.decode(id, v.value())?),
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
                table.insert(item.id, self.encode(item)?.as_slice())?;
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
                .map(|v| self.decode(id, v.value()))
                .transpose()?;
            let Some(mut item) = stored else {
                return Ok(None);
            };
            change(&mut item);
            table.insert(id, self.encode(&item)?.as_slice())?;
            item
        };
        txn.commit()?;
        Ok(Some(saved))
    }

    /// Saves the formatted versions of a text item.
    pub fn put_formats(&self, id: u64, formats: &Formats) -> Result<()> {
        let bytes = formats.encode();
        let stored = match &self.vault {
            Some(vault) => vault.seal(&vault::formats_context(id), &bytes)?,
            None => bytes,
        };
        let txn = self.db.begin_write()?;
        txn.open_table(FORMATS)?.insert(id, stored.as_slice())?;
        txn.commit()?;
        Ok(())
    }

    /// The formatted versions of a text item, if it has any.
    pub fn formats(&self, id: u64) -> Result<Option<Formats>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(FORMATS)?;
        let Some(stored) = table.get(id)? else {
            return Ok(None);
        };
        let bytes = match &self.vault {
            Some(vault) => vault.unseal(&vault::formats_context(id), stored.value())?,
            None => stored.value().to_vec(),
        };
        Formats::decode(&bytes).map(Some)
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
            let mut formats = txn.open_table(FORMATS)?;
            for item in items {
                table.remove(item.id)?;
                formats.remove(item.id)?;
                for model in &models {
                    vectors.remove((model.as_str(), item.id))?;
                }
            }
        }
        txn.commit()?;
        for item in items {
            if let Some(p) = &item.image {
                if item.origin == Origin::Clipboard {
                    self.images.remove(p);
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
}

/// The files of copied images, encrypted when the history has a key. Cheap
/// to clone, for the threads that read and write them.
#[derive(Clone)]
pub struct ImageFiles {
    dir: PathBuf,
    vault: Option<Arc<Vault>>,
}

impl ImageFiles {
    /// The file a stored image path names. Encrypted images are stored
    /// relative to the images folder; imported ones keep the path of the
    /// user's own file.
    pub fn path(&self, stored: &Path) -> PathBuf {
        if stored.is_absolute() {
            stored.to_path_buf()
        } else {
            self.dir.join(stored)
        }
    }

    /// Saves a copied image. Returns the path to keep in its item.
    pub fn save(&self, id: u64, rgba: &image::RgbaImage) -> Result<PathBuf> {
        let Some(vault) = &self.vault else {
            let path = self.dir.join(format!("{id}.png"));
            rgba.save(&path)?;
            return Ok(path);
        };
        let mut png = Vec::new();
        rgba.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)?;
        let name = PathBuf::from(format!("{id}.png.enc"));
        std::fs::write(
            self.dir.join(&name),
            vault.seal(&vault::image_context(id), &png)?,
        )?;
        Ok(name)
    }

    /// An image of the history, decoded.
    pub fn open(&self, id: u64, stored: &Path) -> Result<image::DynamicImage> {
        let path = self.path(stored);
        if path.extension().is_none_or(|e| e != "enc") {
            return Ok(image::open(&path)?);
        }
        let vault = self
            .vault
            .as_ref()
            .context("an encrypted image, but no key")?;
        let sealed = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let png = vault.unseal(&vault::image_context(id), &sealed)?;
        Ok(image::load_from_memory(&png)?)
    }

    /// Deletes a copied image's file. Files outside the images folder are
    /// the user's own, and stay.
    fn remove(&self, stored: &Path) {
        let path = self.path(stored);
        if !path.starts_with(&self.dir) {
            return;
        }
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                log::warn!("deleting {}: {e}", path.display());
            }
            _ => {}
        }
    }
}

fn open_database(path: &Path) -> Result<Database> {
    let db = redb::Builder::new()
        .set_cache_size(CACHE_BYTES)
        .create(path)
        .context("opening database")?;
    create_tables(&db)?;
    Ok(db)
}

/// Creates missing tables and stamps the file with SCHEMA_VERSION; a file
/// from a later version is refused.
fn create_tables(db: &Database) -> Result<()> {
    let txn = db.begin_write()?;
    {
        txn.open_table(ITEMS)?;
        txn.open_table(SECRETS)?;
        txn.open_table(FORMATS)?;
        txn.open_table(EMBEDDINGS)?;
        txn.open_table(MODELS)?;
        let mut meta = txn.open_table(META)?;
        let version = meta.get("schema_version")?.map_or(1, |v| v.value());
        if version > SCHEMA_VERSION {
            bail!("the history was saved by a newer version of the app (format {version}); update the app");
        }
        meta.insert("schema_version", SCHEMA_VERSION)?;
    }
    txn.commit()?;
    Ok(())
}

/// The history's key as stored: sealed for the Windows account.
fn stored_key(db: &Database) -> Result<Option<Vec<u8>>> {
    let txn = db.begin_read()?;
    let table = txn.open_table(SECRETS)?;
    Ok(table.get(HISTORY_KEY)?.map(|v| v.value().to_vec()))
}

fn has_items(db: &Database) -> Result<bool> {
    let txn = db.begin_read()?;
    Ok(!txn.open_table(ITEMS)?.is_empty()?)
}

/// Gives an empty history a new key. `None` on systems without DPAPI.
fn new_key(db: &Database) -> Result<Option<Vault>> {
    let Some((vault, sealed)) = Vault::create()? else {
        return Ok(None);
    };
    let txn = db.begin_write()?;
    txn.open_table(SECRETS)?
        .insert(HISTORY_KEY, sealed.as_slice())?;
    txn.commit()?;
    Ok(Some(vault))
}

/// Moves a history this account can't open out of the way, whole: it opens
/// again on the account and PC that made it. Returns where it went.
fn move_aside(db_path: &Path, images_dir: &Path) -> Result<PathBuf> {
    let stamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S");
    let aside = db_path.with_file_name(format!("history-unreadable-{stamp}.redb"));
    std::fs::rename(db_path, &aside)
        .with_context(|| format!("moving the unreadable history to {}", aside.display()))?;
    if images_dir.exists() {
        let images_aside = images_dir.with_file_name(format!("images-unreadable-{stamp}"));
        std::fs::rename(images_dir, &images_aside)
            .with_context(|| format!("moving its images to {}", images_aside.display()))?;
        std::fs::create_dir_all(images_dir)?;
    }
    Ok(aside)
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

    fn file_contains(path: &Path, text: &str) -> bool {
        let bytes = std::fs::read(path).unwrap();
        bytes.windows(text.len()).any(|w| w == text.as_bytes())
    }

    #[test]
    fn formats_keep_each_version_or_its_absence() {
        for formats in [
            Formats::default(),
            Formats {
                html: Some(b"<b>hi</b>".to_vec()),
                rtf: None,
            },
            Formats {
                html: None,
                rtf: Some(b"{\\rtf1 hi}".to_vec()),
            },
            Formats {
                html: Some(Vec::new()),
                rtf: Some(b"x".to_vec()),
            },
        ] {
            assert_eq!(Formats::decode(&formats.encode()).unwrap(), formats);
        }
        assert!(
            Formats::decode(&[1, 9, 0, 0, 0, b'x']).is_err(),
            "cut short"
        );
    }

    #[test]
    fn formatting_is_stored_apart_and_deleted_with_its_item() {
        let dir = temp_dir("formats");
        let path = dir.join("history.redb");
        let store = Store::open(&path, &dir).unwrap();
        let mut it = item(1, "bold words");
        it.formatted = true;
        store.put(&it).unwrap();
        let formats = Formats {
            html: Some(b"<b>bold words in a formatted copy</b>".to_vec()),
            rtf: Some(b"{\\rtf1 {\\b bold words in a formatted copy}}".to_vec()),
        };
        store.put_formats(1, &formats).unwrap();
        assert_eq!(store.formats(1).unwrap(), Some(formats));
        assert_eq!(store.formats(2).unwrap(), None);
        if store.is_encrypted() {
            drop(store);
            assert!(!file_contains(&path, "formatted copy"));
            let store = Store::open(&path, &dir).unwrap();
            store.delete(&it).unwrap();
            assert_eq!(store.formats(1).unwrap(), None);
        } else {
            store.delete(&it).unwrap();
            assert_eq!(store.formats(1).unwrap(), None);
        }
        std::fs::remove_dir_all(dir).ok();
    }

    #[cfg(windows)]
    #[test]
    fn a_new_history_is_encrypted() {
        let dir = temp_dir("encrypted");
        let path = dir.join("history.redb");
        let store = Store::open(&path, &dir).unwrap();
        assert!(store.is_encrypted());
        store.put(&item(1, "a secret phrase")).unwrap();
        assert_eq!(&*store.get(1).unwrap().unwrap().text, "a secret phrase");
        let image = image::RgbaImage::from_pixel(4, 3, image::Rgba([10, 20, 30, 255]));
        let stored = store.images.save(5, &image).unwrap();
        assert_eq!(stored, PathBuf::from("5.png.enc"));
        assert!(!std::fs::read(dir.join(&stored))
            .unwrap()
            .starts_with(b"\x89PNG"));
        assert_eq!(store.images.open(5, &stored).unwrap().into_rgba8(), image);
        drop(store);
        assert!(!file_contains(&path, "a secret phrase"));
        let store = Store::open(&path, &dir).unwrap();
        assert_eq!(&*store.get(1).unwrap().unwrap().text, "a secret phrase");
        drop(store);
        std::fs::remove_dir_all(dir).ok();
    }

    #[cfg(windows)]
    #[test]
    fn a_plain_history_from_an_older_version_is_moved_aside() {
        let dir = temp_dir("plain");
        let path = dir.join("history.redb");
        let images_dir = dir.join("images");
        std::fs::create_dir_all(&images_dir).unwrap();
        // Plain rows and no key, as an older version wrote them.
        {
            let db = Database::create(&path).unwrap();
            let txn = db.begin_write().unwrap();
            {
                let mut items = txn.open_table(ITEMS).unwrap();
                let it = item(1, "plain text");
                items
                    .insert(it.id, serde_json::to_vec(&it).unwrap().as_slice())
                    .unwrap();
            }
            txn.commit().unwrap();
        }
        let store = Store::open(&path, &images_dir).unwrap();
        let aside = store
            .moved_aside
            .clone()
            .expect("the old history is moved aside");
        assert!(file_contains(&aside, "plain text"), "kept as it was");
        assert!(store.is_encrypted());
        assert!(store.all_items().unwrap().is_empty());
        drop(store);
        std::fs::remove_dir_all(dir).ok();
    }

    #[cfg(windows)]
    #[test]
    fn a_history_this_account_cant_open_is_moved_aside() {
        let dir = temp_dir("aside");
        let path = dir.join("history.redb");
        let images_dir = dir.join("images");
        std::fs::create_dir_all(&images_dir).unwrap();
        let store = Store::open(&path, &images_dir).unwrap();
        store.put(&item(1, "from another account")).unwrap();
        drop(store);
        // What another account's key looks like here: one DPAPI can't open.
        {
            let db = Database::create(&path).unwrap();
            let txn = db.begin_write().unwrap();
            txn.open_table(SECRETS)
                .unwrap()
                .insert(HISTORY_KEY, b"another account's key".as_slice())
                .unwrap();
            txn.commit().unwrap();
        }
        let store = Store::open(&path, &images_dir).unwrap();
        let aside = store
            .moved_aside
            .clone()
            .expect("the old history is moved aside");
        assert!(aside.exists());
        assert!(store.is_encrypted());
        assert!(store.all_items().unwrap().is_empty());
        assert!(images_dir.is_dir());
        drop(store);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn deletes_reach_every_model() {
        let dir = temp_dir("models");
        let store = Store::open(&dir.join("t.redb"), &dir).unwrap();
        let it = item(1, "a");
        store.put(&it).unwrap();
        for model in ["m1", "m2"] {
            store
                .put_embedding(model, 1, &Embedding::Dense(vec![1.0]))
                .unwrap();
        }
        store.delete(&it).unwrap();
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
