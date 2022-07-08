/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Transposition tables.
//!
//! A transposition table is a large hash-map from hashkeys of board positions
//! to useful information about each position. The intent of a transposition
//! table is twofold: first, if the same position is reached through multiple
//! lines, the engine can reuse its old evaluation. Second, in multithreaded
//! contexts, the transposition table is the only way in which two threads can
//! communicate about their search.
//!
//! Fiddler's transposition table has no locks and is unsafe; i.e. it has
//! concurrent access to the same entries. We require that the retrieved move
//! from a transposition table be checked for legality before it is played.

use std::{
    alloc::{alloc_zeroed, dealloc, realloc, Layout},
    marker::PhantomData,
    mem::size_of,
    ptr::null,
};

use fiddler_base::{Eval, Move};

#[derive(Clone, Debug)]
/// A table which stores transposition data. It will automatically evict an
/// "old" element if another one takes its place. It behaves much like a
/// hash-map from positions to table-entries.
pub struct TTable {
    /// List of all entries in the transposition table. The length of `entries`
    /// must always be a power of two. To allow concurrent access, we must use
    /// unsafe code.
    ///
    /// If `entries` is null, then nothing has been allocated yet.
    entries: *mut TTEntry,
    /// The mask for retrieving entries from the table. Should always be 0 if
    /// `entries` is null.
    mask: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A safe-exposed API for accessing entries in a transposition table. The guard
/// is annotated with a lifetime so that it cannot outlive the table it indexes
/// into.
pub struct TTEntryGuard<'a> {
    /// Whether the entry we point to is actually valid.
    valid: bool,
    /// The hash which created the reference in the table.
    hash: u64,
    /// A pointer to the entry in the transposition table.
    entry: *mut TTEntry,
    /// Ensures that the guard does not outlive the table it points to.
    _phantom: PhantomData<&'a TTable>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// An entry in the transposition table.
pub struct TTEntry {
    /// The age of the entry, i.e. the number of searches since this entry was
    /// inserted.
    age: u8, // 1 byte
    /// The hash key of the entry.
    hash: u64, // 8 bytes
    /// The depth to which this entry was searched.
    pub depth: u8, // 1 byte
    /// The best move in the position when this entry was searched. Will be
    /// `Move::BAD_MOVE` when there are no moves or the best move is unknown.
    pub best_move: Move, // 2 bytes
    /// The lower bound on the evaluation of the position.
    pub lower_bound: Eval, // 2 bytes
    /// The upper bound on the evaluation of the position.
    pub upper_bound: Eval, // 2 bytes

                           // total size of an entry: 16 bytes. TODO think of ways of shrinking this.
}

impl TTable {
    /// Construct a new TTable with no entries.
    pub const fn new() -> TTable {
        TTable {
            entries: null::<TTEntry>() as *mut TTEntry,
            mask: 0,
        }
    }

    pub fn with_size(size_mb: usize) -> TTable {
        let max_num_entries = size_mb / size_of::<TTEntry>();
        let new_size = if max_num_entries.is_power_of_two() {
            max_num_entries
        } else {
            // round down to lower power of two
            max_num_entries.next_power_of_two() >> 1
        };

        TTable::with_capacity(new_size.trailing_zeros() as usize)
    }

    /// Create a transposition table with a fixed capacity. The capacity is
    /// *not* the number of entries, but rather log_2 of the number of entries.
    ///
    /// # Panics
    ///
    /// This function will panic if `capacity_log2` is large enough to cause
    /// overflow.
    pub fn with_capacity(capacity_log2: usize) -> TTable {
        let n_entries = 1 << capacity_log2;
        TTable {
            entries: unsafe {
                let layout = Layout::array::<TTEntry>(n_entries).unwrap();
                alloc_zeroed(layout) as *mut TTEntry
            },
            mask: (n_entries - 1) as u64,
        }
    }

    /// Get the evaluation data stored by this table for a given key, if it
    /// exists. Returns `None` if no data corresponding to the key exists.
    pub fn get<'a>(&self, hash_key: u64) -> TTEntryGuard<'a> {
        if self.entries.is_null() {
            // cannot index into empty table.
            return TTEntryGuard {
                valid: false,
                hash: 0,
                entry: self.entries,
                _phantom: PhantomData,
            };
        }
        let idx = (hash_key & self.mask) as usize;
        let entry = unsafe { self.entries.add(idx) };
        let entry_ref = unsafe { entry.as_ref().unwrap() };
        if entry_ref.hash == hash_key && entry_ref.best_move.value() != 0 {
            // it's a match! Return a valid guard.
            TTEntryGuard {
                valid: true,
                hash: entry_ref.hash,
                entry,
                _phantom: PhantomData,
            }
        } else {
            // No match found. Return an invalid guard.
            TTEntryGuard {
                valid: false,
                hash: hash_key,
                entry,
                _phantom: PhantomData,
            }
        }
    }

