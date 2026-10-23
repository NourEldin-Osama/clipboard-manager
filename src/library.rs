//! The clipboard history: every item in memory, with its search text and its
//! vector, kept in step with the database. All changes to the history go
//! through here; there is no UI code, so the rules can be tested on their own.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;

use crate::embed::Embedding;
use crate::store::{now, Item, Kind, Origin, Store};

/// Longest tag kept, in characters.
const MAX_TAG_CHARS: usize = 40;

pub struct Library {
    store: Arc<Store>,
    /// Every item, in the order it was added.
    items: Vec<Item>,
    /// Each item's searchable text, lower-cased, for keyword search.
    lower: HashMap<u64, String>,
    /// The current model's vector for each item that has one.
    embeddings: HashMap<u64, Embedding>,
    next_id: u64,
}

/// An item taken out of the history but not yet deleted from the disk, so it
/// can still be put back ("Undo").
pub struct Removed {
    pub item: Item,
    /// Where it was in `items()`, so it comes back in the same place.
    pub position: usize,
    embedding: Option<Embedding>,
}

impl Library {
    pub fn open(store: Arc<Store>) -> Result<Self> {
        let items = store.all_items()?;
        let next_id = store.next_id()?;
        let lower = items
            .iter()
            .map(|i| (i.id, i.searchable_text().to_lowercase()))
            .collect();
        Ok(Self {
            store,
            items,
            lower,
            embeddings: HashMap::new(),
            next_id,
        })
    }

    /// For the worker and the image files; history rows change only here.
    pub fn store(&self) -> &Arc<Store> {
        &self.store
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn lower(&self) -> &HashMap<u64, String> {
        &self.lower
    }

    pub fn embeddings(&self) -> &HashMap<u64, Embedding> {
        &self.embeddings
    }

    pub fn get(&self, id: u64) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// An id no item has used in this session.
    pub fn new_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Saves a new item and adds it to the history.
    pub fn add(&mut self, item: Item) -> Result<()> {
        self.store.put(&item)?;
        self.lower
            .insert(item.id, item.searchable_text().to_lowercase());
        self.items.push(item);
        Ok(())
    }

    /// A repeated copy: the clipboard item with the same content counts as
    /// new again (it moves to the top). Returns whether there was one.
    pub fn touch_duplicate(&mut self, hash: u64) -> Result<bool> {
        let Some(it) = self
            .items
            .iter_mut()
            .find(|i| i.hash == hash && i.origin == Origin::Clipboard)
        else {
            return Ok(false);
        };
        it.created = now();
        let (id, created) = (it.id, it.created);
        self.store.update(id, |stored| stored.created = created)?;
        Ok(true)
    }

    /// Copying an item from the list counts as a use ("Most used"), and makes
    /// a clipboard item recent.
    pub fn record_use(&mut self, id: u64) -> Result<()> {
        let Some(it) = self.items.iter_mut().find(|i| i.id == id) else {
            return Ok(());
        };
        it.uses = it.uses.saturating_add(1);
        if it.origin == Origin::Clipboard {
            it.created = now();
        }
        let (uses, created) = (it.uses, it.created);
        self.store.update(id, |stored| {
            stored.uses = uses;
            stored.created = created;
        })?;
        Ok(())
    }

    pub fn toggle_pin(&mut self, id: u64) -> Result<()> {
        let Some(it) = self.items.iter_mut().find(|i| i.id == id) else {
            return Ok(());
        };
        it.pinned = !it.pinned;
        let pinned = it.pinned;
        self.store.update(id, |stored| stored.pinned = pinned)?;
        Ok(())
    }

    /// Text the OCR worker found and already saved.
    pub fn set_ocr(&mut self, id: u64, text: &str) {
        if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
            it.ocr = Some(text.into());
            self.lower.insert(id, it.searchable_text().to_lowercase());
        }
    }

