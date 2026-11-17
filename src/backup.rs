//! A backup of the history in one file, locked with a password, and putting
//! one back.
//!
//! The file starts with a header in the open: a magic string, the format's
//! version, the number of PBKDF2 rounds and a random salt. PBKDF2-HMAC-SHA256
//! makes a 32-byte key from the password and the salt. Records follow, each
//! sealed with AES-256-GCM and a random nonce of its own: one per item, then
//! an end record with the number of items. Each record is bound to the header
//! and to its number, so no record can be changed, swapped, dropped or taken
//! from another backup, and a file cut short lacks its end record. A wrong
//! password and a changed byte look the same: a record that doesn't open.
//!
//! An item record holds the item as the history keeps it, its formatted
//! versions and the picture of a copied image, so a backup needs neither the
//! history's key nor this PC. Settings, API keys and search vectors are not
//! in it. An imported file stays where it is: its item keeps the path.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};

use crate::store::{Formats, Item, Kind, Origin, Store};

/// What a backup starts with.
const MAGIC: &[u8; 24] = b"CLIPBOARD-MANAGER-BACKUP";
/// The format. A later one gets a higher number, which this version refuses.
const VERSION: u32 = 1;
/// The PBKDF2 rounds of a new backup: OWASP's figure for HMAC-SHA256.
const ROUNDS: u32 = 600_000;
/// A header asking for more rounds than this is damaged, not just slow.
const MOST_ROUNDS: u32 = 10_000_000;
const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;
const TAG_LEN: usize = 16;
/// The magic string, the version, the rounds and the salt.
const HEADER_LEN: usize = MAGIC.len() + 4 + 4 + SALT_LEN;
/// A record longer than this is a damaged file.
const MOST_RECORD_BYTES: usize = 1 << 30;
/// What a record holds, its first byte once opened.
const ITEM_RECORD: u8 = 1;
const END_RECORD: u8 = 2;
/// A restore hands its items over at this many, or this many bytes of text,
/// so the window adds them a batch at a time.
const BATCH_ITEMS: usize = 200;
const BATCH_BYTES: usize = 4 << 20;

/// Why a backup can't be read, or restored.
#[derive(Debug)]
pub enum Problem {
    /// The file isn't a backup of this app.
    NotBackup,
    /// A later version of the app made it.
    Newer,
    /// The password is wrong, or the file was changed, damaged or cut short.
    Refused,
    /// Reading the file, or saving an item, failed.
    Failed(anyhow::Error),
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::NotBackup => f.write_str("the file is not a backup of this app"),
            Problem::Newer => f.write_str("a newer version of the app made the backup"),
            Problem::Refused => {
                f.write_str("the password is wrong, or the file was changed or damaged")
            }
            Problem::Failed(e) => write!(f, "{e:#}"),
        }
    }
}

impl From<std::io::Error> for Problem {
    fn from(e: std::io::Error) -> Self {
        Problem::Failed(e.into())
    }
}

impl From<anyhow::Error> for Problem {
    fn from(e: anyhow::Error) -> Self {
        Problem::Failed(e)
    }
}

/// What a restore did.
#[derive(Debug, PartialEq)]
pub struct Restored {
    pub added: usize,
    /// Items whose kind and content the history had already.
    pub skipped: usize,
}

/// How far a restore is.
pub enum Step {
    /// The password opened the backup.
    Opened,
    /// The whole file was checked, and it holds this many items.
    Checked(usize),
    /// This many items were added or skipped so far.
    Done(usize),
}

/// Writes `items` to a new backup at `path`, locked with `password`. It is
/// written beside `path` first and takes its place once whole, so a backup
/// already there stays as it was if this one fails. `step` hears how many
/// items are written so far. Returns how many items it holds.
pub fn export(
    store: &Store,
    items: &[Arc<Item>],
    path: &Path,
    password: &str,
    step: impl FnMut(usize),
) -> Result<usize> {
    export_with_rounds(store, items, path, password, ROUNDS, step)
}

