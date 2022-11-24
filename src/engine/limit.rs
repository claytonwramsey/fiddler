/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

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

//! Search limiting.
//!
//! It makes little sense to wait until a search decides that it's done, as it's hard to predict
//! when said search will be done.
//! The code in here is used to create limits to how long we can search, so that we don't have to
//! wait forever.
//!
//! A search limit is an inherently concurrent structure - much care must be taken to avoid
//! deadlocking.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex, RwLock,
    },
    time::{Duration, Instant},
};

use super::SearchError;

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
/// A limit to how long an engine should search for.
pub struct SearchLimit {
    /// Whether the search is truly over.
    over: AtomicBool,
    /// The cumulative number of nodes which have been searched since the first call to `start`.
    num_nodes: AtomicU64,
    /// A cap on the total number of nodes to search.
    /// If the cap is `None`, then there is no limit to the number of nodes to search.
    pub nodes_cap: RwLock<Option<u64>>,
    /// The time at which the search was started.
    start_time: Mutex<Instant>,
    /// The time at which the search will end.
    /// Will be `None` if the search is untimed.
    end_time: RwLock<Option<Instant>>,
    /// The duration of the search.
    /// If the duration is `None`, then there is no limit to the duration of the search.
    pub search_duration: Mutex<Option<Duration>>,
}

impl SearchLimit {
    #[must_use]
    /// Create a new `SearchLimit` which will never stop.
    pub fn new() -> SearchLimit {
        SearchLimit {
            over: AtomicBool::new(false),
            num_nodes: AtomicU64::new(0),
            nodes_cap: RwLock::new(None),
            start_time: Mutex::new(Instant::now()),
            end_time: RwLock::new(None),
            search_duration: Mutex::new(None),
        }
    }

    /// Start the search limit by setting its start time to now.
    ///
    /// # Errors
    ///
    /// This function will return an error if a lock was poisoned.
    pub fn start(&self) -> Result<(), SearchError> {
        self.num_nodes.store(0, Ordering::Relaxed);
        self.over.store(false, Ordering::Relaxed);
        *self.start_time.lock().map_err(|_| SearchError::Poison)? = Instant::now();
        let opt_duration = self
            .search_duration
            .lock()
            .map_err(|_| SearchError::Poison)?;
        if let Some(dur) = *opt_duration {
            *self.end_time.write().map_err(|_| SearchError::Poison)? = Some(Instant::now() + dur);
        };
        Ok(())
    }

    /// Immediately halt the search, and mark this current search as over.
    pub fn stop(&self) {
        self.over.store(true, Ordering::Relaxed);
    }

    #[inline(always)]
    /// Poll whether the search is over.
    pub fn is_over(&self) -> bool {
        self.over.load(Ordering::Relaxed)
    }

    #[inline(always)]
    /// Check the elapsed time to see if this search is over and if so update
    /// accordingly.
    ///
    /// # Errors
    ///
    /// This function will return an error if a lock was poisoned.
    pub fn update_time(&self) -> Result<bool, SearchError> {
        if let Some(end) = *self.end_time.read().map_err(|_| SearchError::Poison)? {
            if Instant::now() > end {
                self.over.store(true, Ordering::Relaxed);
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[inline(always)]
    /// Increment the total number of nodes searched.
    ///
    /// # Errors
    ///
    /// This function will return an error if a lock was poisoned.
    pub fn add_nodes(&self, nodes: u64) -> Result<(), SearchError> {
        self.num_nodes.fetch_add(nodes, Ordering::Relaxed);
        if let Some(max_nodes) = *self.nodes_cap.read()? {
            if self.num_nodes.load(Ordering::Relaxed) > max_nodes {
                self.over.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    #[inline(always)]
    /// Get the cumulative number of nodes searched.
    pub fn num_nodes(&self) -> u64 {
        self.num_nodes.load(Ordering::Relaxed)
    }
}

impl Default for SearchLimit {
    /// Construct a `SearchLimit` which will never stop.
    fn default() -> Self {
        SearchLimit::new()
    }
}
