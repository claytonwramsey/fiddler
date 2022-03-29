use std::sync::PoisonError;

use crate::base::{Eval, Move};

pub mod candidacy;
pub mod config;
pub mod evaluate;
pub mod greedy;
pub mod limit;
pub mod pst;
pub mod search;
pub mod thread;
pub mod transposition;
pub mod pick;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The types of errors which can occur during a search.
pub enum SearchError {
    /// This search failed due to timeout.
    Timeout,
    /// This search failed because a lock was poisoned.
    Poison,
    /// This searched failed because a thread failed to join.
    Join,
}

impl<T> From<PoisonError<T>> for SearchError {
    #[inline]
    fn from(_: PoisonError<T>) -> Self {
        SearchError::Poison
    }
}

/// The result of performing a search. The `Ok` version is the tuple of (best
/// move, evalaution, depth), while the `Err` version contains a reason why the
/// search failed.
pub type SearchResult = Result<(Move, Eval, u8), SearchError>;

#[inline]
/// Compute the effective branch factor given a given search depth and a number
/// of nodes evaluated.
fn branch_factor(depth: u8, num_nodes: u64) -> f64 {
    (num_nodes as f64).powf(1f64 / (depth as f64))
}