fn export_with_rounds(
    store: &Store,
    items: &[Arc<Item>],
    path: &Path,
    password: &str,
    rounds: u32,
    mut step: impl FnMut(usize),
) -> Result<usize> {
    let mut name = path.as_os_str().to_owned();
    name.push(".part");
    let part = PathBuf::from(name);
    let written = write_file(store, items, &part, password, rounds, &mut step).and_then(|()| {
        std::fs::rename(&part, path).with_context(|| format!("saving {}", path.display()))
    });
    if let Err(e) = written {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    Ok(items.len())
}

/// Writes the backup of `items` to `file`, and makes sure it is on the disk.
fn write_file(
    store: &Store,
    items: &[Arc<Item>],
    file: &Path,
    password: &str,
    rounds: u32,
    step: &mut impl FnMut(usize),
) -> Result<()> {
    let out = File::create(file).with_context(|| format!("creating {}", file.display()))?;
    let mut writer = Writer::new(BufWriter::new(out), password, rounds)?;
    for (index, item) in items.iter().enumerate() {
        writer.add(&entry_of(store, item))?;
        step(index + 1);
    }
    let out = writer.finish()?.into_inner().map_err(|e| e.into_error())?;
    out.sync_all()?;
    Ok(())
}

/// Checks that `path` is a backup this version can read, before its
/// password is asked.
pub fn check_file(path: &Path) -> Result<(), Problem> {
    Header::read(&mut File::open(path)?).map(|_| ())
}

/// Puts back the items of the backup at `path`, locked with `password`. The
/// whole file is checked first, so a wrong password or a changed or cut
/// file adds nothing. Then each item whose kind and content are not in
/// `known` (the history's) gets an id from `new_id`, its formatted versions
/// and picture are saved in `store`, and `add` takes the items in batches,
/// to add to the history. `step` hears how far it is.
pub fn restore(
    path: &Path,
    password: &str,
    store: &Store,
    known: &HashSet<(Kind, u64)>,
    mut new_id: impl FnMut() -> u64,
    mut step: impl FnMut(Step),
    mut add: impl FnMut(Vec<Item>),
) -> Result<Restored, Problem> {
    let mut reader = Reader::open(BufReader::new(File::open(path)?), password)?;
    let mut entry = reader.next_entry()?;
    step(Step::Opened);
    let mut total = 0;
    while entry.is_some() {
        total += 1;
        entry = reader.next_entry()?;
    }
    step(Step::Checked(total));
    // Read again with the same key, which took long to make.
    let mut input = BufReader::new(File::open(path)?);
    if Header::read(&mut input)?.bytes != reader.header.bytes {
        return Err(Problem::Refused);
    }
    let mut reader = Reader::with_key(input, reader.header, reader.key);
    let mut restored = Restored {
        added: 0,
        skipped: 0,
    };
    let mut batch = Vec::new();
    let mut batch_bytes = 0;
    while let Some(Entry {
        mut item,
        formats,
        image,
    }) = reader.next_entry()?
    {
        if known.contains(&(item.kind, item.hash)) {
            restored.skipped += 1;
        } else {
            item.id = new_id();
            item.formatted = false;
            if let Some(formats) = formats {
                store.put_formats(item.id, &formats)?;
                item.formatted = true;
            }
            match image {
                Some(png) => item.image = Some(store.images.save_png(item.id, &png)?),
                // An imported picture is the user's own file, still where it was.
                None if item.origin == Origin::Imported => {}
                None => item.image = None,
            }
            batch_bytes += item.text.len();
            batch.push(item);
            restored.added += 1;
        }
        step(Step::Done(restored.added + restored.skipped));
        if batch.len() >= BATCH_ITEMS || batch_bytes >= BATCH_BYTES {
            add(std::mem::take(&mut batch));
            batch_bytes = 0;
        }
    }
    if !batch.is_empty() {
        add(batch);
    }
    Ok(restored)
}

/// One item of a backup.
struct Entry {
    item: Item,
    formats: Option<Formats>,
    /// The PNG file of a copied picture.
    image: Option<Vec<u8>>,
}

/// `item` as a backup keeps it, with its formatted versions and the file of
/// its copied picture. A part that can't be read is left out, and logged.
fn entry_of(store: &Store, item: &Item) -> Entry {
    let formats = if item.formatted {
        store.formats(item.id).unwrap_or_else(|e| {
            log::warn!(
                "the backup leaves out the formatting of item {}: {e:#}",
                item.id
            );
            None
        })
    } else {
        None
    };
    let image = match (&item.image, item.origin) {
        (Some(stored), Origin::Clipboard) => match store.images.read(item.id, stored) {
            Ok(png) => Some(png),
            Err(e) => {
                log::warn!(
                    "the backup leaves out the picture of item {}: {e:#}",
                    item.id
                );
                None
            }
        },
        _ => None,
    };
    Entry {
        item: item.clone(),
        formats,
        image,
    }
}

/// A backup's header: what its key is made with, and what each of its
/// records is bound to.
#[derive(Clone)]
struct Header {
    bytes: [u8; HEADER_LEN],
}

impl Header {
    fn new(rounds: u32, salt: &[u8; SALT_LEN]) -> Self {
        let mut bytes = [0u8; HEADER_LEN];
        let (magic, rest) = bytes.split_at_mut(MAGIC.len());
        magic.copy_from_slice(MAGIC);
        rest[..4].copy_from_slice(&VERSION.to_le_bytes());
        rest[4..8].copy_from_slice(&rounds.to_le_bytes());
        rest[8..].copy_from_slice(salt);
        Self { bytes }
    }

    /// Reads a header and checks what it can without the password.
    fn read(input: &mut impl Read) -> Result<Self, Problem> {
        let mut bytes = [0u8; HEADER_LEN];
        let read = read_up_to(input, &mut bytes)?;
        if read < MAGIC.len() || bytes[..MAGIC.len()] != MAGIC[..] {
            return Err(Problem::NotBackup);
        }
        let header = Self { bytes };
        if read < MAGIC.len() + 4 {
            return Err(Problem::Refused);
        }
        match header.number(MAGIC.len()) {
            VERSION => {}
            later if later > VERSION => return Err(Problem::Newer),
            _ => return Err(Problem::Refused),
        }
        if read < HEADER_LEN || !(1..=MOST_ROUNDS).contains(&header.rounds()) {
            return Err(Problem::Refused);
        }
        Ok(header)
    }

    fn number(&self, at: usize) -> u32 {
        let mut number = [0u8; 4];
        number.copy_from_slice(&self.bytes[at..at + 4]);
        u32::from_le_bytes(number)
    }

    fn rounds(&self) -> u32 {
        self.number(MAGIC.len() + 4)
    }

    fn salt(&self) -> &[u8] {
        &self.bytes[HEADER_LEN - SALT_LEN..]
    }

    /// The key `password` makes with this header's salt and rounds: slow
    /// on purpose, so guessing passwords is too.
    fn key(&self, password: &str) -> Result<LessSafeKey> {
        let rounds = NonZeroU32::new(self.rounds()).context("a header with no rounds")?;
        let mut raw = [0u8; KEY_LEN];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            rounds,
            self.salt(),
            password.as_bytes(),
            &mut raw,
        );
        let key = UnboundKey::new(&AES_256_GCM, &raw)
            .map_err(|_| anyhow!("the key has the wrong length"))?;
        Ok(LessSafeKey::new(key))
    }

    /// What the record numbered `number` is bound to: this header and its
    /// place in the file.
    fn bound_to(&self, number: u64) -> [u8; HEADER_LEN + 8] {
        let mut bound = [0u8; HEADER_LEN + 8];
        bound[..HEADER_LEN].copy_from_slice(&self.bytes);
        bound[HEADER_LEN..].copy_from_slice(&number.to_le_bytes());
        bound
    }
}

