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

//! Thread management and synchronization.
//!
//! This is the meat of parallelism in
//! the engine: a `MainSearch` is responsible for coralling all the threads and
//! getting them to work together and on time. The main search also collects all
//! of the output from each individual search and composes it into a single
//! easily-used structure for consumption in the main process.

use std::{
    sync::Arc,
    thread::{spawn, JoinHandle},
    time::Instant,
};

use fiddler_base::Game;

use crate::uci::{EngineInfo, UciMessage};

use super::{
    config::SearchConfig,
    limit::SearchLimit,
    search::{search, SearchResult},
    transposition::TTable,
    SearchError,
};

#[derive(Clone, Debug)]
/// The primary search thread for an engine.
pub struct MainSearch {
    /// The configuration of the search, controlling the search parameters.
    pub config: SearchConfig,
    /// The transposition table, shared across all search threads.
    pub ttable: Arc<TTable>,
    /// The limit to the search.
    pub limit: Arc<SearchLimit>,
}

impl MainSearch {
    /// Construct a new main search with only a single search thread.
    pub fn new() -> MainSearch {
        MainSearch {
            config: SearchConfig::new(),
            ttable: Arc::new(TTable::default()),
            limit: Arc::new(SearchLimit::new()),
        }
    }

    /// Evaluate a position. The searcher will continue searching until its
    /// field `limit` marks itself as over.
    ///
    /// # Error
    ///
    /// An error will be returned according to the cases outlined in
    /// `SearchError`. Such errors are rare, and are generally either the result
    /// of an internal bug or a critical OS interrupt. However, a timeout error
    /// is most likely if the search times out before it can do any computation.
    pub fn evaluate(&self, g: &Game) -> SearchResult {
        self.ttable.age_up(2);
        let tic = Instant::now();
        let mut best_result = Err(SearchError::Timeout);
        for depth in 1..=self.config.depth {
            // iterative deepening
            
            let mut handles: Vec<JoinHandle<SearchResult>> = Vec::new();

            for _thread_id in 0..=self.config.n_helpers {
                let ttable_arc = self.ttable.clone();
                let limit_arc = self.limit.clone();
                let config_copy = self.config;
                let gcopy = g.clone();
                handles.push(spawn(move || {
                    search(gcopy, depth, ttable_arc, &config_copy, limit_arc, false)
                }));
            }

            // now it's our turn to think
            let mut sub_result = search(
                g.clone(),
                depth,
                self.ttable.clone(),
                &self.config,
                self.limit.clone(),
                true,
            );

            for handle in handles {
                let eval_result = handle.join().map_err(|_| SearchError::Join)?;

                match (sub_result, eval_result) {
                    // if this is our first successful thread, use its result
                    (Err(_), Ok(_)) => sub_result = eval_result,
                    // if both were successful, use the deepest result
                    (Ok(ref mut best_search), Ok(ref new_search)) => {
                        best_search.unify_with(new_search);
                    }
                    // error cases cause nothing to happen
                    _ => (),
                };
            }

            if let Ok(ref info) = sub_result {
                // update best result and inform GUI
                best_result = sub_result;
                if let Ok(ref mut best_info) = best_result {
                    best_info.unify_with(info);

                    let elapsed = Instant::now() - tic;
                    println!(
                        "{}",
                        UciMessage::Info(&[
                            EngineInfo::Depth(depth),
                            EngineInfo::Time(elapsed),
                            EngineInfo::Nodes(best_info.num_nodes_evaluated),
                            EngineInfo::NodeSpeed(
                                best_info.num_nodes_evaluated * 1000
                                    / (elapsed.as_millis() as u64 + 1)
                            ),
                            EngineInfo::HashFull(self.ttable.fill_rate_permill()),
                            EngineInfo::Score {
                                eval: best_info.eval,
                                is_lower_bound: false,
                                is_upper_bound: false
                            }
                        ])
                    );
                }
            }
        }

        if let Ok(ref mut info) = best_result {
            // normalize evaluation to be in absolute terms
            info.eval = info.eval.in_perspective(g.board().player_to_move);
        }
        best_result
    }
}

impl Default for MainSearch {
    fn default() -> Self {
        MainSearch::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{cmp::max, time::Instant};

    use crate::evaluate::static_evaluate;

    use super::*;

    /// Compare the speed of a search on a given transposition depth with its
    /// adjacent depths.
    fn transposition_speed_comparison(fen: &str, depth: u8, transposition_depth: u8, nhelpers: u8) {
        let g = Game::from_fen(fen, static_evaluate).unwrap();
        for tdepth in max(0, transposition_depth - 1)..=(transposition_depth + 1) {
            let mut main = MainSearch::new();
            main.config.depth = depth;
            main.config.n_helpers = nhelpers;
            main.config.max_transposition_depth = tdepth;

            let tic = Instant::now();
            main.evaluate(&g).unwrap();
            let toc = Instant::now();
            println!(
                "tdepth {tdepth}: {:.3}s, hashfill {:.3}",
                (toc - tic).as_secs_f32(),
                main.ttable.fill_rate()
            );
        }
    }

    #[test]
    fn transposition_speed_fried_liver() {
        transposition_speed_comparison(
            "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
            11,
            8,
            7,
        );
    }
}
