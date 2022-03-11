use std::ops::Index;

use crate::base::Board;
use crate::base::Move;
use crate::engine::Eval;

///
/// Convenient bad-key value which may help with debugging.
///
const BAD_HASH: u64 = 0xDEADBEEF;

#[derive(Clone, Debug, PartialEq, Eq)]
///
/// A table which stores transposition data. It will automatically evict an
/// "old" element if another one takes its place. It behaves much like a
/// hash-map from positions to table-entries.
///
pub struct TTable {
    ///
    /// Sentinel `None` value that we return a pointer to in case we have a hash
    /// match but not a key-value match
    ///
    sentinel: Option<EvalData>,
    ///
    /// List of all entries in the transposition table.
    ///
    entries: Vec<TTableEntry>,
    ///
    /// Number of occupied slots. Since each entry has two slots (for most 
    /// recent and deepest), this can be at most double the length of `entries`.
    /// 
    occupancy: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
///
/// A struct containing information about prior evaluation of a position.
///
pub struct EvalData {
    ///
    /// The depth to which the position was evaluated.
    ///
    pub depth: i8,
    ///
    /// A lower bound on the evaluation of the position.
    ///
    pub lower_bound: Eval,
    ///
    /// An upper bound on the evaluation of the position.
    ///
    pub upper_bound: Eval,
    ///
    /// The critical move in the position. Will be `Move::BAD_MOVE` if the
    /// critical move is unknown.
    ///
    pub critical_move: Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
///
///  An entry in the transposition table.
///
struct TTableEntry {
    ///
    /// The most recently entered data in this entry.
    ///
    pub recent: Slot,
    ///
    /// The data with the deepest evaluation ever requested in this entry.
    /// 
    pub deepest: Slot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    ///
    /// The hash which caused this entry. Used as a speedy way to avoid
    /// comparing a whole board.
    ///
    pub hash: u64,
    ///
    /// The data for this slot.
    /// 
    pub data: Option<EvalData>,
}

impl TTable {
    ///
    /// Create a transposition table with a fixed capacity.
    ///
    pub fn with_capacity(capacity: usize) -> TTable {
        TTable {
            sentinel: None,
            entries: vec![
                TTableEntry {
                    recent: Slot { 
                        hash: BAD_HASH, 
                        data: None
                    },
                    deepest: Slot { 
                        hash: BAD_HASH, 
                        data: None
                    },
                };
                capacity
            ],
            occupancy: 0,
        }
    }

    ///
    /// Store some evaluation data in the transposition table.
    ///
    pub fn store(&mut self, key: Board, value: EvalData) {
        let index = key.hash as usize % self.entries.len();
        let mut entry = unsafe {
            // We trust that this is safe since we modulo'd by the length.
            self.entries.get_unchecked_mut(index)
        };
        let new_slot = Slot{
            hash: key.hash,
            data: Some(value)
        };
        let overwrite_deepest = match entry.deepest.data {
            Some(data) => value.depth >= data.depth,
            None => {
                // increment occupancy because we are overwriting None
                self.occupancy += 1;
                true
            },
        };
        if overwrite_deepest {
            entry.deepest = new_slot;
        }
        if entry.recent.data.is_none() {
            self.occupancy += 1;
        }
        entry.recent = new_slot;
    }

    ///
    /// Clear the transposition table. Will *not* lose any capacity.
    ///
    pub fn clear(&mut self) {
        self.entries = self
            .entries
            .iter()
            .map(|_| TTableEntry {
                recent: Slot { 
                    hash: BAD_HASH, 
                    data: None 
                },
                deepest: Slot { 
                    hash: BAD_HASH, 
                    data: None 
                },
            })
            .collect();
        self.occupancy = 0;
    }

    #[inline]
    ///
    /// Get the fill proportion of this transposition table. The fill 
    /// proportion is 0 for an empty table and 1 for a completely full one.
    /// 
    pub fn fill_rate(&self) -> f32 {
        (self.occupancy as f32) / (2. * self.entries.len() as f32)
    }

    #[inline]
    ///
    /// Get the fill rate proportion of this transposition table out of 1000. 
    /// Typically used for UCI.
    /// 
    pub fn fill_rate_permill(&self) -> u16 {
        (self.occupancy * 500 / self.entries.len()) as u16
    }
}

impl Default for TTable {
    fn default() -> TTable {
        TTable::with_capacity(1 << 20)
    }
}

impl Index<&Board> for TTable {
    type Output = Option<EvalData>;

    fn index(&self, key: &Board) -> &Self::Output {
        let index = key.hash as usize % self.entries.len();
        let entry = unsafe {
            // We trust that this will not lead to a memory error because index
            // was modulo'd by the length of entries.
            self.entries.get_unchecked(index)
        };

        if entry.deepest.hash == key.hash {
            return &entry.deepest.data;
        }

        if entry.recent.hash == key.hash {
            return &entry.recent.data;
        }

        /*
        // Although this line is theoretically needed, in practice, there are
        // essentially no Zobrist hash collisions. We skip this step to save
        // speed. A collision here would however be a logic error.

        // Since the hashes matched, these positions are likely equal.
        // Check whether they're truly equal.
        if *key != entry.key {
            println!("true zobrist collision!");
            return &self.sentinel;
        }
        */

        &self.sentinel
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
