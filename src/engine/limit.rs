use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

/// A search limit which has been surrounded by a lock for thread safety.
pub type ArcLimit = Arc<RwLock<SearchLimit>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A limit to how long an engine should search for.
pub struct SearchLimit {
    /// The cumulative number of nodes which have been searched since the first
    /// call to `start`.
    num_nodes: u64,
    /// A cap on the total number of nodes to search. If the cap is `None`,
    /// then there is no limit to the number of nodes to search.
    pub nodes_cap: Option<u64>,
    /// The time at which the search was started.
    start_time: Instant,
    /// The durection of the search. If the duration is `None`, then there is
    /// no limit to the duration of the search.
    pub search_duration: Option<Duration>,
    /// Whether to force a stop immediately.
    force_stop: bool,
}

impl SearchLimit {
    /// Create a new `SearchLimit` which will never stop.
    pub fn new() -> SearchLimit {
        SearchLimit {
            num_nodes: 0,
            nodes_cap: None,
            start_time: Instant::now(),
            search_duration: None,
            force_stop: false,
        }
    }

    /// Start the search limit, by setting its start time to now.
    pub fn start(&mut self) {
        self.num_nodes = 0;
        self.start_time = Instant::now();
    }

    /// Poll whether the search is over.
    pub fn is_over(&self) -> bool {
        if let Some(cap) = self.nodes_cap {
            if self.num_nodes > cap {
                println!("over by number of nodes");
                return true;
            }
        }
        if let Some(duration) = self.search_duration {
            if Instant::now() - self.start_time > duration {
                return true;
            }
        }

        self.force_stop
    }

    /// Increment the total number of nodes searched.
    pub fn add_nodes(&mut self, nodes: u64) {
        self.num_nodes += nodes;
    }

    /// Get the cumulative number of nodes searched.
    pub fn num_nodes(&self) -> u64 {
        self.num_nodes
    }
}
