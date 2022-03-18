#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Configuration options for a search.
pub struct SearchConfig {
    /// The depth at which this algorithm will evaluate a position.
    pub depth: u8,

    /// The maximum depth to which the engine will add or edit entries in the
    /// transposition table.
    pub max_transposition_depth: u8,

    /// The number of moves at each layer which will be searched to a full
    /// depth, as opposed to a lower-than-target depth.
    pub num_early_moves: u8,

    /// The number of nodes which have to be searched before it is worthwhile
    /// to update the search limit with this information.
    pub limit_update_increment: u64,
}

impl SearchConfig {
    pub fn new() -> SearchConfig {
        SearchConfig {
            depth: 10,
            max_transposition_depth: 8,
            num_early_moves: 4,
            limit_update_increment: 100,
        }
    }
}
