use std::collections::hash_map::DefaultHasher;
use std::ops::Index;
use std::hash::{Hasher, Hash};

use crate::base::Board;
use crate::engine::Eval;

/**
 * Convenient bad-key value which may help with debugging.
 */
const BAD_HASH: u64 = 0x00000000DEADBEEF;

#[derive(Clone, Debug, PartialEq, Eq)]
/**
 *  A table which stores transposition data. It will automatically evict an
 *  "old" element if another one takes its place. It behaves much like a
 *  hash-map from positions to table-entries.
*/
pub struct TTable {
    /**
     * Sentinel `None` value that we return a pointer to in case we have a hash 
     * match but not a key-value match
     */
    sentinel: Option<EvalData>,
    entries: Vec<TTableEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvalData {
    /**
     * The depth to which the position was evaluated.
     */
    pub depth: i8,
    /**
     * A lower bound on the evaluation of the position.
     */
    pub lower_bound: Eval,
    /**
     * An upper bound on the evaluation of the position.
     */
    pub upper_bound: Eval,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/**
 *  An entry in the transposition table.
 */
struct TTableEntry {
    /**
     *  The hash which caused this entry. Used as a speedy way to avoid
     *  comparing a whole board.
     */
    pub hash: u64,
    /**
     *  The board with this evaluation.
     */
    pub key: Board,
    /**
     * The transposition data.
     */
    pub data: Option<EvalData>,
}

impl TTable {

    pub fn with_capacity(capacity: usize) -> TTable {
        TTable {
            sentinel: None,
            entries: vec![TTableEntry {
                hash: BAD_HASH,
                key: Board::BAD_BOARD,
                data: None
            }; capacity],
        }
    }

    pub fn store(&mut self, key: Board, value: EvalData) {
        
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        let index = hash as usize % self.entries.len();
        self.entries[index] = TTableEntry{
            hash: hash,
            key: key,
            data: Some(value),
        };
    }
}

impl Default for TTable {
    fn default() -> TTable {
        TTable::with_capacity(1 << 16)
    }
}

impl Index<&Board> for TTable {
    type Output = Option<EvalData>;

    fn index(&self, key: &Board) -> &Self::Output {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let key_hash = hasher.finish();
        let index = key_hash as usize & self.entries.len();
        let entry = &self.entries[index];

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
        };
        ttable.store(b, data);
        data.upper_bound = Eval(4);
        ttable.store(b, data);
        assert_eq!(ttable[&b], Some(data));

    }
}