use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::RwLock;

use crate::base::Board;
use crate::base::Eval;
use crate::base::Move;

/// Convenient bad-key value which may help with debugging.
const BAD_HASH: u64 = 0xDEADBEEF;

/// The ordering in which total occupancy in the transposition table should be
/// updated.
const OCCUPANCY_ORDERING: Ordering = Ordering::Relaxed;

#[derive(Debug)]
/// A table which stores transposition data. It will automatically evict an
/// "old" element if another one takes its place. It behaves much like a
/// hash-map from positions to table-entries.
pub struct TTable {
    /// List of all entries in the transposition table.
    entries: Vec<RwLock<TTableEntry>>,

    /// Number of occupied slots. Since each entry has two slots (for most
    /// recent and deepest), this can be at most double the length of `entries`.
    occupancy: AtomicUsize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A struct containing information about prior evaluation of a position.
pub struct EvalData {
    /// The depth to which the position was evaluated.
    pub depth: i8,

    /// A lower bound on the evaluation of the position.
    pub lower_bound: Eval,

    /// An upper bound on the evaluation of the position.
    pub upper_bound: Eval,

    /// The critical move in the position. Will be `Move::BAD_MOVE` if the
    /// critical move is unknown.
    pub critical_move: Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
///  An entry in the transposition table.
struct TTableEntry {
    /// The most recently entered data in this entry.
    pub recent: Slot,

    /// The data with the deepest evaluation ever requested in this entry.
    pub deepest: Slot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    /// The hash which caused this entry. Used as a speedy way to avoid
    /// comparing a whole board.
    pub hash: u64,

    /// The age of this slot. Initialized to zero, and then incremented every
    /// time the transposition table is aged up.
    pub age: u8,

    /// The data for this slot.
    pub data: Option<EvalData>,
}

impl Slot {
    pub const EMPTY: Slot = Slot {
        hash: BAD_HASH,
        age: 0,
        data: None,
    };
}

impl TTable {
    /// Create a transposition table with a fixed capacity.
    pub fn with_capacity(capacity: usize) -> TTable {
        let mut table = TTable {
            entries: Vec::new(),
            occupancy: AtomicUsize::new(0),
        };

        for _ in 0..capacity {
            table.entries.push(RwLock::new(TTableEntry {
                recent: Slot::EMPTY,
                deepest: Slot::EMPTY,
            }));
        }

        table
    }

    /// Age up all the entries in this table, and for any slot which is at
    /// least as old as the max age, evict it.
    pub fn age_up(&mut self, max_age: u8) {
        for lock in self.entries.iter_mut() {
            let mut entry = lock.write().unwrap();
            // do not alter the most recent one since it will be overwritten
            // anyway if needed
            if entry.deepest.data.is_some() {
                entry.deepest.age += 1;
                if entry.deepest.age >= max_age {
                    self.occupancy.fetch_sub(1, OCCUPANCY_ORDERING);
                    entry.deepest = Slot::EMPTY;
                }
            }
        }
    }

    /// Store some evaluation data in the transposition table.
    pub fn store(&self, key: Board, value: EvalData) {
        let index = key.hash as usize % self.entries.len();
        let mut entry = unsafe {
            // We trust that this is safe since we modulo'd by the length.
            self.entries.get_unchecked(index).write().unwrap()
        };
        let new_slot = Slot {
            hash: key.hash,
            age: 0,
            data: Some(value),
        };
        let overwrite_deepest = match entry.deepest.data {
            Some(data) => value.depth >= data.depth,
            None => {
                // increment occupancy because we are overwriting None
                self.occupancy.fetch_add(1, OCCUPANCY_ORDERING);
                true
            }
        };
        if overwrite_deepest {
            entry.deepest = new_slot;
        }
        if entry.recent.data.is_none() {
            self.occupancy.fetch_add(1, OCCUPANCY_ORDERING);
        }
        entry.recent = new_slot;
    }

    /// Get the evaluation data stored by this table for a given key, if it
    /// exists. Returns `&None` if no such key exists.
    pub fn get(&self, hash_key: u64) -> Option<EvalData> {
        let index = hash_key as usize % self.entries.len();
        let entry = unsafe {
            // We trust that this will not lead to a memory error because index
            // was modulo'd by the length of entries.
            self.entries.get_unchecked(index).read().unwrap()
        };

        // Theoretically, we would need to check for key equality (such as the
        // original board) here, but in practice key collisions are so rare
        // that the extra performance loss makes it not worth it.

        if entry.deepest.hash == hash_key {
            return entry.deepest.data; //&entry.deepest.data;
        }

        if entry.recent.hash == hash_key {
            return entry.recent.data;
        }

        None
    }

    /// Clear the transposition table. Will *not* lose any capacity.
    pub fn clear(&mut self) {
        self.entries = self
            .entries
            .iter()
            .map(|_| {
                RwLock::new(TTableEntry {
                    recent: Slot {
                        hash: BAD_HASH,
                        age: 0,
                        data: None,
                    },
                    deepest: Slot {
                        hash: BAD_HASH,
                        age: 0,
                        data: None,
                    },
                })
            })
            .collect();
        self.occupancy.store(0, OCCUPANCY_ORDERING);
    }

    #[inline]
    /// Get the fill proportion of this transposition table. The fill
    /// proportion is 0 for an empty table and 1 for a completely full one.
    pub fn fill_rate(&self) -> f32 {
        (self.occupancy.load(OCCUPANCY_ORDERING) as f32) / (2. * self.entries.len() as f32)
    }

    #[inline]
    /// Get the fill rate proportion of this transposition table out of 1000.
    /// Typically used for UCI.
    pub fn fill_rate_permill(&self) -> u16 {
        (self.occupancy.load(OCCUPANCY_ORDERING) * 500 / self.entries.len()) as u16
    }
}

impl Default for TTable {
    fn default() -> TTable {
        TTable::with_capacity(1 << 18)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_already_in() {
        let ttable = TTable::default();
        assert!(ttable.get(Board::default().hash).is_none());
    }

    #[test]
    fn test_store_data() {
        let ttable = TTable::default();
        let b = Board::default();
        let data = EvalData {
            depth: 0,
            upper_bound: Eval::DRAW,
            lower_bound: Eval::DRAW,
            critical_move: Move::BAD_MOVE,
        };
        ttable.store(b, data);
        assert_eq!(ttable.get(b.hash), Some(data));
    }
}
