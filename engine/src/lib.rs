use fiddler_base::{Eval, Move};
use search::SearchError;

mod candidacy;
mod config;
pub mod evaluate;
pub mod limit;
pub mod material;
mod pick;
pub mod pst;
mod search;
pub mod thread;
pub mod time;
mod transposition;

/// UCI-compliant parser and data structures.
pub mod uci;

#[allow(unused)]
/// Compute the effective branch factor given a given search depth and a number
/// of nodes evaluated.
fn branch_factor(depth: u8, num_nodes: u64) -> f64 {
    (num_nodes as f64).powf(1f64 / (depth as f64))
}