/// Writes a backup record by record, so only one item's picture is in
/// memory at a time. A record is its length, its nonce, and what it holds,
/// sealed.
struct Writer<W: Write> {
    out: W,
    header: Header,
    key: LessSafeKey,
    random: SystemRandom,
    /// The next record's number.
    number: u64,
    items: u64,
}

impl<W: Write> Writer<W> {
    /// Starts a backup locked with `password`, its key made with `rounds`.
    fn new(mut out: W, password: &str, rounds: u32) -> Result<Self> {
        let random = SystemRandom::new();
        let mut salt = [0u8; SALT_LEN];
        random
            .fill(&mut salt)
            .map_err(|_| anyhow!("no random numbers for a salt"))?;
        let header = Header::new(rounds, &salt);
        let key = header.key(password)?;
        out.write_all(&header.bytes)?;
        Ok(Self {
            out,
            header,
            key,
            random,
            number: 0,
            items: 0,
        })
    }

    /// Adds an item: the item as JSON, then its HTML, its RTF and its
    /// picture, each of which may be missing.
    fn add(&mut self, entry: &Entry) -> Result<()> {
        let mut record = vec![ITEM_RECORD];
        put_part(&mut record, Some(&serde_json::to_vec(&entry.item)?));
        let formats = entry.formats.as_ref();
        put_part(&mut record, formats.and_then(|f| f.html.as_deref()));
        put_part(&mut record, formats.and_then(|f| f.rtf.as_deref()));
        put_part(&mut record, entry.image.as_deref());
        self.seal(record)?;
        self.items += 1;
        Ok(())
    }

