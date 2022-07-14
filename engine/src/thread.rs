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
//! the engine: a `MainSearch` is responsible for corralling all the threads and
//! getting them to work together and on time. The main search also collects all
//! of the output from each individual search and composes it into a single
//! easily-used structure for consumption in the main process.

use std::{thread::scope, time::Instant};

use fiddler_base::Game;

use crate::uci::{EngineInfo, UciMessage};

use super::{
    config::SearchConfig,
    limit::SearchLimit,
    search::{search, SearchResult},
    transposition::TTable,
    SearchError,
};

#[derive(Debug)]
/// The primary search thread for an engine.
pub struct MainSearch {
    /// The configuration of the search, controlling the search parameters.
    pub config: SearchConfig,
    /// The transposition table, shared across all search threads.
    pub ttable: TTable,
    /// The limit to the search.
    pub limit: SearchLimit,
}

impl MainSearch {
    /// Construct a new main search with only a single search thread.
    pub fn new() -> MainSearch {
        MainSearch {
            config: SearchConfig::new(),
            ttable: TTable::with_capacity(25),
            limit: SearchLimit::new(),
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
        let tic = Instant::now();
        let mut best_result = Err(SearchError::Timeout);
        scope(|s| {
            for depth in 1..=self.config.depth {
                // iterative deepening

                let mut handles = Vec::new();

                for _thread_id in 0..self.config.n_helpers {
                    let gcopy = g.clone();
                    handles.push(s.spawn(move || {
                        search(gcopy, depth, &self.ttable, &self.config, &self.limit, false)
                    }));
                }

                // now it's our turn to think
                let mut sub_result = search(
                    g.clone(),
                    depth,
                    &self.ttable,
                    &self.config,
                    &self.limit,
                    true,
                );

                for handle in handles {
                    let eval_result = handle.join().map_err(|_| SearchError::Join)?;

                    match (&mut sub_result, &eval_result) {
                        // if this is our first successful thread, use its result
                        (Err(_), Ok(_)) => sub_result = eval_result.clone(),
                        // if both were successful, use the deepest result
                        (Ok(ref mut best_search), Ok(ref new_search)) => {
                            best_search.unify_with(new_search);
                        }
                        _ => (),
                        // error cases cause nothing to happen
                    };
                }

                if sub_result.is_ok() {
                    // update best result and inform GUI
                    best_result = sub_result;
                    let elapsed = Instant::now() - tic;
                    if let Ok(ref best_info) = best_result {
                        println!(
                            "{}",
                            UciMessage::Info(&[
                                EngineInfo::Depth(best_info.depth),
                                EngineInfo::Score {
                                    eval: best_info.eval,
                                    is_lower_bound: false,
                                    is_upper_bound: false
                                },
                                EngineInfo::Nodes(best_info.num_nodes_evaluated),
                                EngineInfo::NodeSpeed(
                                    1000 * best_info.num_nodes_evaluated
                                        / (elapsed.as_millis() + 1) as u64
                                ),
                                EngineInfo::Time(elapsed),
                                EngineInfo::Pv(&best_info.pv),
                                EngineInfo::HashFull(self.ttable.fill_rate_permill()),
                            ])
                        );
                        if best_info.eval.is_mate() {
                            // don't bother searching deeper if we already found 
                            // mate
                            break;
                        }
                    }
                }
            }

            if let Ok(ref mut info) = best_result {
                // normalize evaluation to be in absolute terms
                info.eval = info.eval.in_perspective(g.board().player_to_move);
            }
            best_result
        })
    }
}

impl Default for MainSearch {
    fn default() -> Self {
        MainSearch::new()
    }
}

#[cfg(test)]
mod tests {

    use fiddler_base::{movegen::is_legal, Score};

    use crate::evaluate::static_evaluate;

    use super::*;

    fn search_helper(fen: &str, depth: u8) {
        let mut g = Game::from_fen(fen, static_evaluate).unwrap();
        let mut main = MainSearch::new();
        main.config.n_helpers = 15;
        main.config.depth = depth;
        let info = main.evaluate(&g).unwrap();
        for m in info.pv {
            assert!(is_legal(m, g.position()));
            g.make_move(m, Score::centipawns(0, 0));
        }
    }

    #[test]
    fn search_opening() {
        search_helper(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            5,
        );
    }

    #[test]
    fn search_fried_liver() {
        search_helper(
            "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
            7,
        );
    }
}
