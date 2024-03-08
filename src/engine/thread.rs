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

//! Thread management and synchronization.
//!
//! This is the meat of parallelism in the engine: a `MainSearch` is responsible or corralling all
//! the threads and getting them to work together and on time.
//! The main search also collects all of the output from each individual search and composes it into
//! a single easily-used structure for consumption in the main process.

use std::{thread::scope, time::Instant};

use crate::base::game::Game;

use super::{evaluate::Eval, search::SearchInfo};

use super::{limit::SearchLimit, search::search, transposition::TTable};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Configuration options for a search.
pub struct SearchConfig {
    /// The depth at which this algorithm will evaluate a position.
    pub depth: u8,
    /// The number of helper threads.
    /// If this value is 0, then the search is single-threaded.
    pub n_helpers: u8,
    /// The number of nodes which have to be searched before it is worthwhile to update the search
    /// limit with this information.
    pub limit_update_increment: u64,
}

impl SearchConfig {
    #[must_use]
    /// Construct a new search configuration with some common default values.
    pub fn new() -> SearchConfig {
        SearchConfig {
            depth: 10,
            n_helpers: 0,
            limit_update_increment: 100,
        }
    }
}

impl Default for SearchConfig {
    fn default() -> SearchConfig {
        SearchConfig::new()
    }
}

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
    #[must_use]
    /// Construct a new main search with only a single search thread.
    pub fn new() -> MainSearch {
        MainSearch {
            config: SearchConfig::new(),
            ttable: TTable::with_size(0),
            limit: SearchLimit::infinite(),
        }
    }

    #[allow(clippy::result_unit_err)]
    /// Evaluate a position.
    /// The searcher will continue searching until its field `limit` marks itself as over.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if no search is completed in the allocated time.
    ///
    /// # Panics
    ///
    /// This function may panic if any of the search threads panic.
    pub fn evaluate(&self, g: &Game) -> Result<SearchInfo, ()> {
        let tic = Instant::now();
        let mut best_result = Err(());

        // The previous iteration's evaluation, used for windowing
        let mut prev_eval = None;
        scope(|s| {
            for depth in 1..=self.config.depth {
                // iterative deepening

                let mut handles = Vec::new();

                for _thread_id in 0..self.config.n_helpers {
                    handles
                        .push(s.spawn(move || self.aspiration_search(g, depth, false, prev_eval)));
                }

                // now it's our turn to think
                let mut sub_result = self.aspiration_search(g, depth, true, prev_eval);

                for handle in handles {
                    let eval_result = handle.join().unwrap();

                    match (&mut sub_result, &eval_result) {
                        // if this is our first successful thread, use its result
                        (Err(()), Ok(_)) => sub_result.clone_from(&eval_result),
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
                    let elapsed = tic.elapsed();
                    if let Ok(ref best_info) = best_result {
                        prev_eval = Some(best_info.eval);
                        print!(
                            "info depth {} seldepth {} score ",
                            best_info.depth, best_info.selective_depth
                        );
                        match best_info.eval.moves_to_mate() {
                            Some(n) => {
                                if best_info.eval > Eval::DRAW {
                                    print!("mate {n} ");
                                } else {
                                    print!("mate -{n} ");
                                }
                            }
                            None => print!("cp {} ", best_info.eval.centipawn_val()),
                        };
                        print!(
                            "nodes {} nps {} time {} hashfull {} pv",
                            best_info.num_nodes_evaluated,
                            1000 * u128::from(best_info.num_nodes_evaluated)
                                / (elapsed.as_millis() + 1),
                            elapsed.as_millis(),
                            self.ttable.fill_rate_permill(),
                        );
                        for m in &best_info.pv {
                            print!(" {}", m.to_uci());
                        }
                        println!();
                    }
                }
            }

            if let Ok(ref mut info) = best_result {
                // normalize evaluation to be in absolute terms
                info.eval = info.eval.in_perspective(g.meta().player);
            }
            best_result
        })
    }

    fn aspiration_search(
        &self,
        g: &Game,
        depth: u8,
        main: bool,
        prev_eval: Option<Eval>,
    ) -> Result<SearchInfo, ()> {
        if let Some(ev) = prev_eval {
            // we have a previous score we can use to window this search
            let (alpha, beta) = if ev.is_mate() {
                if ev < Eval::DRAW {
                    // we are getting mated.
                    // search for ways we can get mated faster
                    (Eval::MIN, ev + Eval::centipawns(1))
                } else {
                    // we are matting, search for faster wins
                    (ev - Eval::centipawns(1), Eval::MAX)
                }
            } else {
                match depth & 0x1u8 {
                    // even depth means that we expect the evaluation to decrease
                    0 => (ev - Eval::centipawns(100), ev + Eval::centipawns(10)),
                    // odd depth means that we expect the evaluation to increase
                    1 => (ev - Eval::centipawns(10), ev + Eval::centipawns(100)),
                    _ => unreachable!(),
                }
            };
            let window_result = search(
                g.clone(),
                depth,
                &self.ttable,
                &self.config,
                &self.limit,
                main,
                alpha,
                beta,
            );

            if let Ok(ref res) = window_result {
                if alpha < res.eval && res.eval < beta {
                    return window_result;
                }
            }
        }

        search(
            g.clone(),
            depth,
            &self.ttable,
            &self.config,
            &self.limit,
            main,
            Eval::MIN,
            Eval::MAX,
        )
    }
}

impl Default for MainSearch {
    fn default() -> Self {
        MainSearch::new()
    }
}

#[cfg(any(test, bench))]
mod tests {

    use crate::base::movegen::is_legal;

    use super::*;

    fn search_helper(fen: &str, depth: u8) {
        let mut g = Game::from_fen(fen).unwrap();
        let mut main = MainSearch::new();
        main.config.n_helpers = 0;
        main.config.depth = depth;
        main.ttable.resize(1000);
        let info = main.evaluate(&g).unwrap();
        for m in info.pv {
            assert!(is_legal(m, &g));
            g.make_move(m);
        }
    }

    #[test]
    fn search_opening() {
        search_helper(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            10,
        );
    }

    #[test]
    fn search_fried_liver() {
        search_helper(
            "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
            10,
        );
    }
}