    /// Ends the backup with the record that counts its items, and hands
    /// the output back.
    fn finish(mut self) -> Result<W> {
        let mut record = vec![END_RECORD];
        record.extend_from_slice(&self.items.to_le_bytes());
        self.seal(record)?;
        self.out.flush()?;
        Ok(self.out)
    }

    fn seal(&mut self, mut record: Vec<u8>) -> Result<()> {
        if record.len() + TAG_LEN > MOST_RECORD_BYTES {
            bail!("an item is too big for a backup");
        }
        let mut nonce = [0u8; NONCE_LEN];
        self.random
            .fill(&mut nonce)
            .map_err(|_| anyhow!("no random numbers for a nonce"))?;
        self.key
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.header.bound_to(self.number)),
                &mut record,
            )
            .map_err(|_| anyhow!("encrypting failed"))?;
        let length = u32::try_from(record.len()).context("a record too long")?;
        self.out.write_all(&length.to_le_bytes())?;
        self.out.write_all(&nonce)?;
        self.out.write_all(&record)?;
        self.number += 1;
        Ok(())
    }
}

/// Reads a backup record by record.
struct Reader<R: Read> {
    input: R,
    header: Header,
    key: LessSafeKey,
    /// The next record's number.
    number: u64,
    items: u64,
    ended: bool,
}

impl<R: Read> Reader<R> {
    /// Opens a backup with `password`. Slow on purpose: its key takes many
    /// rounds to make.
    fn open(mut input: R, password: &str) -> Result<Self, Problem> {
        let header = Header::read(&mut input)?;
        let key = header.key(password)?;
        Ok(Self::with_key(input, header, key))
    }

    /// Reads the records after `header`, with the key made for it.
    fn with_key(input: R, header: Header, key: LessSafeKey) -> Self {
        Self {
            input,
            header,
            key,
            number: 0,
            items: 0,
            ended: false,
        }
    }

    /// The next item; `None` once the end record is read and the file ends
    /// there.
    fn next_entry(&mut self) -> Result<Option<Entry>, Problem> {
        if self.ended {
            return Ok(None);
        }
        let record = self.open_record()?;
        match record.split_first() {
            Some((&ITEM_RECORD, parts)) => {
                self.items += 1;
                // Sealed by this app: parts that don't fit are a bug, not a
                // damaged file.
                decode_entry(parts).map(Some).map_err(Problem::Failed)
            }
            Some((&END_RECORD, count)) => {
                if count != self.items.to_le_bytes() {
                    return Err(Problem::Refused);
                }
                // Nothing may follow it.
                let mut rest = Vec::new();
                (&mut self.input).take(1).read_to_end(&mut rest)?;
                if !rest.is_empty() {
                    return Err(Problem::Refused);
                }
                self.ended = true;
                Ok(None)
            }
            _ => Err(Problem::Refused),
        }
    }

