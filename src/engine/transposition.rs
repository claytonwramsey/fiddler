use std::ops::Index;

use crate::base::Board;
use crate::base::Move;
use crate::engine::Eval;

/// Convenient bad-key value which may help with debugging.
const BAD_HASH: u64 = 0xDEADBEEF;

#[derive(Clone, Debug, PartialEq, Eq)]
/// A table which stores transposition data. It will automatically evict an
/// "old" element if another one takes its place. It behaves much like a
/// hash-map from positions to table-entries.
pub struct TTable {
    /// Sentinel `None` value that we return a pointer to in case we have a hash
    /// match but not a key-value match.
    sentinel: Option<EvalData>,

    /// List of all entries in the transposition table.
    entries: Vec<TTableEntry>,

    /// Number of occupied slots. Since each entry has two slots (for most
    /// recent and deepest), this can be at most double the length of `entries`.
    occupancy: usize,
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
        TTable {
            sentinel: None,
            entries: vec![
                TTableEntry {
                    recent: Slot::EMPTY,
                    deepest: Slot::EMPTY,
                };
                capacity
            ],
            occupancy: 0,
        }
    }

    /// Age up all the entries in this table, and for any slot which is at
    /// least as old as the max age, evict it.
    pub fn age_up(&mut self, max_age: u8) {
        for entry in self.entries.iter_mut() {
            for slot in [&mut entry.recent, &mut entry.deepest] {
                if slot.data.is_some() {
                    slot.age += 1;
                    if slot.age >= max_age {
                        self.occupancy -= 1;
                        *slot = Slot::EMPTY;
                    }
                }
            }
        }
    }

    /// Store some evaluation data in the transposition table.
    pub fn store(&mut self, key: Board, value: EvalData) {
        let index = key.hash as usize % self.entries.len();
        let mut entry = unsafe {
            // We trust that this is safe since we modulo'd by the length.
            self.entries.get_unchecked_mut(index)
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
                self.occupancy += 1;
                true
            }
        };
        if overwrite_deepest {
            entry.deepest = new_slot;
        }
        if entry.recent.data.is_none() {
            self.occupancy += 1;
        }
        entry.recent = new_slot;
    }

    /// Get the evaluation data stored by this table for a given key, if it
    /// exists. Returns `&None` if no such key exists.
    pub fn get(&self, hash_key: u64) -> &Option<EvalData> {
        let index = hash_key as usize % self.entries.len();
        let entry = unsafe {
            // We trust that this will not lead to a memory error because index
            // was modulo'd by the length of entries.
            self.entries.get_unchecked(index)
        };

        // Theoretically, we would need to check for key equality (such as the
        // original board) here, but in practice key collisions are so rare
        // that the extra performance loss makes it not worth it.

        if entry.deepest.hash == hash_key {
            return &entry.deepest.data;
        }

        if entry.recent.hash == hash_key {
            return &entry.recent.data;
        }

        &self.sentinel
    }

    /// Clear the transposition table. Will *not* lose any capacity.
    pub fn clear(&mut self) {
        self.entries = self
            .entries
            .iter()
            .map(|_| TTableEntry {
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
            .collect();
        self.occupancy = 0;
    }

    #[inline]
    /// Get the fill proportion of this transposition table. The fill
    /// proportion is 0 for an empty table and 1 for a completely full one.
    pub fn fill_rate(&self) -> f32 {
        (self.occupancy as f32) / (2. * self.entries.len() as f32)
    }

    #[inline]
    /// Get the fill rate proportion of this transposition table out of 1000.
    /// Typically used for UCI.
    pub fn fill_rate_permill(&self) -> u16 {
        (self.occupancy * 500 / self.entries.len()) as u16
    }
}

impl Default for TTable {
    fn default() -> TTable {
        TTable::with_capacity(1 << 18)
    }
}

impl Index<&Board> for TTable {
    type Output = Option<EvalData>;

    #[inline]
    fn index(&self, key: &Board) -> &Self::Output {
        self.get(key.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_already_in() {
        let ttable = TTable::default();
        assert!(ttable[&Board::default()].is_none());
    }

    #[test]
    fn test_store_data() {
        let mut ttable = TTable::default();
        let b = Board::default();
        let data = EvalData {
            depth: 0,
            upper_bound: Eval(0),
            lower_bound: Eval(0),
            critical_move: Move::BAD_MOVE,
        };
        ttable.store(b, data);
        assert_eq!(ttable[&b], Some(data));
    }
}
