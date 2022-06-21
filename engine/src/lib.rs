/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

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

//! Code which defines the engine's behavior. Included below are methods for 
//! evaluating positions, searching trees, storing data, configuring engines, 
//! and more.

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
pub mod transposition;
pub mod uci;

#[allow(unused)]
/// Compute the effective branch factor given a given search depth and a number
/// of nodes evaluated.
fn branch_factor(depth: u8, num_nodes: u64) -> f64 {
    (num_nodes as f64).powf(1f64 / (depth as f64))
}