    /// The next record, opened. A file that ends first was cut short.
    fn open_record(&mut self) -> Result<Vec<u8>, Problem> {
        let mut length = [0u8; 4];
        let mut nonce = [0u8; NONCE_LEN];
        if read_up_to(&mut self.input, &mut length)? < length.len() {
            return Err(Problem::Refused);
        }
        let length = u32::from_le_bytes(length) as usize;
        if !(TAG_LEN..=MOST_RECORD_BYTES).contains(&length)
            || read_up_to(&mut self.input, &mut nonce)? < nonce.len()
        {
            return Err(Problem::Refused);
        }
        // Grown as it is read, so a damaged length can't ask for much memory.
        let mut record = Vec::with_capacity(length.min(1 << 20));
        (&mut self.input)
            .take(length as u64)
            .read_to_end(&mut record)?;
        if record.len() < length {
            return Err(Problem::Refused);
        }
        let opened = self
            .key
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.header.bound_to(self.number)),
                &mut record,
            )
            .map_err(|_| Problem::Refused)?
            .len();
        record.truncate(opened);
        self.number += 1;
        Ok(record)
    }
}

/// Adds a part of an item record: whether it is there, then its length and
/// its bytes.
fn put_part(record: &mut Vec<u8>, part: Option<&[u8]>) {
    match part {
        Some(bytes) => {
            record.push(1);
            record.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            record.extend_from_slice(bytes);
        }
        None => record.push(0),
    }
}

/// Takes the next part off `parts`.
fn take_part<'a>(parts: &mut &'a [u8]) -> Result<Option<&'a [u8]>> {
    let (&present, rest) = parts.split_first().context("a part is missing")?;
    if present == 0 {
        *parts = rest;
        return Ok(None);
    }
    let length = rest.get(..4).context("a part is cut short")?;
    let length = u32::from_le_bytes(length.try_into()?) as usize;
    let bytes = rest.get(4..4 + length).context("a part is cut short")?;
    *parts = &rest[4 + length..];
    Ok(Some(bytes))
}

/// An item record's parts, after its first byte.
fn decode_entry(mut parts: &[u8]) -> Result<Entry> {
    let json = take_part(&mut parts)?.context("an item record without its item")?;
    let html = take_part(&mut parts)?;
    let rtf = take_part(&mut parts)?;
    let image = take_part(&mut parts)?;
    if !parts.is_empty() {
        bail!("an item record with more parts than it should have");
    }
    let item: Item = serde_json::from_slice(json).context("an item of the backup")?;
    let formats = (html.is_some() || rtf.is_some()).then(|| Formats {
        html: html.map(<[u8]>::to_vec),
        rtf: rtf.map(<[u8]>::to_vec),
    });
    Ok(Entry {
        item,
        formats,
        image: image.map(<[u8]>::to_vec),
    })
}

