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

use fiddler_base::Move;

use crate::evaluate::Eval;

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
    buckets: *mut Bucket,
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

/// The number of entries in a single bucket.
const BUCKET_SIZE: usize = 3;
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A `Bucket` is a container for transposition table entries, designed to make
/// cache access faster.
/// The core idea is that we can load all the entries sent to a specific index
/// in the transposition table all fit in one cache line.
struct Bucket {
    /// A block of entries.
    pub entries: [TTEntry; BUCKET_SIZE],
    /// Padding bits to make a bucket exactly 32 bytes, the size of a cache
    /// line.
    _pad: [u8; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// An entry in the transposition table.
pub struct TTEntry {
    /// A packed tag containing the age of the entry in the lower 7 bits and a
    /// bit to determine whether this entry is unused in the highest bit.
    tag: u8, // 1 byte
    /// The lower 16 bits hash key of the entry.
    key_low16: u16, // 2 bytes
    /// The depth to which this entry was searched. If the depth is negative,
    /// this means that it was a special type of search.
    pub depth: i8, // 1 byte
    /// The best move in the position when this entry was searched. Will be
    /// `Move::BAD_MOVE` when there are no moves or the best move is unknown.
    pub best_move: Move, // 2 bytes
    /// The lower bound on the evaluation of the position.
    pub lower_bound: Eval, // 2 bytes
    /// The upper bound on the evaluation of the position.
    pub upper_bound: Eval, // 2 bytes

                           // total size of an entry: 10 bytes. TODO think of ways of shrinking this.
}

impl TTable {
    #[must_use]
    /// Construct a new `TTable` with no entries.
    pub const fn new() -> TTable {
        TTable {
            buckets: null::<Bucket>() as *mut Bucket,
            mask: 0,
        }
    }

    #[must_use]
    /// Construct a `TTable` with a given size, in megabytes.
    pub fn with_size(size_mb: usize) -> TTable {
        let max_num_buckets = size_mb / size_of::<Bucket>();
        let new_size = if max_num_buckets.is_power_of_two() {
            max_num_buckets
        } else {
            // round down to lower power of two
            max_num_buckets.next_power_of_two() >> 1
        };

        TTable::with_capacity(new_size.trailing_zeros() as usize)
    }

    #[must_use]
    #[allow(clippy::cast_ptr_alignment)]
    /// Create a transposition table with a fixed capacity.
    /// The capacity is *not* the number of entries, but rather log base 2 of
    /// the number of buckets.
    ///
    /// # Panics
    ///
    /// This function will panic if `capacity_log2` is large enough to cause
    /// overflow.
    pub fn with_capacity(capacity_log2: usize) -> TTable {
        let n_buckets = 1 << capacity_log2;
        TTable {
            buckets: unsafe {
                let layout = Layout::array::<Bucket>(n_buckets).unwrap();
                alloc_zeroed(layout).cast::<Bucket>()
            },
            mask: (n_buckets - 1) as u64,
        }
    }

    #[inline(always)]
    /// Compute the index for an entry with a given key.
    fn index_for(&self, hash_key: u64) -> usize {
        ((hash_key >> 16) & self.mask) as usize
    }

    #[must_use]
    #[allow(clippy::cast_ptr_alignment, clippy::cast_possible_truncation)]
    /// Get the evaluation data stored by this table for a given key, if it
    /// exists.
    /// Returns `None` if no data corresponding to the key exists.
    ///
    /// # Panics
    ///
    /// This function may panic due to an internal error.
    pub fn get<'a>(&self, hash_key: u64) -> TTEntryGuard<'a> {
        if self.buckets.is_null() {
            // cannot index into empty table.
            return TTEntryGuard {
                valid: false,
                hash: 0,
                entry: null::<TTEntry>() as *mut TTEntry,
                _phantom: PhantomData,
            };
        }
        let idx = self.index_for(hash_key);
        let bucket = unsafe { self.buckets.add(idx) };
        let mut entry_ptr = bucket.cast::<TTEntry>();

        // first, see if we can find a match in the bucket
        for _ in 0..BUCKET_SIZE {
            let entry_ref = unsafe { entry_ptr.as_ref().unwrap() };
            // we intentionally truncate here
            if entry_ref.key_low16 == hash_key as u16 && (entry_ref.tag & 0x80 != 0) {
                // it's a match!
                return TTEntryGuard {
                    valid: true,
                    hash: hash_key,
                    entry: entry_ptr,
                    _phantom: PhantomData,
                };
            }
            entry_ptr = unsafe { entry_ptr.add(1) }
        }

        // no match found. pick the oldest entry to replace
        entry_ptr = bucket.cast::<TTEntry>();
        let mut eldest_entry = entry_ptr;
        let mut eldest_age = 0;
        for _ in 0..BUCKET_SIZE {
            let entry_ref = unsafe { entry_ptr.as_ref().unwrap() };
            if entry_ref.tag & 0x80 == 0 {
                // we found an unoccupied entry. return it immediately
                return TTEntryGuard {
                    valid: false,
                    hash: hash_key,
                    entry: entry_ptr,
                    _phantom: PhantomData,
                };
            }
            let age = entry_ref.tag & 0x7F;
            if age > eldest_age {
                eldest_entry = entry_ptr;
                eldest_age = age;
            }
            entry_ptr = unsafe { entry_ptr.add(1) };
        }

        TTEntryGuard {
            valid: false,
            hash: hash_key,
            entry: eldest_entry,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    /// Get an estimate of the fill rate proportion of this transposition table
    /// out of 1000. Typically used for UCI.
    ///
    /// # Panics
    ///
    /// This function may panic due to an internal error.
    pub fn fill_rate_permill(&self) -> u16 {
        if self.buckets.is_null() {
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
                let bucket = unsafe {
                    self.buckets
                        .add((idx_unbounded & self.mask) as usize)
                        .as_ref()
                        .unwrap()
                };
                num_full += bucket.entries.iter().filter(|e| e.tag & 0x80 != 0).count();
            }
            (num_full / BUCKET_SIZE) as u16
        }
    }

    /// Age up all the entries in this table, and for any slot which is at
    /// least as old as the max age, evict it.
    ///
    /// # Panics
    ///
    /// This function may panic due to an internal error.
    pub fn age_up(&mut self, max_age: u8) {
        debug_assert!(max_age <= 0x7F);
        if !self.buckets.is_null() {
            for idx in 0..=self.mask {
                let bucket = unsafe { self.buckets.add(self.index_for(idx)).as_mut().unwrap() };
                for entry in &mut bucket.entries {
                    if entry.tag & 0x7F > max_age {
                        *entry = TTEntry::new();
                    }
                }
            }
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_ptr_alignment)]
    /// Resize the hash table to use no more than `size_mb` megabytes.
    ///
    /// # Panics
    ///
    /// Will panic in the case of an OOM or allocation failure.
    pub fn resize(&mut self, size_mb: usize) {
        let max_num_entries = size_mb * 1_000_000 / size_of::<Bucket>();
        let new_size = if max_num_entries.is_power_of_two() {
            max_num_entries
        } else {
            // round down to lower power of two
            max_num_entries.next_power_of_two() >> 1
        };

        let old_buckets = self.buckets;
        let mut old_size = self.mask as usize + 1;
        if old_buckets.is_null() {
            old_size = 0;
        }
        if new_size == 0 {
            if !old_buckets.is_null() {
                unsafe {
                    dealloc(
                        old_buckets.cast::<u8>(),
                        Layout::array::<Bucket>(old_size).unwrap(),
                    );
                };
            }
            self.buckets = null::<Bucket>() as *mut Bucket;
            self.mask = 0;
        } else if new_size < old_size {
            // move entries down if possible
            let new_mask = new_size - 1;
            for idx in new_size..old_size {
                // try to copy the entries which will be deallocated backward
                let bucket = unsafe { *self.buckets.add(idx) };
                // if there was an entry at this index, move it down to fit
                // into the shrunken table
                let new_idx = idx & new_mask;
                // TODO more intelligently overwrite than just blindly
                // writing
                let new_bucket_slot = unsafe { self.buckets.add(new_idx).as_mut().unwrap() };
                *new_bucket_slot = bucket;
            }
            // realloc to shrink this
            self.buckets = unsafe {
                realloc(
                    self.buckets.cast::<u8>(),
                    Layout::array::<Bucket>(old_size).unwrap(),
                    new_size * size_of::<Bucket>(),
                )
                .cast::<Bucket>()
            };
            self.mask = new_mask as u64;
        } else {
            // the table is growing
            self.buckets = unsafe {
                alloc_zeroed(Layout::array::<Bucket>(new_size).unwrap()).cast::<Bucket>()
            };

            self.mask = (new_size - 1) as u64;
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    /// Get the size of this table, in megabytes. Does not include the size of
    /// the struct itself, but rather just the heap-allocated table size.
    pub fn size_mb(&self) -> usize {
        if self.buckets.is_null() {
            0
        } else {
            size_of::<Bucket>() * (self.mask as usize + 1) / 1_000_000
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    /// Clear all entries in the table.
    pub fn clear(&mut self) {
        if !self.buckets.is_null() {
            let n_entries = self.mask as usize + 1;
            // fill the whole table with zeros
            unsafe { self.buckets.write_bytes(0, n_entries) };
        }
    }
}

impl TTEntry {
    const fn new() -> TTEntry {
        TTEntry {
            tag: 0,
            key_low16: 0,
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
    #[allow(clippy::cast_possible_truncation)]
    fn drop(&mut self) {
        if !self.buckets.is_null() {
            let size = self.mask as usize + 1;
            // memory was allocated, need to deallocate
            unsafe {
                dealloc(
                    self.buckets.cast::<u8>(),
                    Layout::array::<Bucket>(size).unwrap(),
                );
            }
        }
        // if the size is zero, no allocation was performed
    }
}

impl<'a> TTEntryGuard<'a> {
    #[must_use]
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

    #[allow(clippy::cast_possible_truncation)]
    /// Save the value pointed to by this entry guard.
    pub fn save(&mut self, depth: i8, best_move: Move, lower_bound: Eval, upper_bound: Eval) {
        if !self.entry.is_null() {
            unsafe {
                *self.entry = TTEntry {
                    tag: 0x80,
                    key_low16: self.hash as u16,
                    depth,
                    best_move,
                    lower_bound,
                    upper_bound,
                }
            }
        }
    }
}

impl TTEntry {
    /// A stand-in value for the depth to which captures only are searched.
    pub const DEPTH_CAPTURES: i8 = -1;
}

#[cfg(test)]
mod tests {
    use fiddler_base::{Move, Square};

    use super::*;

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
            tag: 0x80,
            key_low16: 12,
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
            tag: 0x80,
            key_low16: 12,
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
    /// Test that an entry with the same hash can overwrite another entry.
    fn overwrite() {
        let e0 = TTEntry {
            tag: 0x80,
            key_low16: 2022,
            depth: 5,
            best_move: Move::normal(Square::E2, Square::E4),
            lower_bound: Eval::DRAW,
            upper_bound: Eval::centipawns(100),
        };
        let e1 = TTEntry {
            tag: 0x80,
            key_low16: 2022,
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
            tag: 0x80,
            key_low16: 2022,
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

    #[test]
    /// Test that a `Bucket` is in fact the size of a cache line.
    fn bucket_size() {
        assert_eq!(size_of::<Bucket>(), 32);
    }
}
