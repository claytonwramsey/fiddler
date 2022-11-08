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

//! Primary search algorithms.
//!
//! All chess engines do some sort of tree searching, and as a classical engine,
//! Fiddler uses a variation of Minimax search.
//! In this case, Fiddler uses principal-variation search, which runs in
//! Omega(b^{d/2}) time, so long as the move ordering is correct and causes the
//! most critical moves to be searched first at each depth.
//!
//! At each leaf of the principal-variation search, a second, shorter quiescence
//! search is performed to exhaust all captures in the position, preventing the
//! mis-evaluation of positions with hanging pieces.

use crate::base::{
    movegen::{has_moves, is_legal, CAPTURES},
    Move,
};

use super::{
    evaluate::{Eval, ScoredGame},
    transposition::{TTEntry, TTEntryGuard},
};

use super::{
    evaluate::leaf_evaluate, limit::SearchLimit, pick::MovePicker,
    thread::SearchConfig, transposition::TTable,
};

use std::{cmp::max, sync::PoisonError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
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
    #[inline(always)]
    fn from(_: PoisonError<T>) -> Self {
        SearchError::Poison
    }
}

#[allow(clippy::module_name_repetitions)]
/// The result of performing a search.
/// The `Ok` version contains data on the search, while the `Err` version
/// contains a reason why the search failed.
pub type SearchResult = Result<SearchInfo, SearchError>;