    /// Images the OCR worker has not read yet.
    pub fn images_without_ocr(&self) -> impl Iterator<Item = u64> + '_ {
        self.items
            .iter()
            .filter(|i| i.kind == Kind::Image && i.ocr.is_none())
            .map(|i| i.id)
    }

    // ----- tags --------------------------------------------------------------

    /// Every tag in use with its item count, most used first.
    pub fn all_tags(&self) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, (String, usize)> = HashMap::new();
        for t in self.items.iter().flat_map(|i| &i.tags) {
            counts
                .entry(t.to_lowercase())
                .or_insert_with(|| (t.clone(), 0))
                .1 += 1;
        }
        let mut tags: Vec<(String, usize)> = counts.into_values().collect();
        tags.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });
        tags
    }

    pub fn any_has_tag(&self, tag: &str) -> bool {
        self.items.iter().any(|i| i.has_tag(tag))
    }

    /// Adds comma-separated tags, keeping the spelling of a tag already in use.
    pub fn add_tags(&mut self, id: u64, input: &str) -> Result<()> {
        let known = self.all_tags();
        let Some(it) = self.items.iter_mut().find(|i| i.id == id) else {
            return Ok(());
        };
        for t in input
            .split(',')
            .map(|t| t.trim().trim_start_matches('#').trim())
            .filter(|t| !t.is_empty())
        {
            let t: String = t.chars().take(MAX_TAG_CHARS).collect();
            if it.has_tag(&t) {
                continue;
            }
            let t = known
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(&t))
                .map(|(k, _)| k.clone())
                .unwrap_or(t);
            it.tags.push(t);
        }
        self.save_tags(id)
    }

    pub fn remove_tag(&mut self, id: u64, tag: &str) -> Result<()> {
        if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
            it.tags.retain(|t| !t.eq_ignore_ascii_case(tag));
        }
        self.save_tags(id)
    }

    /// Removes a tag from every item that has it. Returns how many had it.
    pub fn delete_tag(&mut self, tag: &str) -> Result<usize> {
        let ids: Vec<u64> = self
            .items
            .iter()
            .filter(|i| i.has_tag(tag))
            .map(|i| i.id)
            .collect();
        for id in &ids {
            self.remove_tag(*id, tag)?;
        }
        Ok(ids.len())
    }

    /// Saves an item's tags, and only them, so OCR text saved meanwhile stays.
    fn save_tags(&mut self, id: u64) -> Result<()> {
        let Some(it) = self.items.iter().find(|i| i.id == id) else {
            return Ok(());
        };
        let (tags, lower) = (it.tags.clone(), it.searchable_text().to_lowercase());
        self.store.update(id, |stored| stored.tags = tags)?;
        self.lower.insert(id, lower);
        Ok(())
    }

    // ----- vectors -----------------------------------------------------------

    /// A newly loaded model's stored vectors replace the old model's.
    pub fn set_embeddings(&mut self, stored: Vec<(u64, Embedding)>) {
        self.embeddings = stored.into_iter().collect();
    }

    pub fn set_embedding(&mut self, id: u64, e: Embedding) {
        self.embeddings.insert(id, e);
    }

    /// Another model is loading: the vectors in memory no longer match it.
    pub fn clear_embeddings(&mut self) {
        self.embeddings.clear();
    }

    /// Items the current model has no vector for yet.
    pub fn missing_embeddings(&self) -> impl Iterator<Item = u64> + '_ {
        self.items
            .iter()
            .filter(|i| !self.embeddings.contains_key(&i.id))
            .map(|i| i.id)
    }

    /// Hashes of the files imported so far, to skip importing them again.
    pub fn imported_hashes(&self) -> HashSet<u64> {
        self.items
            .iter()
            .filter(|i| i.origin == Origin::Imported)
            .map(|i| i.hash)
            .collect()
    }

    // ----- removing ----------------------------------------------------------

    /// Takes an item out of the history in memory only: `forget` then deletes
    /// it from the disk, and `put_back` undoes the removal.
    pub fn take(&mut self, id: u64) -> Option<Removed> {
        let position = self.items.iter().position(|i| i.id == id)?;
        let item = self.items.remove(position);
        self.lower.remove(&id);
        let embedding = self.embeddings.remove(&id);
        Some(Removed {
            item,
            position,
            embedding,
        })
    }

    pub fn put_back(&mut self, removed: Removed) {
        let Removed {
            item,
            position,
            embedding,
        } = removed;
        self.lower
            .insert(item.id, item.searchable_text().to_lowercase());
        if let Some(e) = embedding {
            self.embeddings.insert(item.id, e);
        }
        let position = position.min(self.items.len());
        self.items.insert(position, item);
    }

    /// Deletes a taken item from the disk: its row, vectors and image file.
    pub fn forget(&self, removed: Removed) -> Result<()> {
        self.store.delete(&removed.item)
    }

    /// How many items Clear history would delete.
    pub fn clearable_count(&self) -> usize {
        self.items.iter().filter(|i| is_clearable(i)).count()
    }

    /// Deletes the oldest clearable items beyond `max`. Returns the removed ids.
    pub fn prune(&mut self, max: usize) -> Result<Vec<u64>> {
        let mut history: Vec<(i64, u64)> = self
            .items
            .iter()
            .filter(|i| is_clearable(i))
            .map(|i| (i.created, i.id))
            .collect();
        if history.len() <= max {
            return Ok(Vec::new());
        }
        history.sort();
        let excess = history.len() - max;
        let doomed: HashSet<u64> = history.into_iter().take(excess).map(|(_, id)| id).collect();
        self.remove_where(|i| doomed.contains(&i.id))
    }

    /// Deletes every clearable item. Returns the removed ids.
    pub fn clear(&mut self) -> Result<Vec<u64>> {
        self.remove_where(is_clearable)
    }

    /// Deletes the matching items from the disk in one transaction, then from
    /// memory. If the disk part fails, nothing changes.
    fn remove_where(&mut self, doomed: impl Fn(&Item) -> bool) -> Result<Vec<u64>> {
        let removed: Vec<Item> = self.items.iter().filter(|i| doomed(i)).cloned().collect();
        self.store.delete_many(&removed)?;
        let ids: Vec<u64> = removed.iter().map(|i| i.id).collect();
        for id in &ids {
            self.lower.remove(id);
            self.embeddings.remove(id);
        }
        self.items.retain(|i| !doomed(i));
        Ok(ids)
    }
}