    /// Get an estimate of the fill rate proportion of this transposition table
    /// out of 1000. Typically used for UCI.
    pub fn fill_rate_permill(&self) -> u16 {
        if self.entries.is_null() {
            // I suppose a transposition table with no entries is 100% full.
            1000
        } else {
            // take a sample of the first 1000 entries
            let mut num_full = 0;

            // if the size is lower than 1000, we will visit some entries twice,
            // but I guess that's OK since it's meant to just be a rough
            // estimate.
            for idx_unbounded in 0..1000 {
                // prevent overflow
                let idx = (idx_unbounded & self.mask) as usize;
                if unsafe { self.entries.add(idx).as_ref().unwrap().hash } != 0 {
                    // a real entry lives here
                    num_full += 1;
                }
            }
            num_full
        }
    }

    /// Age up all the entries in this table, and for any slot which is at
    /// least as old as the max age, evict it.
    pub fn age_up(&mut self, max_age: u8) {
        if !self.entries.is_null() {
            for idx in 0..=self.mask as usize {
                let entry: &mut TTEntry = unsafe { self.entries.add(idx).as_mut().unwrap() };
                entry.age += 1;
                if entry.age >= max_age {
                    *entry = TTEntry::new();
                }
            }
        }
    }

    /// Resize the hash table to use no more than `size_mb` megabytes.
    pub fn resize(&mut self, size_mb: usize) {
        let max_num_entries = size_mb * 1_000_000 / size_of::<TTEntry>();
        let new_size = if max_num_entries.is_power_of_two() {
            max_num_entries
        } else {
            // round down to lower power of two
            max_num_entries.next_power_of_two() >> 1
        };

        let old_entries = self.entries;
        let mut old_size = self.mask as usize + 1;
        if old_entries.is_null() {
            old_size = 0;
        }
        if new_size == 0 {
            if !old_entries.is_null() {
                unsafe {
                    dealloc(
                        old_entries as *mut u8,
                        Layout::array::<TTEntry>(old_size).unwrap(),
                    )
                };
            }
            self.entries = null::<TTEntry>() as *mut TTEntry;
            self.mask = 0;
        } else if new_size < old_size {
            // move entries down if possible
            let new_mask = new_size - 1;
            for idx in new_size..old_size {
                // try to copy the entries which will be deallocated backward
                let entry = unsafe { *self.entries.add(idx) };
                if entry.hash != 0 {
                    // if there was an entry at this index, move it down to fit
                    // into the shrunken table
                    let new_idx = idx & new_mask;
                    // TODO more intelligently overwrite than just blindly
                    // writing
                    let new_entry_slot = unsafe { self.entries.add(new_idx).as_mut().unwrap() };
                    *new_entry_slot = entry;
                }
            }
            // realloc to shrink this
            self.entries = unsafe {
                realloc(
                    self.entries as *mut u8,
                    Layout::array::<TTEntry>(old_size).unwrap(),
                    new_size * size_of::<TTEntry>(),
                ) as *mut TTEntry
            };
            self.mask = new_mask as u64;
        } else {
            // the table is growing
            self.entries = if old_entries.is_null() {
                unsafe { alloc_zeroed(Layout::array::<TTEntry>(new_size).unwrap()) as *mut TTEntry }
            } else {
                let ptr = unsafe {
                    realloc(
                        self.entries as *mut u8,
                        Layout::array::<TTEntry>(old_size).unwrap(),
                        new_size * size_of::<TTEntry>(),
                    ) as *mut TTEntry
                };
                unsafe {
                    // write the new block with zeros
                    ptr.add(old_size).write_bytes(0, new_size - old_size);
                }
                ptr
            };
            let new_mask = (new_size - 1) as u64;
            // the mask got bigger, so some entries may need to move right
            for idx in 0..old_size {
                let entry = unsafe { *self.entries.add(idx) };
                if entry.hash != 0 {
                    // if there was an entry at this index, move it up
                    let new_idx = (entry.hash & new_mask) as usize;
                    // TODO more intelligently overwrite than just blindly
                    // writing
                    let new_entry_slot = unsafe { self.entries.add(new_idx).as_mut().unwrap() };
                    *new_entry_slot = entry;
                }
            }

            self.mask = new_mask;
        }
    }

    /// Get the size of this table, in megabytes. Does not include the size of
    /// the struct itself, but rather just the heap-allocated table size.
    pub fn size_mb(&self) -> usize {
        if self.entries.is_null() {
            0
        } else {
            size_of::<TTEntry>() * (self.mask as usize + 1) / 1_000_000
        }
    }

