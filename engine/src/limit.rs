use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use super::SearchError;

#[derive(Debug)]
/// A limit to how long an engine should search for.
pub struct SearchLimit {
    /// Whether the search is truly over.
    over: AtomicBool,
    /// The cumulative number of nodes which have been searched since the first
    /// call to `start`.
    num_nodes: AtomicU64,
    /// A cap on the total number of nodes to search. If the cap is `None`,
    /// then there is no limit to the number of nodes to search.
    pub nodes_cap: Mutex<Option<u64>>,
    /// The time at which the search was started.
    start_time: Mutex<Instant>,
    /// The time at which the search will end. Will be `None` if the search is
    /// untimed.
    end_time: Mutex<Option<Instant>>,
    /// The duration of the search. If the duration is `None`, then there is
    /// no limit to the duration of the search.
    pub search_duration: Mutex<Option<Duration>>,
}

impl SearchLimit {
    /// Create a new `SearchLimit` which will never stop.
    pub fn new() -> SearchLimit {
        SearchLimit {
            over: AtomicBool::new(false),
            num_nodes: AtomicU64::new(0),
            nodes_cap: Mutex::new(None),
            start_time: Mutex::new(Instant::now()),
            end_time: Mutex::new(None),
            search_duration: Mutex::new(None),
        }
    }

    /// Start the search limit, by setting its start time to now.
    pub fn start(&self) -> Result<(), SearchError> {
        self.num_nodes.store(0, Ordering::Relaxed);
        self.over.store(false, Ordering::Relaxed);
        *self.start_time.lock().map_err(|_| SearchError::Poison)? = Instant::now();
        let opt_duration = self
            .search_duration
            .lock()
            .map_err(|_| SearchError::Poison)?;
        if let Some(dur) = *opt_duration {
            *self.end_time.lock().map_err(|_| SearchError::Poison)? = Some(Instant::now() + dur);
        };
        Ok(())
    }

    #[inline]
    /// Poll whether the search is over.
    pub fn is_over(&self) -> bool {
        self.over.load(Ordering::Relaxed)
    }

    #[inline]
    /// Check the elapsed time to see if this search is over, and if so, update
    /// accordingly.
    pub fn update_time(&self) -> Result<bool, SearchError> {
        if let Some(end) = *self.end_time.lock().map_err(|_| SearchError::Poison)? {
            if Instant::now() > end {
                self.over.store(true, Ordering::Relaxed);
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[inline]
    /// Increment the total number of nodes searched. If a lock acquisition 
    /// failure occurs, will return an error.
    pub fn add_nodes(&self, nodes: u64) -> Result<(), SearchError> {
        self.num_nodes.fetch_add(nodes, Ordering::Relaxed);
        if let Some(max_nodes) = *self.nodes_cap.lock()? {
            if self.num_nodes.load(Ordering::Relaxed) > max_nodes {
                self.over.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    #[inline]
    /// Get the cumulative number of nodes searched.
    pub fn num_nodes(&self) -> u64 {
        self.num_nodes.load(Ordering::Relaxed)
    }
}

impl Default for SearchLimit {
    fn default() -> Self {
        SearchLimit::new()
    }
}
