use std::hash::{Hash, Hasher};
use std::ops::Index;
use nohash_hasher::NoHashHasher;

use crate::base::Board;
use crate::base::Move;
use crate::engine::Eval;

///
/// Convenient bad-key value which may help with debugging.
///
const BAD_HASH: u64 = 0x00000000DEADBEEF;

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
    /// The hash which caused this entry. Used as a speedy way to avoid
    /// comparing a whole board.
    ///
    pub hash: u64,
    ///
    /// The board with this evaluation.
    ///
    pub key: Board,
    ///
    /// The transposition data.
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
                    hash: BAD_HASH,
                    key: Board::BAD_BOARD,
                    data: None
                };
                capacity
            ],
        }
    }

    ///
    /// Store some evaluation data in the transposition table.
    ///
    pub fn store(&mut self, key: Board, value: EvalData) {
        let mut hasher = NoHashHasher::<u64>::default();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let index = hash as usize % self.entries.len();
        unsafe {
            // We trust that this will not lead to an out of bounds as the
            // index has been modulo'd by the length of the entry table.
            *self.entries.get_unchecked_mut(index) = TTableEntry {
                hash,
                key,
                data: Some(value),
            };
        }
    }

    ///
    /// Clear the transposition table. Will *not* lose any capacity.
    ///
    pub fn clear(&mut self) {
        self.entries = self
            .entries
            .iter()
            .map(|_| TTableEntry {
                hash: BAD_HASH,
                key: Board::BAD_BOARD,
                data: None,
            })
            .collect();
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
        let mut hasher = NoHashHasher::<u64>::default();
        key.hash(&mut hasher);
        let key_hash = hasher.finish();
        let index = key_hash as usize % self.entries.len();
        let entry = unsafe {
            // We trust that this will not lead to a memory error because index
            // was modulo'd by the length of entries.
            self.entries.get_unchecked(index)
        };

        // First, compare hashes to "fast-track" checking if these
        // positions are truly equal.
        if entry.hash != key_hash {
            return &self.sentinel;
        }

        // Since the hashes matched, these positions are likely equal.
        // Check whether they're truly equal.
        if *key != entry.key {
            return &self.sentinel;
        }

        &entry.data
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

    #[test]
    fn test_eviction() {
        let mut ttable = TTable::with_capacity(1);
        let b = Board::default();
        let mut data = EvalData {
            depth: 0,
            upper_bound: Eval(0),
            lower_bound: Eval(0),
            critical_move: Move::BAD_MOVE,
        };
        ttable.store(b, data);
        data.upper_bound = Eval(4);
        ttable.store(b, data);
        assert_eq!(ttable[&b], Some(data));
    }
}