    /// Clear all entries in the table.
    pub fn clear(&mut self) {
        if !self.entries.is_null() {
            let n_entries = self.mask as usize + 1;
            // fill the whole table with zeros
            unsafe { self.entries.write_bytes(0, n_entries) };
        }
    }
}

impl TTEntry {
    const fn new() -> TTEntry {
        TTEntry {
            age: 0,
            hash: 0,
            depth: 0,
            best_move: Move::BAD_MOVE,
            lower_bound: Eval::DRAW,
            upper_bound: Eval::DRAW,
        }
    }
}

unsafe impl Send for TTable {}
unsafe impl Sync for TTable {}

impl Drop for TTable {
    fn drop(&mut self) {
        if !self.entries.is_null() {
            let size = self.mask as usize + 1;
            // memory was allocated, need to deallocate
            unsafe {
                dealloc(
                    self.entries as *mut u8,
                    Layout::array::<TTEntry>(size).unwrap(),
                );
            }
        }
        // if the size is zero, no allocation was performed
    }
}

impl<'a> TTEntryGuard<'a> {
    /// Get the entry pointed to by this guard. Will return `None` if the guard
    /// was created by a probe miss on the transposition table.
    ///
    /// Due to hash collision, the entry may not be correct for us. Therefore it
    /// is prudent to check that the move in the transposition table is actually
    /// legal before playing it.
    pub fn entry(&self) -> Option<&'a TTEntry> {
        if self.valid {
            // null case is handled by `as_ref()`
            unsafe { self.entry.as_ref() }
        } else {
            None
        }
    }

    /// Save the value pointed to by this entry guard.
    pub fn save(&mut self, depth: u8, best_move: Move, lower_bound: Eval, upper_bound: Eval) {
        if !self.entry.is_null() {
            unsafe {
                *self.entry = TTEntry {
                    age: 0,
                    hash: self.hash,
                    depth,
                    best_move,
                    lower_bound,
                    upper_bound,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use fiddler_base::{Eval, Move, Square};

    use super::{TTEntry, TTable};

    #[test]
    /// Test that a hash table miss is correctly created.
    fn guaranteed_miss() {
        let tt = TTable::with_capacity(4);
        assert!(tt.get(12345).entry().is_none());
    }

    #[test]
    /// Test that we correctly find a hit in a transposition table.
    fn guaranteed_hit() {
        let tt = TTable::with_capacity(4);
        let entry = TTEntry {
            age: 0,
            hash: 12,
            depth: 5,
            best_move: Move::normal(Square::E2, Square::E4),
            lower_bound: Eval::DRAW,
            upper_bound: Eval::centipawns(100),
        };
        tt.get(12).save(
            entry.depth,
            entry.best_move,
            entry.lower_bound,
            entry.upper_bound,
        );

        assert_eq!(tt.get(12).entry(), Some(&entry));
    }

    #[test]
    /// Test that writing to an empty table is a no-op.
    fn attempt_write_empty_table() {
        let tt = TTable::new();
        let entry = TTEntry {
            age: 0,
            hash: 12,
            depth: 5,
            best_move: Move::normal(Square::E2, Square::E4),
            lower_bound: Eval::DRAW,
            upper_bound: Eval::centipawns(100),
        };
        tt.get(12).save(
            entry.depth,
            entry.best_move,
            entry.lower_bound,
            entry.upper_bound,
        );

        assert_eq!(tt.get(12).entry(), None);
    }

    #[test]
    /// Test that entries are preserved after resizing a table.
    fn entry_preserved_after_expand() {
        let mut tt = TTable::with_size(1000);
        let entry = TTEntry {
            age: 0,
            hash: 2022,
            depth: 5,
            best_move: Move::normal(Square::E2, Square::E4),
            lower_bound: Eval::DRAW,
            upper_bound: Eval::centipawns(100),
        };
        tt.get(2022).save(
            entry.depth,
            entry.best_move,
            entry.lower_bound,
            entry.upper_bound,
        );
        tt.resize(2000);

        assert_eq!(tt.get(2022).entry(), Some(&entry));
    }

    #[test]
    /// Test that an entry with the same has can overwrite another entry.
    fn overwrite() {
        let e0 = TTEntry {
            age: 0,
            hash: 2022,
            depth: 5,
            best_move: Move::normal(Square::E2, Square::E4),
            lower_bound: Eval::DRAW,
            upper_bound: Eval::centipawns(100),
        };
        let e1 = TTEntry {
            age: 0,
            hash: 2022,
            depth: 7,
            best_move: Move::normal(Square::E4, Square::E5),
            lower_bound: Eval::BLACK_MATE,
            upper_bound: -Eval::centipawns(100),
        };

        let tt = TTable::with_capacity(4);

        tt.get(2022)
            .save(e0.depth, e0.best_move, e0.lower_bound, e0.upper_bound);

        tt.get(2022)
            .save(e1.depth, e1.best_move, e1.lower_bound, e1.upper_bound);

        assert_eq!(tt.get(2022).entry(), Some(&e1));
    }

    #[test]
    /// Test that an empty transposition table, when resized, works correctly.
    fn resize_empty_table() {
        let mut tt = TTable::new();
        tt.resize(2000);
        let entry = TTEntry {
            age: 0,
            hash: 2022,
            depth: 5,
            best_move: Move::normal(Square::E2, Square::E4),
            lower_bound: Eval::DRAW,
            upper_bound: Eval::centipawns(100),
        };
        tt.get(2022).save(
            entry.depth,
            entry.best_move,
            entry.lower_bound,
            entry.upper_bound,
        );

        assert_eq!(tt.get(2022).entry(), Some(&entry));
    }
}