/// What the history limit and Clear history may delete: copied items the
/// user did not pin or tag. Imported files stay too.
fn is_clearable(item: &Item) -> bool {
    item.origin == Origin::Clipboard && !item.is_kept()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Kind;

    fn open(name: &str) -> (std::path::PathBuf, Library) {
        let dir = std::env::temp_dir().join(format!("cbm-library-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("t.redb"), &dir).unwrap());
        (dir, Library::open(store).unwrap())
    }

    fn add(lib: &mut Library, text: &str, created: i64) -> u64 {
        let id = lib.new_id();
        let mut item = Item::new(id, Kind::Text, Origin::Clipboard, text.into());
        item.created = created;
        lib.add(item).unwrap();
        id
    }

    #[test]
    fn changes_reach_the_disk_and_survive_a_reopen() {
        let (dir, mut lib) = open("reopen");
        let a = add(&mut lib, "Invoice", 1);
        lib.toggle_pin(a).unwrap();
        lib.add_tags(a, "work, #Taxes").unwrap();
        lib.record_use(a).unwrap();
        let store = lib.store.clone();
        drop(lib);
        let lib = Library::open(store).unwrap();
        let it = lib.get(a).unwrap();
        assert!(it.pinned);
        assert_eq!(it.tags, ["work", "Taxes"]);
        assert_eq!(it.uses, 1);
        assert!(lib.lower()[&a].contains("#taxes"), "tags are searchable");
        drop(lib);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn tags_keep_the_spelling_already_in_use() {
        let (dir, mut lib) = open("tags");
        let a = add(&mut lib, "a", 1);
        let b = add(&mut lib, "b", 2);
        lib.add_tags(a, "Work").unwrap();
        lib.add_tags(b, "work, work").unwrap();
        assert_eq!(lib.get(b).unwrap().tags, ["Work"]);
        assert_eq!(lib.all_tags(), [("Work".to_string(), 2)]);
        assert_eq!(lib.delete_tag("WORK").unwrap(), 2);
        assert!(!lib.any_has_tag("work"));
        drop(lib);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn prune_and_clear_keep_pinned_and_tagged_items() {
        let (dir, mut lib) = open("prune");
        let old = add(&mut lib, "old", 1);
        let pinned = add(&mut lib, "pinned", 2);
        let tagged = add(&mut lib, "tagged", 3);
        let new = add(&mut lib, "new", 4);
        lib.toggle_pin(pinned).unwrap();
        lib.add_tags(tagged, "keep").unwrap();
        assert_eq!(lib.clearable_count(), 2);
        assert_eq!(lib.prune(1).unwrap(), [old]);
        assert_eq!(lib.clearable_count(), 1);
        assert_eq!(lib.clear().unwrap(), [new]);
        assert_eq!(lib.clearable_count(), 0);
        let left: Vec<u64> = lib.items().iter().map(|i| i.id).collect();
        assert_eq!(left, [pinned, tagged]);
        assert_eq!(lib.store.all_items().unwrap().len(), 2);
        drop(lib);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_taken_item_can_be_put_back_or_forgotten() {
        let (dir, mut lib) = open("undo");
        let a = add(&mut lib, "first", 1);
        let b = add(&mut lib, "second", 2);
        lib.set_embedding(a, Embedding::Dense(vec![1.0]));
        let removed = lib.take(a).unwrap();
        assert!(lib.get(a).is_none() && lib.embeddings().is_empty());
        lib.put_back(removed);
        assert_eq!(lib.items()[0].id, a, "it comes back in its place");
        assert!(lib.embeddings().contains_key(&a));
        let removed = lib.take(b).unwrap();
        lib.forget(removed).unwrap();
        assert!(lib.store.get(b).unwrap().is_none());
        drop(lib);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_repeated_copy_moves_the_old_item_to_the_top() {
        let (dir, mut lib) = open("duplicate");
        let a = add(&mut lib, "same", 1);
        let hash = lib.get(a).unwrap().hash;
        assert!(lib.touch_duplicate(hash).unwrap());
        assert!(lib.get(a).unwrap().created > 1);
        assert!(lib.store.get(a).unwrap().unwrap().created > 1);
        assert!(!lib.touch_duplicate(hash ^ 1).unwrap());
        drop(lib);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ocr_text_becomes_searchable() {
        let (dir, mut lib) = open("ocr");
        let id = lib.new_id();
        lib.add(Item::new(id, Kind::Image, Origin::Clipboard, "".into()))
            .unwrap();
        assert_eq!(lib.images_without_ocr().collect::<Vec<_>>(), [id]);
        lib.set_ocr(id, "Total Due");
        assert!(lib.lower()[&id].contains("total due"));
        assert_eq!(lib.images_without_ocr().count(), 0);
        drop(lib);
        std::fs::remove_dir_all(dir).ok();
    }
}
