//! The deletes that can still be undone. Each delete is one entry: one item,
//! the items selected together, or what delete by age and the history limit
//! removed. Its items are out of the history in memory but stay on the disk
//! until the entry's time is over or the app quits. Ctrl+Z undoes the newest
//! entry, and a message's Undo its own.

use std::time::Duration;

use crate::library::{Library, Removed};

/// How long a delete can be undone. Its time runs as the messages' time
/// does: only while a window shows and the pointer is not on the messages.
const UNDO_WINDOW: Duration = Duration::from_secs(20);

/// One delete that can still be undone.
struct Entry {
    /// Its message's Undo names it by this.
    number: u64,
    /// Taken from the last to the first, so each comes back in its place
    /// when they go back in the other order.
    removed: Vec<Removed>,
    /// How much of `UNDO_WINDOW` has run.
    waited: Duration,
}

impl Entry {
    fn ids(&self) -> Vec<u64> {
        self.removed.iter().map(|r| r.item.id).collect()
    }

    fn can_undo(&self) -> bool {
        self.waited < UNDO_WINDOW
    }
}

#[derive(Default)]
pub(super) struct UndoHistory {
    /// The oldest first.
    entries: Vec<Entry>,
    last_number: u64,
}

impl UndoHistory {
    /// Keeps the items of a delete, as `Library::take` gave them. Returns
    /// the number its message's Undo names.
    pub(super) fn add(&mut self, removed: Vec<Removed>) -> u64 {
        self.last_number += 1;
        self.entries.push(Entry {
            number: self.last_number,
            removed,
            waited: Duration::ZERO,
        });
        self.last_number
    }

    /// Runs `step` of every delete's time.
    pub(super) fn tick(&mut self, step: Duration) {
        for entry in &mut self.entries {
            entry.waited += step;
        }
    }

    /// Takes out the deletes whose time is over: their numbers, and their
    /// items, which are now to be deleted from the disk.
    pub(super) fn take_expired(&mut self) -> (Vec<u64>, Vec<Removed>) {
        let (over, left): (Vec<Entry>, Vec<Entry>) = std::mem::take(&mut self.entries)
            .into_iter()
            .partition(|entry| !entry.can_undo());
        self.entries = left;
        let numbers = over.iter().map(|entry| entry.number).collect();
        let removed = over.into_iter().flat_map(|entry| entry.removed).collect();
        (numbers, removed)
    }

    /// Takes out every delete, as the app quits: their items, to be deleted
    /// from the disk.
    pub(super) fn take_all(&mut self) -> Vec<Removed> {
        let entries = std::mem::take(&mut self.entries);
        entries
            .into_iter()
            .flat_map(|entry| entry.removed)
            .collect()
    }