/// Fills as much of `buffer` as `input` has. Returns how much it filled:
/// less only where `input` ends.
fn read_up_to(input: &mut impl Read, buffer: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match input.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;

    /// Few rounds, so the tests run quickly; the reader takes the number
    /// from the header.
    const TEST_ROUNDS: u32 = 1_000;
    const PASSWORD: &str = "correct horse battery";

    /// A history of its own in a new folder.
    fn library(name: &str) -> (PathBuf, Library) {
        let dir = std::env::temp_dir().join(format!("cbm-backup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let images = dir.join("images");
        std::fs::create_dir_all(&images).unwrap();
        let store = Arc::new(Store::open(&dir.join("history.redb"), &images).unwrap());
        (dir, Library::open(store).unwrap())
    }

    fn add_text(library: &mut Library, text: &str) -> u64 {
        let id = library.new_id();
        library
            .add(Item::new(id, Kind::Text, Origin::Clipboard, text.into()))
            .unwrap();
        id
    }

    /// Writes a backup of `library`'s items to `file`.
    fn export_to(library: &Library, file: &Path) {
        let items = library.items().to_vec();
        let written =
            export_with_rounds(library.store(), &items, file, PASSWORD, TEST_ROUNDS, |_| {});
        assert_eq!(written.unwrap(), items.len());
    }

    /// Restores `file` into `library` as the window does: the items it adds
    /// go into the history once the restore hands them over.
    fn restore_into(
        library: &mut Library,
        file: &Path,
        password: &str,
    ) -> Result<Restored, Problem> {
        let store = library.store().clone();
        let known = library.contents();
        let ids = library.ids();
        let mut batches = Vec::new();
        let restored = restore(
            file,
            password,
            &store,
            &known,
            || ids.next(),
            |_| {},
            |batch| batches.push(batch),
        );
        for batch in batches {
            library.add_restored(batch).unwrap();
        }
        restored
    }

    /// An item as JSON without what a restore gives it anew: its id and
    /// the name of its picture's file.
    fn comparable(item: &Item) -> serde_json::Value {
        let mut json = serde_json::to_value(item).unwrap();
        let fields = json.as_object_mut().unwrap();
        fields.remove("id");
        fields.remove("image");
        json
    }

    /// A backup that `library` wrote, and its bytes.
    fn written_backup(dir: &Path, library: &Library) -> (PathBuf, Vec<u8>) {
        let file = dir.join("history.cmbackup");
        export_to(library, &file);
        let bytes = std::fs::read(&file).unwrap();
        (file, bytes)
    }

    /// A history with a few items of each kind, for the tests that change a
    /// backup's bytes.
    fn small_history(name: &str) -> (PathBuf, Library) {
        let (dir, mut library) = library(name);
        for text in ["first", "second", "third"] {
            add_text(&mut library, text);
        }
        (dir, library)
    }

    /// Whether restoring `bytes` into an empty history is refused, and adds
    /// nothing.
    fn refused(name: &str, bytes: &[u8]) -> bool {
        let (dir, mut target) = library(name);
        let file = dir.join("changed.cmbackup");
        std::fs::write(&file, bytes).unwrap();
        let result = restore_into(&mut target, &file, PASSWORD);
        let empty = target.items().is_empty() && target.store().all_items().unwrap().is_empty();
        drop(target);
        std::fs::remove_dir_all(dir).ok();
        matches!(result, Err(Problem::Refused)) && empty
    }

    #[test]
    fn a_backup_brings_back_every_item_with_its_parts() {
        let (dir, mut source) = library("round-trip");
        let note = add_text(&mut source, "Meeting moved to Thursday");
        source.toggle_pin(note).unwrap();
        source.add_tags(note, "work, #Plans").unwrap();
        source.record_use(note).unwrap();
        let link = source.new_id();
        let mut it = Item::new(
            link,
            Kind::Link,
            Origin::Clipboard,
            "https://example.com".into(),
        );
        it.created = 1_700_000_000;
        it.app = Some("chrome.exe".into());
        it.program = Some(PathBuf::from(r"C:\Program Files\Chrome\chrome.exe"));
        source.add(it).unwrap();
        // A formatted copy.
        let formatted = source.new_id();
        let formats = Formats {
            html: Some(b"<b>bold words</b>".to_vec()),
            rtf: Some(b"{\\rtf1 {\\b bold words}}".to_vec()),
        };
        source.store().put_formats(formatted, &formats).unwrap();
        let mut it = Item::new(
            formatted,
            Kind::Text,
            Origin::Clipboard,
            "bold words".into(),
        );
        it.formatted = true;
        source.add(it).unwrap();
        // A copied picture with its text read.
        let picture = image::RgbaImage::from_fn(5, 3, |x, y| {
            image::Rgba([x as u8 * 40, y as u8 * 80, 7, 255])
        });
        let shot = source.new_id();
        let mut it = Item::new(shot, Kind::Image, Origin::Clipboard, "".into());
        it.image = Some(source.store().images.save(shot, &picture).unwrap());
        it.width = 5;
        it.height = 3;
        it.hash = 42;
        it.ocr = Some("Total Due".into());
        source.add(it).unwrap();
        // An imported file.
        let file_id = source.new_id();
        let mut it = Item::new(
            file_id,
            Kind::Text,
            Origin::Imported,
            "notes on disk".into(),
        );
        it.source = Some(PathBuf::from(r"C:\Notes\notes.txt"));
        source.add(it).unwrap();

        let (file, _) = written_backup(&dir, &source);
        let (target_dir, mut target) = library("round-trip-target");
        let restored = restore_into(&mut target, &file, PASSWORD).unwrap();
        assert_eq!(
            restored,
            Restored {
                added: 5,
                skipped: 0
            }
        );
        // From the disk, as the next start reads them.
        let mut back = target.store().all_items().unwrap();
        back.sort_by_key(|it| it.id);
        assert_eq!(back.len(), 5);
        for (before, after) in source.items().iter().zip(&back) {
            assert_eq!(comparable(before), comparable(after), "item {}", before.id);
        }
        let pinned = &back[0];
        assert!(pinned.pinned);
        assert_eq!(pinned.tags, ["work", "Plans"]);
        assert_eq!(pinned.uses, 1);
        let formatted = &back[2];
        assert!(formatted.formatted);
        assert_eq!(target.store().formats(formatted.id).unwrap(), Some(formats));
        let shot = &back[3];
        let stored = shot.image.as_deref().expect("the picture is back");
        let opened = target.store().images.open(shot.id, stored).unwrap();
        assert_eq!(opened.into_rgba8(), picture);
        assert_eq!(shot.ocr.as_deref(), Some("Total Due"));
        assert_eq!(back[4].origin, Origin::Imported);
        // The texts never show in the file.
        let bytes = std::fs::read(&file).unwrap();
        for text in ["Meeting", "example.com", "bold words", "Total Due", "Plans"] {
            assert!(
                !bytes.windows(text.len()).any(|w| w == text.as_bytes()),
                "{text} shows in the backup"
            );
        }
        drop((source, target));
        std::fs::remove_dir_all(dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
    }

    #[test]
    fn an_empty_history_makes_a_backup_too() {
        let (dir, mut library) = library("empty");
        let (file, _) = written_backup(&dir, &library);
        let restored = restore_into(&mut library, &file, PASSWORD).unwrap();
        assert_eq!(
            restored,
            Restored {
                added: 0,
                skipped: 0
            }
        );
        drop(library);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_wrong_password_is_refused_and_adds_nothing() {
        let (dir, library) = small_history("wrong-password");
        let (file, _) = written_backup(&dir, &library);
        let (target_dir, mut target) = self::library("wrong-password-target");
        let mut opened = false;
        let result = restore(
            &file,
            "not the password",
            target.store(),
            &target.contents(),
            || target.ids().next(),
            |step| opened |= matches!(step, Step::Opened),
            |_| panic!("nothing is handed over"),
        );
        assert!(matches!(result, Err(Problem::Refused)), "{result:?}");
        assert!(!opened);
        assert!(target.store().all_items().unwrap().is_empty());
        // The right one opens it.
        assert_eq!(restore_into(&mut target, &file, PASSWORD).unwrap().added, 3);
        drop((library, target));
        std::fs::remove_dir_all(dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
    }

    #[test]
    fn a_changed_byte_is_refused_and_adds_nothing() {
        let (dir, library) = small_history("changed");
        let (_, bytes) = written_backup(&dir, &library);
        // The rounds, the salt, a record's length, its nonce, what it holds,
        // the middle of the file, and the end record's tag.
        let places = [
            MAGIC.len() + 4,
            HEADER_LEN - 1,
            HEADER_LEN,
            HEADER_LEN + 4,
            HEADER_LEN + 4 + NONCE_LEN,
            bytes.len() / 2,
            bytes.len() - 1,
        ];
        for at in places {
            let mut changed = bytes.clone();
            changed[at] ^= 0x01;
            assert!(refused(&format!("changed-{at}"), &changed), "byte {at}");
        }
        drop(library);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_cut_file_or_one_with_more_after_its_end_is_refused() {
        let (dir, library) = small_history("cut");
        let (_, bytes) = written_backup(&dir, &library);
        // The end record: its length, nonce, kind, count and tag.
        let end_record = 4 + NONCE_LEN + 1 + 8 + TAG_LEN;
        for length in [
            HEADER_LEN,
            HEADER_LEN + 2,
            bytes.len() / 2,
            bytes.len() - end_record,
            bytes.len() - 1,
        ] {
            assert!(
                refused(&format!("cut-{length}"), &bytes[..length]),
                "cut at {length} of {}",
                bytes.len()
            );
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert!(refused("longer", &longer));
        drop(library);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn records_cant_be_reordered() {
        let (dir, library) = small_history("reordered");
        let (_, bytes) = written_backup(&dir, &library);
        // Each record's place in the file: its length is the 4 bytes before
        // its nonce.
        let mut places = Vec::new();
        let mut at = HEADER_LEN;
        while at < bytes.len() {
            let length = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            let end = at + 4 + NONCE_LEN + length;
            places.push(at..end);
            at = end;
        }
        assert_eq!(places.len(), 4, "three items and the end");
        let mut swapped = bytes[..HEADER_LEN].to_vec();
        for record in [&places[1], &places[0], &places[2], &places[3]] {
            swapped.extend_from_slice(&bytes[record.clone()]);
        }
        assert!(refused("swapped", &swapped));
        let mut dropped = bytes[..HEADER_LEN].to_vec();
        for record in [&places[0], &places[2], &places[3]] {
            dropped.extend_from_slice(&bytes[record.clone()]);
        }
        assert!(refused("dropped", &dropped));
        drop(library);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_file_that_is_no_backup_or_from_a_later_version_is_told_apart() {
        let dir = std::env::temp_dir().join(format!("cbm-backup-header-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("other.cmbackup");
        std::fs::write(&file, b"just some notes").unwrap();
        assert!(matches!(check_file(&file), Err(Problem::NotBackup)));
        let mut later = Header::new(TEST_ROUNDS, &[3; SALT_LEN]).bytes.to_vec();
        later[MAGIC.len()..MAGIC.len() + 4].copy_from_slice(&(VERSION + 1).to_le_bytes());
        std::fs::write(&file, &later).unwrap();
        assert!(matches!(check_file(&file), Err(Problem::Newer)));
        std::fs::write(&file, Header::new(TEST_ROUNDS, &[3; SALT_LEN]).bytes).unwrap();
        assert!(check_file(&file).is_ok());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_restore_skips_items_already_in_the_history() {
        let (dir, mut source) = library("skips");
        for text in ["alpha", "beta", "gamma"] {
            add_text(&mut source, text);
        }
        let (file, _) = written_backup(&dir, &source);
        let (target_dir, mut target) = library("skips-target");
        add_text(&mut target, "beta");
        // The same content as another kind of item is not the same item.
        let files = target.new_id();
        let mut it = Item::new(files, Kind::Files, Origin::Clipboard, "C:\\a.txt".into());
        it.hash = crate::store::fnv1a(b"gamma");
        target.add(it).unwrap();
        let restored = restore_into(&mut target, &file, PASSWORD).unwrap();
        assert_eq!(
            restored,
            Restored {
                added: 2,
                skipped: 1
            }
        );
        let texts = |library: &Library| {
            let mut texts: Vec<String> = library
                .store()
                .all_items()
                .unwrap()
                .iter()
                .filter(|it| it.kind == Kind::Text)
                .map(|it| it.text.to_string())
                .collect();
            texts.sort();
            texts
        };
        assert_eq!(texts(&target), ["alpha", "beta", "gamma"]);
        // A second time, everything is there already.
        let again = restore_into(&mut target, &file, PASSWORD).unwrap();
        assert_eq!(
            again,
            Restored {
                added: 0,
                skipped: 3
            }
        );
        assert_eq!(target.items().len(), 4);
        let ids: HashSet<u64> = target.items().iter().map(|it| it.id).collect();
        assert_eq!(ids.len(), 4, "each item has its own id");
        drop((source, target));
        std::fs::remove_dir_all(dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
    }

    #[test]
    fn a_failed_export_keeps_the_backup_already_there() {
        let (dir, library) = small_history("keeps");
        let file = dir.join("history.cmbackup");
        std::fs::write(&file, b"an older backup").unwrap();
        // A folder where its part file would go makes the export fail.
        let mut name = file.as_os_str().to_owned();
        name.push(".part");
        std::fs::create_dir_all(PathBuf::from(name)).unwrap();
        let items = library.items().to_vec();
        let written = export_with_rounds(
            library.store(),
            &items,
            &file,
            PASSWORD,
            TEST_ROUNDS,
            |_| {},
        );
        assert!(written.is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"an older backup");
        drop(library);
        std::fs::remove_dir_all(dir).ok();
    }
}
