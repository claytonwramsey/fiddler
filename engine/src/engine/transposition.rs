use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::base::Eval;
use crate::base::Move;

/// Convenient bad-key value which may help with debugging.
const BAD_HASH: u64 = 0xDEADBEEF;

#[derive(Debug)]
/// A table which stores transposition data. It will automatically evict an
/// "old" element if another one takes its place. It behaves much like a
/// hash-map from positions to table-entries.
pub struct TTable {
    /// List of all entries in the transposition table.
    entries: Vec<TTableEntry>,

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

#[derive(Debug)]
///  An entry in the transposition table.
struct TTableEntry {
    /// The most recently entered data in this entry.
    pub recent: Slot,

    /// The data with the deepest evaluation ever requested in this entry.
    pub deepest: Slot,
}

#[derive(Debug)]
struct Slot {
    /// The hash which caused this entry. Used as a speedy way to avoid
    /// comparing a whole board. The hash has been xor'd with the data to allow
    /// for lockless access. Will be equal to `BAD_HASH` for empty entries.
    pub hash: AtomicU64,

    /// The corresponding data for this slot, including its age.
    pub data: AtomicU64,
}

impl Slot {
    fn empty() -> Slot {
        Slot {
            hash: AtomicU64::new(BAD_HASH),
            data: AtomicU64::new(0),
        }
    }
}

impl TTable {
    /// Create a transposition table with a fixed capacity.
    pub fn with_capacity(capacity: usize) -> TTable {
        let mut table = TTable {
            entries: Vec::new(),
            occupancy: AtomicUsize::new(0),
        };

        for _ in 0..capacity {
            table.entries.push(TTableEntry {
                recent: Slot::empty(),
                deepest: Slot::empty(),
            });
        }

        table
    }

    /// Age up all the entries in this table, and for any slot which is at
    /// least as old as the max age, evict it.
    pub fn age_up(&mut self, max_age: u8) {
        for entry in self.entries.iter_mut() {
            // no need to age up recent since it will be overwritten anyway
            let hash = entry.deepest.hash.load(Ordering::Relaxed);
            let datum = entry.deepest.data.load(Ordering::Relaxed);
            if hash != BAD_HASH {
                let new_age = unpack_age(datum) + 1;
                if new_age < max_age {
                    // the entry is not too old
                    let new_datum = (datum & 0x00FFFFFFFFFFFFFF) | ((new_age as u64) << 56);
                    let new_hash = hash ^ datum ^ new_datum;
                    entry.deepest.hash.store(new_hash, Ordering::Relaxed);
                    entry.deepest.data.store(new_datum, Ordering::Relaxed);
                } else {
                    // the entry is too old, evict it
                    entry.deepest.hash.store(BAD_HASH, Ordering::Relaxed);
                    entry.deepest.data.store(0, Ordering::Relaxed);
                    self.occupancy.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// Store some evaluation data in the transposition table.
    pub fn store(&self, key: u64, value: EvalData) {
        let index = key as usize % self.entries.len();
        let entry = unsafe {
            // We trust that this is safe since we modulo'd by the length.
            self.entries.get_unchecked(index)
        };
        let packed_data = pack(
            0,
            value.depth,
            value.upper_bound,
            value.lower_bound,
            value.critical_move,
        );
        let modulated_hash = key ^ packed_data;

        // check if we can overwrite the deepest entry
        // don't overwrite the most recent entry if we don't have to
        let slot_to_overwrite = {
            let deepest_hash = entry.deepest.hash.load(Ordering::Relaxed);
            if deepest_hash == BAD_HASH {
                // overwriting an empty entry, increment occupancy
                self.occupancy.fetch_add(1, Ordering::Relaxed);
                &entry.deepest
            } else {
                let deepest_datum = entry.deepest.data.load(Ordering::Relaxed);
                if unpack_depth(deepest_datum) <= value.depth {
                    &entry.deepest
                } else {
                    if entry.recent.hash.load(Ordering::Relaxed) == BAD_HASH {
                        // overwriting an empty entry, increment occupancy
                        self.occupancy.fetch_add(1, Ordering::Relaxed);
                    }
                    &entry.recent
                }
            }
        };

        slot_to_overwrite
            .hash
            .store(modulated_hash, Ordering::Relaxed);
        slot_to_overwrite.data.store(packed_data, Ordering::Relaxed);
    }

    /// Get the evaluation data stored by this table for a given key, if it
    /// exists. Returns `None` if no data corresponding to the key exists.
    pub fn get(&self, hash_key: u64) -> Option<EvalData> {
        let index = hash_key as usize % self.entries.len();
        let entry = unsafe {
            // We trust that this will not lead to a memory error because index
            // was modulo'd by the length of entries.
            self.entries.get_unchecked(index)
        };

        for slot in [&entry.deepest, &entry.recent] {
            let value = slot.data.load(Ordering::Relaxed);
            // check that the hash key would be stored as the same one which is
            // already stored
            // theoretically, this could result in an error due to hash
            // collision, but we accept that this is rare enough
            if value ^ hash_key == slot.hash.load(Ordering::Relaxed) {
                return Some(unpack_data(value));
            }
        }

        None
    }

    /// Clear the transposition table. Will *not* lose any capacity.
    pub fn clear(&mut self) {
        self.entries = self
            .entries
            .iter()
            .map(|_| TTableEntry {
                recent: Slot::empty(),
                deepest: Slot::empty(),
            })
            .collect();
        self.occupancy.store(0, Ordering::Relaxed);
    }

    #[inline]
    /// Get the fill proportion of this transposition table. The fill
    /// proportion is 0 for an empty table and 1 for a completely full one.
    pub fn fill_rate(&self) -> f32 {
        (self.occupancy.load(Ordering::Relaxed) as f32) / (2. * self.entries.len() as f32)
    }

    #[inline]
    /// Get the fill rate proportion of this transposition table out of 1000.
    /// Typically used for UCI.
    pub fn fill_rate_permill(&self) -> u16 {
        (self.occupancy.load(Ordering::Relaxed) * 500 / self.entries.len()) as u16
    }
}

impl Default for TTable {
    fn default() -> TTable {
        TTable::with_capacity(1 << 18)
    }
}

#[inline]
/// Given a packed entry in the transposition table, get the age of the entry.
const fn unpack_age(packed: u64) -> u8 {
    (packed >> 56) as u8
}

#[inline]
/// Given a packed entry in the transposition table, get the depth of the entry.
const fn unpack_depth(packed: u64) -> i8 {
    ((packed >> 48) & 0xFF) as i8
}

#[inline]
/// Unpack some data which was stored in the transposition table.
const fn unpack_data(packed: u64) -> EvalData {
    EvalData {
        depth: unpack_depth(packed),
        lower_bound: Eval::centipawns(((packed >> 32) & 0xFFFF) as i16),
        upper_bound: Eval::centipawns(((packed >> 16) & 0xFFFF) as i16),
        critical_move: Move::from_val((packed & 0xFFFF) as u16),
    }
}

#[inline]
/// Pack some data to be stored in the transposition table.
const fn pack(
    age: u8,
    depth: i8,
    upper_bound: Eval,
    lower_bound: Eval,
    critical_move: Move,
) -> u64 {
    (age as u64) << 56
        | (depth as u64) << 48
        | (upper_bound.centipawn_val() as u64) << 32
        | (lower_bound.centipawn_val() as u64) << 16
        | (critical_move.value() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Board;

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
        ttable.store(b.hash, data);
        assert_eq!(ttable.get(b.hash), Some(data));
    }
}
