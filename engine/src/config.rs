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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Configuration options for a search.
pub struct SearchConfig {
    /// The depth at which this algorithm will evaluate a position.
    pub depth: u8,
    /// The number of helper threads. If this value is 0, then the search is
    /// single-threaded.
    pub n_helpers: u8,
    /// The maximum depth to which the engine will add or edit entries in the
    /// transposition table.
    pub max_transposition_depth: u8,
    /// The number of moves at each layer which will be searched to a full
    /// depth, as opposed to a lower-than-target depth.
    pub num_early_moves: usize,
    /// The number of nodes which have to be searched before it is worthwhile
    /// to update the search limit with this information.
    pub limit_update_increment: u64,
}

impl SearchConfig {
    pub fn new() -> SearchConfig {
        SearchConfig {
            depth: 10,
            n_helpers: 0,
            max_transposition_depth: 7,
            num_early_moves: 4,
            limit_update_increment: 100,
        }
    }
}

impl Default for SearchConfig {
    fn default() -> SearchConfig {
        SearchConfig::new()
    }
}