#[allow(clippy::too_many_arguments, clippy::cast_possible_wrap)]
/// Evaluate the given game.
/// The evaluation will be from the player's perspective, i.e. inverted if the
/// player to move is Black.
///
/// Inputs:
/// * `g`: the game which will be evaluated.
/// * `ttable`: a reference to the shared transposition table.
/// * `config`: the configuration of this search.
/// * `limit`:the search limiter, which will be interiorly mutated by this
///     function.
/// * `is_main`: whether or not this search is the "main" search or a subjugate
///     thread, and determines responsibilities as such.
/// * `alpha`: a lower bound on the evaluation.
///     This is primarily intended to be used for aspiration windowing, and in
///     most cases will be set to `Eval::MIN`.
/// * `beta`: is an upper bound on the evaluation.
///     This is primarily intended to be used for aspiration windowing, and in
///     most cases will be set to `Eval::MAX`.
pub fn search(
    mut g: ScoredGame,
    depth: u8,
    ttable: &TTable,
    config: &SearchConfig,
    limit: &SearchLimit,
    is_main: bool,
    alpha: Eval,
    beta: Eval,
) -> SearchResult {
    g.start_search();
    let mut searcher = PVSearch::new(g, ttable, config, limit, is_main);
    let mut pv = Vec::new();

    let eval = searcher.pvs::<true, true, true>(
        depth as i8,
        0,
        alpha,
        beta,
        &mut pv,
    )?;

    Ok(SearchInfo {
        pv,
        eval,
        num_transpositions: searcher.num_transpositions,
        num_nodes_evaluated: searcher.num_nodes_evaluated,
        depth,
        selective_depth: searcher.selective_depth,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
/// Information about the search which will be returned at the end of a search.
pub struct SearchInfo {
    /// The principal variation.
    pub pv: Vec<Move>,
    /// The evaluation of the position.
    pub eval: Eval,
    /// The number of times a transposition table get was successful.
    pub num_transpositions: u64,
    /// The number of nodes evaluated in this search.
    pub num_nodes_evaluated: u64,
    /// The highest depth at which this search succeeded.
    pub depth: u8,
    /// The selective search depth, i.e. the highest depth to which any position
    /// was considered.
    pub selective_depth: u8,
}

impl SearchInfo {
    /// Unify with another `SearchInfo`, selecting the most accurate evaluation
    /// (by depth) and summing the number of transpositions and nodes evaluated.
    pub fn unify_with(&mut self, other: &SearchInfo) {
        let other_is_better = other.depth > self.depth
            || (other.depth == self.depth && other.pv.len() > self.pv.len());
        if other_is_better {
            self.pv = other.pv.clone();
            self.eval = other.eval;
            self.depth = other.depth;
        }
        self.selective_depth = max(self.selective_depth, other.selective_depth);
        self.num_nodes_evaluated += other.num_nodes_evaluated;
        self.num_transpositions += other.num_transpositions;
    }
}

#[derive(Clone, Debug)]
/// A structure containing data which is shared across function calls to a
/// principal variation search.
struct PVSearch<'a> {
    /// The game being searched.
    game: ScoredGame,
    /// The transposition table.
    ttable: &'a TTable,
    /// The set of "killer" moves. Each index corresponds to a depth (0 is most
    /// shallow, etc).
    killer_moves: Vec<Move>,
    /// The cumulative number of nodes evaluated in this evaluation.
    num_nodes_evaluated: u64,
    /// The cumulative number of nodes visited since we last updated the limit.
    nodes_since_limit_update: u16,
    /// The cumulative number of transpositions.
    num_transpositions: u64,
    /// The configuration of this search.
    config: &'a SearchConfig,
    /// The limit to this search.
    limit: &'a SearchLimit,
    /// Whether this search is the main search.
    is_main: bool,
    /// The highest depth to which any line was searched.
    selective_depth: u8,
}

impl<'a> PVSearch<'a> {
    /// Construct a new `PVSearch` using a given transposition table,
    /// configuration, and limit.
    /// `is_main` is whether the thread is a main search, responsible for
    /// certain synchronization activities.
    pub fn new(
        game: ScoredGame,
        ttable: &'a TTable,
        config: &'a SearchConfig,
        limit: &'a SearchLimit,
        is_main: bool,
    ) -> PVSearch<'a> {
        PVSearch {
            game,
            ttable,
            killer_moves: vec![Move::BAD_MOVE; usize::from(u8::MAX) + 1],
            num_nodes_evaluated: 0,
            nodes_since_limit_update: 0,
            num_transpositions: 0,
            config,
            limit,
            is_main,
            selective_depth: 0,
        }
    }

    /// Use Principal Variation Search to evaluate the given game to a depth.
    ///
    /// At each node, the search will examine all legal moves and try to find
    /// the best line, recursively searching to `depth_to_go` moves deep.
    /// However, some heuristics will cause certain lines to be examined more
    /// deeply than `depth_to_go`, and some less so.
    /// When `depth_to_go` reaches zero, a quiescence search will be performed,
    /// preventing the evaluation of "loud" positions from giving incorrect
    /// results.
    ///
    /// When the search is complete, the `Ok()` variant will contain the
    /// evaluation of the position.
    ///
    /// # Inputs
    ///
    /// * `PV`: Whether this node is a principal variation node.
    ///     At the root, this should be `true`.
    /// * `ROOT`: Whether this is the root node of the search.
    ///     External callers of this function should always set `ROOT` to
    ///     `true`.
    /// * `REDUCE`: Whether heuristic depth reduction should be performed.
    /// * `depth_to_go`: The depth to search the position.
    /// * `depth_so_far`: The depth of the recursive stack when this function
    ///     was called. At the start of the search, `depth_so_far` is 0.
    /// * `alpha`: A lower bound on the evaluation of a parent node, in
    ///     perspective of the player to move.
    ///     One way of thinking of `alpha` is that it is the best score that the
    ///     player to move could get if they made a move which did *not* cause
    ///     `pvs()` to be called in this position.
    ///     When called externally, `alpha` should be equal to `Eval::MIN`.
    /// * `beta`: An upper bound on the evaluation of a parent node, in
    ///     perspective of the player to move.
    ///     `beta` can be thought of as the worst score that the opponent of the
    ///     current player to move could get if they decided not to allow the
    ///     current player to make a move.
    ///     When called externally, `beta` should be equal to `Eval::MAX`.
    /// * `parent_line`: The principal variation line of the parent position.
    ///     `parent_line` will be overwritten with the best line found by this
    ///     search, so long as it achieves an alpha cutoff at some point.
    ///
    /// # Errors
    ///
    /// This function will return an error under the conditions described in
    /// `SearchError`'s variants.
    /// The most likely cause of an error will be `SearchError::Timeout`, which
    /// is returned if the limit times out while `pvs()` is runninself.game.
    pub fn pvs<const PV: bool, const ROOT: bool, const REDUCE: bool>(
        &mut self,
        depth_to_go: i8,
        depth_so_far: u8,
        mut alpha: Eval,
        mut beta: Eval,
        parent_line: &mut Vec<Move>,
    ) -> Result<Eval, SearchError> {
        // verify that ROOT implies PV
        debug_assert!(if ROOT { PV } else { true });

        if self.is_main {
            self.limit.update_time()?;
        }

        if self.limit.is_over() {
            return Err(SearchError::Timeout);
        }

        if depth_to_go <= 0 {
            return self.quiesce::<PV>(depth_so_far, alpha, beta, parent_line);
        }

        self.increment_nodes()?;
        self.selective_depth = max(self.selective_depth, depth_so_far);

        // mate distance pruning
        let lower_bound = -Eval::mate_in(depth_so_far);
        if alpha < lower_bound {
            if beta <= lower_bound {
                if PV {
                    parent_line.clear();
                }
                return Ok(lower_bound);
            }
            alpha = lower_bound;
        }

        let upper_bound = Eval::mate_in(1 + depth_so_far);
        if upper_bound < beta {
            if upper_bound <= alpha {
                if PV {
                    parent_line.clear();
                }
                return Ok(upper_bound);
            }
            beta = upper_bound;
        }

        // detect draws.
        if self.game.drawn_by_repetition() || self.game.board().is_drawn() {
            if PV && alpha < Eval::DRAW {
                parent_line.clear();
            }
            // required so that movepicker only needs to know about current
            // position, and not about history
            return Ok(Eval::DRAW);
        }

        // Retrieve transposition data and use it to improve our estimate on
        // the position
        let mut tt_move = None;
        let mut tt_guard = self.ttable.get(self.game.board().hash);
        if let Some(entry) = tt_guard.entry() {
            self.num_transpositions += 1;
            let m = entry.best_move;
            if is_legal(m, self.game.board()) {
                tt_move = Some(m);
                // check if we can cutoff due to transposition table
                if !PV && entry.depth >= depth_to_go {
                    let upper_bound =
                        entry.upper_bound.step_back_by(depth_so_far);
                    if upper_bound <= alpha {
                        return Ok(upper_bound);
                    }
                    let lower_bound =
                        entry.lower_bound.step_back_by(depth_so_far);
                    if beta <= lower_bound {
                        return Ok(lower_bound);
                    }
                }
            }
        }

        let moves_iter = MovePicker::new(
            *self.game.board(),
            self.game.cookie(),
            tt_move,
            self.killer_moves.get(depth_so_far as usize).copied(),
        );
        let mut best_move = Move::BAD_MOVE;
        let mut best_score = Eval::MIN;

        // The number of moves checked. If this is zero after the move search
        // loop, no moves were played.
        let mut move_count = 0;
        // Whether we were able to overwrite alpha by searching moves.
        let mut overwrote_alpha = false;
        for (m, tag) in moves_iter {
            // The principal variation line, following the best move.
            let mut line = Vec::new();
            move_count += 1;
            self.game.make_move(m, &tag);
            // Prefetch the next transposition table entry as early as possible
            // (~12 Elo)
            self.ttable.prefetch(self.game.board().hash);
            let mut score = Eval::MIN;

            if !PV || move_count > 1 {
                // For moves which are not the first move searched at a PV node,
                // or for moves which are not in a PV node,
                // perform a zero-window search of the position.

                let do_lmr =
                    REDUCE && (PV && move_count > 3) || (!PV && move_count > 1);

                let depth_to_search = if do_lmr {
                    depth_to_go - 2
                } else {
                    depth_to_go - 1
                };

                score = -self.pvs::<false, false, REDUCE>(
                    depth_to_search,
                    depth_so_far + 1,
                    -alpha - Eval::centipawns(1),
                    -alpha,
                    &mut line,
                )?;

                // if the LMR search causes an alpha cutoff, ZW search again at
                // full depth.
                if score > alpha && do_lmr {
                    score = -self.pvs::<false, false, REDUCE>(
                        depth_to_go - 1,
                        depth_so_far + 1,
                        -alpha - Eval::centipawns(1),
                        -alpha,
                        &mut line,
                    )?;
                }
            }

            if PV && (move_count == 1 || (alpha < score && score < beta)) {
                // Either this is the first move on a PV node, or the previous
                // search returned a PV candidate.
                score = -self.pvs::<true, false, REDUCE>(
                    depth_to_go - 1,
                    depth_so_far + 1,
                    -beta,
                    -alpha,
                    &mut line,
                )?;
            }

            let undo_result = self.game.undo();
            debug_assert!(undo_result.is_ok());

            if score > best_score {
                best_score = score;
                best_move = m;

                if score > alpha {
                    // if this move was better than what we've seen before,
                    // write it as the principal variation
                    if PV {
                        write_line(parent_line, m, &line);
                    }

                    if beta <= score {
                        // Beta cutoff: we found a move that was so good that
                        // our opponent would never have let us play it in the
                        // first place. Therefore, we need not consider the
                        // other moves, since we wouldn't be allowed to play
                        // them either.
                        break;
                    }

                    // to keep alpha < beta, only write to alpha if there was
                    // not a beta cutoff
                    overwrote_alpha = true;
                    alpha = score;
                }
            }
        }

        debug_assert!((move_count == 0) ^ has_moves(self.game.board()));

        if move_count == 0 {
            // No moves were played, therefore this position is either a
            // stalemate or a mate.
            best_score = if self.game.board().checkers.is_empty() {
                // stalemated
                Eval::DRAW
            } else {
                // mated
                lower_bound
            };
        }

        debug_assert!(Eval::MIN < best_score && best_score < Eval::MAX);

        ttable_store(
            &mut tt_guard,
            depth_so_far,
            depth_to_go,
            if overwrote_alpha { Eval::MIN } else { alpha },
            beta,
            best_score,
            best_move,
        );

        Ok(best_score)
    }

    /// Use quiescent search (captures only) to evaluate a position as deep as
    /// it needs to go until all loud moves are exhausted.
    /// The given `depth_to_go` does not alter the power of the search, but
    /// serves as a handy tool for the search to understand where it is.
    fn quiesce<const PV: bool>(
        &mut self,
        depth_so_far: u8,
        mut alpha: Eval,
        beta: Eval,
        parent_line: &mut Vec<Move>,
    ) -> Result<Eval, SearchError> {
        if !self.game.board().checkers.is_empty() {
            // don't allow settling if we are in check
            return self.pvs::<PV, false, false>(
                1,
                depth_so_far,
                alpha,
                beta,
                parent_line,
            );
        }

        self.increment_nodes()?;
        self.selective_depth = max(self.selective_depth, depth_so_far);

        // check if the game is over before doing anything
        if let Some(mated) = self.game.end_state() {
            // game is over, quit out immediately
            let score = if mated {
                -Eval::mate_in(depth_so_far)
            } else {
                Eval::DRAW
            };

            if PV && alpha < score {
                parent_line.clear();
            }

            return Ok(score);
        }

        let player = self.game.board().player;

        let mut tt_guard = self.ttable.get(self.game.board().hash);
        if let Some(entry) = tt_guard.entry() {
            if !PV && entry.depth >= TTEntry::DEPTH_CAPTURES {
                // this was a deeper search, just use it
                let upper_bound = entry.upper_bound.step_back_by(depth_so_far);
                if upper_bound <= alpha {
                    return Ok(upper_bound);
                }
                let lower_bound = entry.lower_bound.step_back_by(depth_so_far);
                if beta <= lower_bound {
                    return Ok(lower_bound);
                }
            }
        }
        // capturing is unforced, so we can stop here if the player to move
        // doesn't want to capture.
        let mut score = leaf_evaluate(&self.game).in_perspective(player);
        // println!("{g}: {score}");

        // Whether alpha was overwritten by any move at this depth.
        // Used to determine whether this is an exact evaluation on a position
        // when writing to the transposition table.
        let mut overwrote_alpha = false;
        if alpha < score {
            if PV {
                parent_line.clear();
            }

            if beta <= score {
                // store in the transposition table since we won't be able to
                // use the call at the end
                ttable_store(
                    &mut tt_guard,
                    depth_so_far,
                    TTEntry::DEPTH_CAPTURES,
                    Eval::MIN,
                    beta,
                    score,
                    Move::BAD_MOVE,
                );
                // beta cutoff, this line would not be selected because there is a
                // better option somewhere else
                return Ok(score);
            }

            overwrote_alpha = true;
            alpha = score;
        }

        let mut best_score = score;
        let mut moves = self.game.get_moves::<CAPTURES>();
        moves.sort_by_cached_key(|&(_, (_, eval))| -eval);
        let mut line = Vec::new();

        for (m, tag) in moves {
            self.game.make_move(m, &tag);
            // Prefetch the next transposition table entry as early as possible
            // (~12 Elo)
            self.ttable.prefetch(self.game.board().hash);
            // zero-window search
            score = -self.quiesce::<false>(
                depth_so_far + 1,
                -alpha - Eval::centipawns(1),
                -alpha,
                &mut line,
            )?;
            if PV && alpha < score && score < beta {
                // zero-window search failed high, so there is a better option
                // in this tree. we already have a score from before that we
                // can use as a lower bound in this search.
                score = -self.quiesce::<PV>(
                    depth_so_far + 1,
                    -beta,
                    -alpha,
                    &mut line,
                )?;
            }

            let undo_result = self.game.undo();
            // in test mode, verify that we did correctly undo a move
            debug_assert!(undo_result.is_ok());

            if score > best_score {
                best_score = score;
                if alpha < score {
                    if PV {
                        write_line(parent_line, m, &line);
                    }
                    if beta <= score {
                        // Beta cutoff, we have ound a better line somewhere else
                        self.killer_moves[depth_so_far as usize] = m;
                        break;
                    }

                    overwrote_alpha = true;
                    alpha = score;
                }
            }
        }

        ttable_store(
            &mut tt_guard,
            depth_so_far,
            TTEntry::DEPTH_CAPTURES,
            if overwrote_alpha { Eval::MIN } else { alpha },
            beta,
            best_score,
            Move::BAD_MOVE,
        );
        Ok(best_score)
    }

    #[inline(always)]
    /// Increment the number of nodes searched, copying over the value into the
    /// search limit if it is too high.
    fn increment_nodes(&mut self) -> Result<(), SearchError> {
        self.num_nodes_evaluated += 1;
        self.nodes_since_limit_update += 1;
        if u64::from(self.nodes_since_limit_update)
            > self.config.limit_update_increment
        {
            self.update_node_limits()?;
        }
        Ok(())
    }

    #[inline(always)]
    /// Copy over the number of nodes evaluated by this search into the limit
    /// structure, and zero out our number.
    fn update_node_limits(&mut self) -> Result<(), SearchError> {
        self.limit
            .add_nodes(u64::from(self.nodes_since_limit_update))?;
        self.nodes_since_limit_update = 0;
        Ok(())
    }
}

/// Write all of the contents of `line` into the section [1..] of `parent_line`.
fn write_line(parent_line: &mut Vec<Move>, m: Move, line: &[Move]) {
    parent_line.resize(1, m);
    parent_line[0] = m;
    parent_line.extend(line);
}

/// Store data in the transposition table.
/// `score` is the best score of the position as evaluated, while `alpha`
/// and `beta` are the upper and lower bounds on the overall position due
/// to alpha-beta pruning in the game.
fn ttable_store(
    guard: &mut TTEntryGuard,
    depth_so_far: u8,
    depth_to_go: i8,
    alpha: Eval,
    beta: Eval,
    score: Eval,
    best_move: Move,
) {
    let true_score = score.step_forward_by(depth_so_far);
    let upper_bound = if score < beta { true_score } else { Eval::MAX };
    let lower_bound = if alpha < score { true_score } else { Eval::MIN };
    guard.save(depth_to_go, best_move, lower_bound, upper_bound);
}
#[cfg(test)]
pub mod tests {

    use super::*;
    use crate::base::{game::Tagger, Move, Square};
    use crate::engine::evaluate::ScoreTag;

    /// Helper function to search a position at a given depth.
    ///
    /// # Panics
    ///
    /// This function will panic if searching the position fails or the game is
    /// invalid.
    fn search_helper(fen: &str, depth: u8) -> SearchInfo {
        let mut g = ScoredGame::from_fen(fen).unwrap();
        let config = SearchConfig {
            depth,
            ..Default::default()
        };
        let info = search(
            g.clone(),
            depth,
            &TTable::with_size(1000),
            &config,
            &SearchLimit::default(),
            true,
            Eval::MIN,
            Eval::MAX,
        )
        .unwrap();

        // validate principal variation
        for &m in &info.pv {
            println!("{m}");
            assert!(is_legal(m, g.board()));
            g.make_move(m, &ScoreTag::tag_move(m, g.board(), g.cookie()));
        }

        info
    }

    /// A helper function which ensures that the evaluation of a position is
    /// equal to what we expect it to be.
    /// It will check both a normal search and a search without the
    /// transposition table.
    fn eval_helper(fen: &str, eval: Eval, depth: u8) {
        assert_eq!(search_helper(fen, depth).eval, eval);
    }

    #[test]
    /// Test `PVSearch`'s evaluation of the start position of the game.
    fn eval_start() {
        let info = search_helper(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            8,
        );
        println!("best move: {} [{}]", info.pv[0], info.eval);
    }

    #[test]
    /// A test on the evaluation of the game in the fried liver position.
    /// The only winning move for White is Qd3+.
    fn fried_liver() {
        let info = search_helper(
            "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
            8,
        );
        let m = Move::normal(Square::D1, Square::F3);
        assert_eq!(info.pv[0], m);
    }

    #[test]
    /// Try searching an end-ish game position.
    /// This was used as part of debugging for an illegal PV being created.
    fn endgame() {
        search_helper(
            "2k5/pp3pp1/2p1pr2/Pn2b3/1P1P1P1r/2p1P1N1/6R1/3R2K1 w - - 0 1",
            6,
        );
    }

    #[test]
    /// A test that the engine can find a mate in 1 move.
    fn mate_in_1() {
        // Rb8# is mate in one
        eval_helper("3k4/R7/1R6/5K2/8/8/8/8 w - - 0 1", Eval::mate_in(1), 2);
    }

    #[test]
    /// A test that shows the engine can find a mate in 4 plies, given enough
    /// depth.
    fn mate_in_4_ply() {
        // because black, the player to move, is getting mated, the evaluation
        // is negative here
        eval_helper("3k4/R7/8/5K2/3R4/8/8/8 b - - 0 1", -Eval::mate_in(4), 5);
    }

    #[test]
    /// A test for a puzzle made by Ian. White has mate in 5 with Rxf7+.
    fn mate_in_9_ply() {
        // because capturing a low-value piece is often a "late" move, it is
        // likely to be reduced in depth
        eval_helper(
            "2r2r2/3p1p1k/p3p1p1/3P3n/q3P1Q1/1p5P/1PP2R2/1K4R1 w - - 0 30",
            Eval::mate_in(9),
            11,
        );
    }

    #[test]
    /// Test that the transposition table contains an entry for the root node of
    /// the search.
    fn ttable_populated() {
        let ttable = TTable::with_size(1);
        let g = ScoredGame::new();
        let depth = 5;

        let search_info = search(
            g.clone(),
            depth,
            &ttable,
            &SearchConfig {
                depth: 5,
                ..Default::default()
            },
            &SearchLimit::new(),
            true,
            Eval::MIN,
            Eval::MAX,
        )
        .unwrap();

        let entry = ttable.get(g.board().hash).entry().unwrap();

        // println!("{entry:?}");
        // println!("{search_info:?}");
        assert_eq!(entry.depth, i8::try_from(depth).unwrap());
        assert_eq!(entry.best_move, search_info.pv[0]);
        assert_eq!(entry.lower_bound, entry.upper_bound);
    }
}