    /// The newest delete that can still be undone: the one Ctrl+Z undoes.
    pub(super) fn newest(&self) -> Option<u64> {
        let entry = self.entries.iter().rev().find(|entry| entry.can_undo());
        entry.map(|entry| entry.number)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Puts the items of the delete `number` back into `library`, each in
    /// its place. Returns their ids in the order they went back, the last
    /// one taken first; `None` when it was undone already or its time is
    /// over.
    pub(super) fn undo(&mut self, number: u64, library: &mut Library) -> Option<Vec<u64>> {
        let at = self
            .entries
            .iter()
            .position(|entry| entry.number == number && entry.can_undo())?;
        // An item knows its place in the history as it was when it was
        // taken. The deletes after this one go back first and are taken
        // again after it, so every item lands where it was, and theirs keep
        // their places for their own Undo.
        let later: Vec<Vec<u64>> = self.entries[at + 1..].iter().map(Entry::ids).collect();
        for entry in self.entries[at + 1..].iter_mut().rev() {
            put_back(library, std::mem::take(&mut entry.removed));
        }
        let entry = self.entries.remove(at);
        let ids = put_back(library, entry.removed);
        for (entry, ids) in self.entries[at..].iter_mut().zip(later) {
            entry.removed = ids.iter().filter_map(|&id| library.take(id)).collect();
        }
        Some(ids)
    }
}

/// Puts `removed` back into `library`, the last one taken first. Returns
/// their ids in that order.
fn put_back(library: &mut Library, removed: Vec<Removed>) -> Vec<u64> {
    removed
        .into_iter()
        .rev()
        .map(|removed| {
            let id = removed.item.id;
            library.put_back(removed);
            id
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::store::{Item, Kind, Origin, Store};

    /// A history with an item for each of `texts`, in that order, kept in a
    /// folder of its own that `done` removes.
    fn history(name: &str, texts: &[&str]) -> (std::path::PathBuf, Library, Vec<u64>) {
        let dir = std::env::temp_dir().join(format!("cbm-undo-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("t.redb"), &dir).unwrap());
        let mut library = Library::open(store).unwrap();
        let ids = texts
            .iter()
            .map(|text| {
                let id = library.new_id();
                let item = Item::new(id, Kind::Text, Origin::Clipboard, (*text).into());
                library.add(item).unwrap();
                id
            })
            .collect();
        (dir, library, ids)
    }

    fn done(dir: std::path::PathBuf, library: Library) {
        drop(library);
        std::fs::remove_dir_all(dir).ok();
    }

    /// The texts of the history, in its order.
    fn texts(library: &Library) -> Vec<String> {
        library
            .items()
            .iter()
            .map(|it| it.text.to_string())
            .collect()
    }

    /// Deletes `ids` as one delete, the last in the history first, as the
    /// app takes the items selected together.
    fn delete(library: &mut Library, undo: &mut UndoHistory, ids: &[u64]) -> u64 {
        let removed = ids
            .iter()
            .rev()
            .map(|&id| library.take(id).expect("in the history"))
            .collect();
        undo.add(removed)
    }

    /// Three deletes one after another: Ctrl+Z brings back the newest
    /// first, then the one before it, and each item lands in its place.
    #[test]
    fn ctrl_z_undoes_the_newest_delete_then_the_ones_before_it() {
        let (dir, mut library, ids) = history("order", &["a", "b", "c", "d", "e"]);
        let mut undo = UndoHistory::default();
        let first = delete(&mut library, &mut undo, &[ids[1]]);
        let second = delete(&mut library, &mut undo, &[ids[3]]);
        let third = delete(&mut library, &mut undo, &[ids[0]]);
        assert_eq!(texts(&library), ["c", "e"]);
        for (number, after) in [
            (third, vec!["a", "c", "e"]),
            (second, vec!["a", "c", "d", "e"]),
            (first, vec!["a", "b", "c", "d", "e"]),
        ] {
            assert_eq!(undo.newest(), Some(number));
            undo.undo(number, &mut library).expect("it can be undone");
            assert_eq!(texts(&library), after);
        }
        assert_eq!(undo.newest(), None);
        assert!(undo.is_empty());
        assert_eq!(undo.undo(first, &mut library), None, "undone once");
        done(dir, library);
    }

    /// A message's Undo brings back its own delete while newer ones wait:
    /// every item still lands in its place, also when the newer deletes are
    /// undone after it.
    #[test]
    fn an_older_delete_comes_back_in_its_place_while_newer_ones_wait() {
        let (dir, mut library, ids) = history("older", &["a", "b", "c", "d", "e", "f"]);
        let mut undo = UndoHistory::default();
        let first = delete(&mut library, &mut undo, &[ids[1]]);
        let group = delete(&mut library, &mut undo, &[ids[2], ids[4]]);
        let last = delete(&mut library, &mut undo, &[ids[0]]);
        assert_eq!(texts(&library), ["d", "f"]);
        assert_eq!(undo.undo(first, &mut library), Some(vec![ids[1]]));
        assert_eq!(texts(&library), ["b", "d", "f"]);
        assert_eq!(undo.newest(), Some(last), "Ctrl+Z still has the newest");
        undo.undo(group, &mut library).unwrap();
        assert_eq!(texts(&library), ["b", "c", "d", "e", "f"]);
        undo.undo(last, &mut library).unwrap();
        assert_eq!(texts(&library), ["a", "b", "c", "d", "e", "f"]);
        let found: Vec<u64> = ids.iter().map(|&id| library.get(id).unwrap().id).collect();
        assert_eq!(found, ids, "each is found by its id");
        done(dir, library);
    }

    /// Items selected together go in one delete, and one undo brings them
    /// all back.
    #[test]
    fn a_delete_of_items_selected_together_is_one_entry() {
        let (dir, mut library, ids) = history("group", &["a", "b", "c", "d"]);
        let mut undo = UndoHistory::default();
        let group = delete(&mut library, &mut undo, &[ids[0], ids[2], ids[3]]);
        assert_eq!(texts(&library), ["b"]);
        assert_eq!(undo.newest(), Some(group));
        let back = undo.undo(group, &mut library).unwrap();
        assert_eq!(back, [ids[0], ids[2], ids[3]], "the last one taken first");
        assert_eq!(texts(&library), ["a", "b", "c", "d"]);
        assert_eq!(undo.newest(), None, "nothing more to undo");
        done(dir, library);
    }

    /// A delete can be undone for 20 seconds of its time, also after its
    /// message is gone; then it is gone for good, and its items leave the
    /// disk. Each delete has its own 20 seconds.
    #[test]
    fn a_delete_can_be_undone_for_20_seconds() {
        let (dir, mut library, ids) = history("time", &["a", "b", "c"]);
        let mut undo = UndoHistory::default();
        let first = delete(&mut library, &mut undo, &[ids[0]]);
        undo.tick(Duration::from_secs(12));
        let second = delete(&mut library, &mut undo, &[ids[1]]);
        undo.tick(Duration::from_millis(7_900));
        assert_eq!(undo.take_expired().0, Vec::<u64>::new(), "19.9 seconds");
        undo.tick(Duration::from_millis(100));
        assert_eq!(undo.newest(), Some(second));
        assert_eq!(
            undo.undo(first, &mut library),
            None,
            "its 20 seconds are over"
        );
        let (numbers, removed) = undo.take_expired();
        assert_eq!(numbers, [first]);
        library.forget_many(removed).unwrap();
        assert!(
            library.store().get(ids[0]).unwrap().is_none(),
            "gone from the disk"
        );
        assert!(
            library.store().get(ids[1]).unwrap().is_some(),
            "still on the disk"
        );
        undo.tick(Duration::from_secs(11));
        assert_eq!(undo.newest(), Some(second), "19 seconds");
        undo.tick(Duration::from_secs(1));
        assert_eq!(undo.newest(), None);
        assert_eq!(undo.take_expired().0, [second]);
        assert!(undo.is_empty());
        done(dir, library);
    }

    /// Quitting makes every delete final: all their items leave the disk.
    #[test]
    fn quitting_deletes_every_waiting_item_from_the_disk() {
        let (dir, mut library, ids) = history("quit", &["a", "b", "c"]);
        let mut undo = UndoHistory::default();
        delete(&mut library, &mut undo, &[ids[0]]);
        delete(&mut library, &mut undo, &[ids[1], ids[2]]);
        library.forget_many(undo.take_all()).unwrap();
        assert!(undo.is_empty() && undo.newest().is_none());
        assert!(library.store().all_items().unwrap().is_empty());
        done(dir, library);
    }
}
