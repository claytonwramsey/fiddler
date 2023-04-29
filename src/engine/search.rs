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

use crate::{
    base::{
        game::Game,
        movegen::{get_moves, has_moves, is_legal, GenMode},
        Move,
    },
    engine::evaluate::{calculate_phase, eval_nl_delta},
};

use super::{
    evaluate::{cumulative_init, mg_npm, Eval, Score},
    pick::TaggedMove,
    transposition::{TTEntry, TTEntryGuard},
};

use super::{
    evaluate::leaf_evaluate, limit::SearchLimit, pick::MovePicker, thread::SearchConfig,
    transposition::TTable,
};

use std::cmp::max;

#[allow(clippy::too_many_arguments, clippy::cast_possible_wrap)]
/// Evaluate the given game.
/// The evaluation will be from the player's perspective, i.e. inverted if the
/// player to move is Black.
///
/// # Inputs
///
/// * `g`: the game which will be evaluated.
/// * `ttable`: a reference to the shared transposition table.
/// * `config`: the configuration of this search.
/// * `limit`:the search limiter, which will be interiorly mutated by this function.
/// * `is_main`: whether or not this search is the "main" search or a subjugate thread, and
///   determines responsibilities as such.
/// * `alpha`: a lower bound on the evaluation. This is primarily intended to be used for aspiration
///   windowing, and in most cases will be set to `Eval::MIN`.
/// * `beta`: is an upper bound on the evaluation. This is primarily intended to be used for
///   aspiration windowing, and in most cases will be set to `Eval::MAX`.
///
/// # Errors
///
/// This function will return an `Err` if the search times out.
pub fn search(
    g: Game,
    depth: u8,
    ttable: &TTable,
    config: &SearchConfig,
    limit: &SearchLimit,
    is_main: bool,
    alpha: Eval,
    beta: Eval,
) -> Result<SearchInfo, ()> {
    let mut searcher = PVSearch::new(g, ttable, config, limit, is_main);
    let mut pv = Vec::new();
    let root_mg_npm = mg_npm(&searcher.game);
    let mut initial_state = NodeState {
        depth_since_root: 0,
        cumulative_score: cumulative_init(&searcher.game),
        mg_npm: root_mg_npm,
        phase: calculate_phase(root_mg_npm),
        line: &mut pv,
    };

    let eval = searcher.pvs::<true, true, true>(depth as i8, alpha, beta, &mut initial_state)?;

    Ok(SearchInfo {
        pv,
        eval,
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
    /// The number of nodes evaluated in this search.
    pub num_nodes_evaluated: u64,
    /// The highest depth at which this search succeeded.
    pub depth: u8,
    /// The selective search depth, i.e. the highest depth to which any position
    /// was considered.
    pub selective_depth: u8,
}

impl SearchInfo {
    /// Unify with another `SearchInfo`, selecting the most accurate evaluation (by depth) and
    /// summing the number of transpositions and nodes evaluated.
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
    }
}

#[derive(Clone, Debug)]
/// A structure containing data which is shared across function calls to a principal variation
/// search.
struct PVSearch<'a> {
    /// The game being searched.
    game: Game,
    /// The transposition table.
    ttable: &'a TTable,
    /// The set of "killer" moves. Each index corresponds to a depth (0 is most shallow, etc).
    killer_moves: Vec<Move>,
    /// The cumulative number of nodes evaluated in this evaluation.
    num_nodes_evaluated: u64,
    /// The cumulative number of nodes visited since we last updated the limit.
    nodes_since_limit_update: u16,
    /// The configuration of this search.
    config: &'a SearchConfig,
    /// The limit to this search.
    limit: &'a SearchLimit,
    /// Whether this search is the main search.
    is_main: bool,
    /// The highest depth to which any line was searched.
    selective_depth: u8,
}

/// A structure which contains information about a single node in a principal variation search.
/// This structure exists to make the process of calling functions easier.
struct NodeState<'a> {
    /// The depth of this node since the root node.
    depth_since_root: u8,
    /// The cumulative evaluation of the current game state.
    /// This is given as an input to `leaf_evaluate`, among other things.
    cumulative_score: Score,
    /// The quantity of midgame non-pawn material in the current state of the game.
    /// This is used to determine the phase of future stages of the game.
    mg_npm: Eval,
    /// The current phase of the game: a number in the rangs [0, 1].
    phase: f32,
    /// The line of moves.
    ///  When this state is passed, the line should be empty.
    line: &'a mut Vec<Move>,
}

impl<'a> PVSearch<'a> {
    /// Construct a new `PVSearch` using a given transposition table, configuration, and limit.
    ///
    /// `is_main` is whether the thread is a main search, responsible for certain synchronization
    /// activities.
    pub fn new(
        game: Game,
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
            config,
            limit,
            is_main,
            selective_depth: 0,
        }
    }

    #[inline(never)]
    /// Use Principal Variation Search to evaluate the given game to a depth.
    ///
    /// At each node, the search will examine all legal moves and try to find the best line,
    /// recursively searching to `depth` moves deep.
    /// However, some heuristics will cause certain lines to be examined more deeply than
    /// `depth`, and some less so.
    /// When `depth` reaches zero, a quiescence search will be performed, preventing the
    /// evaluation of "loud" positions from giving incorrect results.
    ///
    /// When the search is complete, the `Ok()` variant will contain the evaluation of the position.
    ///
    /// # Inputs
    ///
    /// * `PV`: Whether this node is a principal variation node. At the root, this should be `true`.
    /// * `ROOT`: Whether this is the root node of the search. External callers of this function
    ///   should always set `ROOT` to `true`.
    /// * `REDUCE`: Whether heuristic depth reduction should be performed.
    /// * `depth`: The depth to search the position.
    /// * `alpha`: A lower bound on the evaluation of a parent node, in perspective of the player to
    ///   move. One way of thinking of `alpha` is that it is the best score that the player to move
    ///   could get if they made a move which did *not* cause `pvs()` to be called in this position.
    ///   When called externally, `alpha` should be equal to `Eval::MIN`.
    /// * `beta`: An upper bound on the evaluation of a parent node, in perspective of the player to
    ///   move. `beta` can be thought of as the worst score that the opponent of the current player
    ///   to move could get if they decided not to allow the current player to make a move. When
    ///   called externally, `beta` should be equal to `Eval::MAX`.
    /// - `state`: The shared state of this node, containing the principal variation and other data.
    ///
    /// # Errors
    ///
    /// This function will return an error under the conditions described in `SearchError`'s
    /// variants.
    /// The most likely cause of an error will be `SearchError::Timeout`, which is returned if the
    /// limit times out while `pvs()` is runn in `self.game`.
    pub fn pvs<const PV: bool, const ROOT: bool, const REDUCE: bool>(
        &mut self,
        depth: i8,
        mut alpha: Eval,
        mut beta: Eval,
        state: &mut NodeState,
    ) -> Result<Eval, ()> {
        // verify that ROOT implies PV
        debug_assert!(if ROOT { PV } else { true });

        if self.is_main {
            self.limit.update_time();
        }

        if self.limit.is_over() {
            return Err(());
        }

        if depth <= 0 {
            return self.quiesce::<PV>(alpha, beta, state);
        }

        self.increment_nodes();
        self.selective_depth = max(self.selective_depth, state.depth_since_root);

        // mate distance pruning
        let lower_bound = -Eval::mate_in(state.depth_since_root);
        if alpha < lower_bound {
            if beta <= lower_bound {
                if PV {
                    state.line.clear();
                }
                return Ok(lower_bound);
            }
            alpha = lower_bound;
        }

        let upper_bound = Eval::mate_in(1 + state.depth_since_root);
        if upper_bound < beta {
            if upper_bound <= alpha {
                if PV {
                    state.line.clear();
                }
                return Ok(upper_bound);
            }
            beta = upper_bound;
        }

        // detect draws.
        if self.game.insufficient_material()
            || self
                .game
                .drawn_by_repetition(u16::from(state.depth_since_root))
        {
            if PV {
                state.line.clear();
            }
            // required so that movepicker only needs to know about current position, and not about
            // history
            return Ok(Eval::DRAW);
        }

        // Retrieve transposition data and use it to improve our estimate on the position
        let mut tt_move = None;
        let mut tt_guard = self.ttable.get(self.game.meta().hash);
        if let Some(entry) = tt_guard.entry() {
            let m = entry.best_move;
            if is_legal(m, &self.game) {
                tt_move = Some(m);
                // check if we can cutoff due to transposition table
                if entry.depth >= depth {
                    let upper_bound = entry.upper_bound.step_back_by(state.depth_since_root);
                    if upper_bound <= alpha {
                        if PV {
                            state.line.clear();
                            state.line.push(m);
                        }
                        return Ok(upper_bound);
                    }
                    let lower_bound = entry.lower_bound.step_back_by(state.depth_since_root);
                    if beta <= lower_bound {
                        if PV {
                            state.line.clear();
                            state.line.push(m);
                        }
                        return Ok(lower_bound);
                    }
                }
            }
        }

        let mut moves_iter = MovePicker::new(
            tt_move,
            self.killer_moves
                .get(state.depth_since_root as usize)
                .copied(),
        );
        let mut best_move = Move::BAD_MOVE;
        let mut best_score = Eval::MIN;

        // The number of moves checked. If this is zero after the move search loop, no moves were
        // played.
        let mut move_count = 0;
        // Whether we were able to overwrite alpha by searching moves.
        let mut overwrote_alpha = false;
        // The principal variation line, following the best move.
        let mut child_line = Vec::new();
        while let Some(tm) = moves_iter.next(&self.game, state.mg_npm) {
            move_count += 1;

            let mut new_state = NodeState {
                depth_since_root: state.depth_since_root + 1,
                cumulative_score: -state.cumulative_score - eval_nl_delta(tm.m, &self.game),
                mg_npm: tm.new_mg_npm,
                phase: tm.phase,
                line: &mut child_line,
            };

            self.game.make_move(tm.m);
            // Prefetch the next transposition table entry as early as possible
            // (~12 Elo)
            self.ttable.prefetch(self.game.meta().hash);

            let mut score = Eval::MIN;

            if !PV || move_count > 1 {
                // For moves which are not the first move searched at a PV node, or for moves which
                // are not in a PV node, perform a zero-window search of the position.

                // Late move reduction:
                // search positions which are unlikely to be the PV at a lower depth.
                // ~400 Elo
                let do_lmr = REDUCE && move_count > 2;

                let depth_to_search = if do_lmr { depth - 3 } else { depth - 1 };

                score = -self.pvs::<false, false, REDUCE>(
                    depth_to_search,
                    -alpha - Eval::centipawns(1),
                    -alpha,
                    &mut new_state,
                )?;

                // if the LMR search causes an alpha cutoff, ZW search again at full depth.
                if score > alpha && do_lmr {
                    score = -self.pvs::<false, false, REDUCE>(
                        depth - 1,
                        -alpha - Eval::centipawns(1),
                        -alpha,
                        &mut new_state,
                    )?;
                }
            }

            if PV && (move_count == 1 || (alpha < score && score < beta)) {
                // Either this is the first move on a PV node, or the previous search returned a PV
                // candidate.
                score =
                    -self.pvs::<true, false, REDUCE>(depth - 1, -beta, -alpha, &mut new_state)?;
            }

            let undo_result = self.game.undo();
            debug_assert!(undo_result.is_ok());

            if score > best_score {
                best_score = score;
                best_move = tm.m;

                if score > alpha {
                    // if this move was better than what we've seen before, write it as the
                    // principal variation
                    if PV {
                        write_line(state.line, tm.m, new_state.line);
                    }

                    if beta <= score {
                        // Beta cutoff: we found a move that was so good that our opponent would
                        // never have let us play it in the first place.
                        // Therefore, we need not consider the other moves, since we wouldn't be
                        // allowed to play them either.
                        break;
                    }

                    // to keep alpha < beta, only write to alpha if there was not a beta cutoff
                    overwrote_alpha = true;
                    alpha = score;
                }
            }
        }

        debug_assert!((move_count == 0) ^ has_moves(&self.game));

        if move_count == 0 {
            // No moves were played, therefore this position is either a stalemate or a mate.
            state.line.clear();
            best_score = if self.game.meta().checkers.is_empty() {
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
            state.depth_since_root,
            depth,
            if overwrote_alpha { Eval::MIN } else { alpha },
            beta,
            best_score,
            best_move,
        );

        Ok(best_score)
    }

    #[inline(never)]
    /// Use quiescent search (captures only) to evaluate a position as deep as it needs to go until
    /// all loud moves are exhausted.
    /// The given `depth` does not alter the power of the search, but  serves as a handy tool
    /// for the search to understand where it is.
    fn quiesce<const PV: bool>(
        &mut self,
        mut alpha: Eval,
        beta: Eval,
        state: &mut NodeState,
    ) -> Result<Eval, ()> {
        if !self.game.meta().checkers.is_empty() {
            // don't allow settling if we are in check (~48 Elo)
            return self.pvs::<PV, false, false>(1, alpha, beta, state);
        }

        self.increment_nodes();
        self.selective_depth = max(self.selective_depth, state.depth_since_root);

        // check if the game is over before doing anything
        if self
            .game
            .drawn_by_repetition(u16::from(state.depth_since_root))
            || self.game.insufficient_material()
        {
            // game is drawn
            if PV {
                state.line.clear();
            }

            return Ok(Eval::DRAW);
        }

        if !has_moves(&self.game) {
            if PV {
                state.line.clear();
            }

            return Ok(if self.game.meta().checkers.is_empty() {
                Eval::DRAW
            } else {
                Eval::BLACK_MATE
            });
        }

        let player = self.game.meta().player;

        let mut tt_guard = self.ttable.get(self.game.meta().hash);
        if let Some(entry) = tt_guard.entry() {
            if entry.depth >= TTEntry::DEPTH_CAPTURES {
                // this was a deeper search, just use it
                let upper_bound = entry.upper_bound.step_back_by(state.depth_since_root);
                if upper_bound <= alpha {
                    if PV {
                        state.line.clear();
                        state.line.push(entry.best_move);
                    }
                    return Ok(upper_bound);
                }
                let lower_bound = entry.lower_bound.step_back_by(state.depth_since_root);
                if beta <= lower_bound {
                    if PV {
                        state.line.clear();
                        state.line.push(entry.best_move);
                    }
                    return Ok(lower_bound);
                }
            }
        }
        // capturing is unforced, so we can stop here if the player to move doesn't want to capture.
        let mut score =
            leaf_evaluate(&self.game, state.cumulative_score, state.phase).in_perspective(player);
        // println!("{}: {score}", self.game);

        // Whether alpha was overwritten by any move at this depth.
        // Used to determine whether this is an exact evaluation on a position when writing to the
        // transposition table.
        let mut overwrote_alpha = false;
        if alpha < score {
            if PV {
                state.line.clear();
            }

            if beta <= score {
                // store in the transposition table since we won't be able to use the call at the
                // end
                ttable_store(
                    &mut tt_guard,
                    state.depth_since_root,
                    TTEntry::DEPTH_CAPTURES,
                    Eval::MIN,
                    beta,
                    score,
                    Move::BAD_MOVE,
                );
                // beta cutoff, this line would not be selected because there is a better option
                // somewhere else
                return Ok(score);
            }

            overwrote_alpha = true;
            alpha = score;
        }

        let mut best_score = score;

        // get captures and sort in descending order of quality
        let mut moves = Vec::with_capacity(10);
        get_moves::<{ GenMode::Captures }>(&self.game, |m| {
            moves.push(TaggedMove::new(&self.game, m, state.mg_npm));
        });
        moves.sort_by_cached_key(|tm| -tm.quality);
        let mut child_line = Vec::new();

        for tm in moves {
            let mut new_state = NodeState {
                depth_since_root: state.depth_since_root + 1,
                cumulative_score: -state.cumulative_score - eval_nl_delta(tm.m, &self.game),
                mg_npm: tm.new_mg_npm,
                phase: tm.phase,
                line: &mut child_line,
            };

            self.game.make_move(tm.m);
            // Prefetch the next transposition table entry as early as possible
            // (~12 Elo)
            self.ttable.prefetch(self.game.meta().hash);
            // zero-window search
            score = -self.quiesce::<false>(-alpha - Eval::centipawns(1), -alpha, &mut new_state)?;
            if PV && alpha < score && score < beta {
                // zero-window search failed high, so there is a better option in this tree.
                // we already have a score from before that we can use as a lower bound in this
                // search.
                score = -self.quiesce::<PV>(-beta, -alpha, &mut new_state)?;
            }

            let undo_result = self.game.undo();
            // in test mode, verify that we did correctly undo a move
            debug_assert!(undo_result.is_ok());

            if score > best_score {
                best_score = score;
                if alpha < score {
                    if PV {
                        write_line(state.line, tm.m, &child_line);
                    }
                    if beta <= score {
                        // Beta cutoff, we have ound a better line somewhere else
                        self.killer_moves[state.depth_since_root as usize] = tm.m;
                        break;
                    }

                    overwrote_alpha = true;
                    alpha = score;
                }
            }
        }

        ttable_store(
            &mut tt_guard,
            state.depth_since_root,
            TTEntry::DEPTH_CAPTURES,
            if overwrote_alpha { Eval::MIN } else { alpha },
            beta,
            best_score,
            Move::BAD_MOVE,
        );
        Ok(best_score)
    }

    /// Increment the number of nodes searched, copying over the value into the search limit if it
    /// is too high.
    fn increment_nodes(&mut self) {
        self.num_nodes_evaluated += 1;
        self.nodes_since_limit_update += 1;
        if u64::from(self.nodes_since_limit_update) > self.config.limit_update_increment {
            self.limit
                .add_nodes(u64::from(self.nodes_since_limit_update));
            self.nodes_since_limit_update = 0;
        }
    }
}

/// Write all of the contents of `line` into the section [1..] of `parent_line`.
fn write_line(parent_line: &mut Vec<Move>, m: Move, line: &[Move]) {
    parent_line.resize(1, m);
    parent_line[0] = m;
    parent_line.extend(line);
}

/// Store data in the transposition table.
/// `score` is the best score of the position as evaluated, while `alpha` and `beta` are the upper
/// and lower bounds on the overall position due to alpha-beta pruning in the game.
fn ttable_store(
    guard: &mut TTEntryGuard,
    depth_so_far: u8,
    depth: i8,
    alpha: Eval,
    beta: Eval,
    score: Eval,
    best_move: Move,
) {
    let true_score = score.step_forward_by(depth_so_far);
    let upper_bound = if score < beta { true_score } else { Eval::MAX };
    let lower_bound = if alpha < score { true_score } else { Eval::MIN };
    guard.save(depth, best_move, lower_bound, upper_bound);
}
#[cfg(test)]
pub mod tests {

    use super::*;
    use crate::base::{game::Game, Move, Square};

    /// Helper function to search a position at a given depth.
    ///
    /// # Panics
    ///
    /// This function will panic if searching the position fails or the game is invalid.
    fn search_helper(fen: &str, depth: u8) -> SearchInfo {
        let mut g = Game::from_fen(fen).unwrap();
        let config = SearchConfig {
            depth,
            ..Default::default()
        };
        let info = search(
            g.clone(),
            depth,
            &TTable::with_size(1000),
            &config,
            &SearchLimit::infinite(),
            true,
            Eval::MIN,
            Eval::MAX,
        )
        .unwrap();

        // validate principal variation
        for &m in &info.pv {
            println!("{m}");
            assert!(is_legal(m, &g));
            g.make_move(m);
        }

        info
    }

    /// A helper function which ensures that the evaluation of a position is equal to what we expect
    /// it to be.
    /// It will check both a normal search and a search without the transposition table.
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
    /// A test that shows the engine can find a mate in 4 plies, given enough depth.
    fn mate_in_4_ply() {
        // because black, the player to move, is getting mated, the evaluation is negative here
        eval_helper("3k4/R7/8/5K2/3R4/8/8/8 b - - 0 1", -Eval::mate_in(4), 6);
    }

    #[test]
    /// A test for a puzzle made by Ian. White has mate in 5 with Rxf7+.
    fn mate_in_9_ply() {
        // because capturing a low-value piece is often a "late" move, it is likely to be reduced in
        // depth
        eval_helper(
            "2r2r2/3p1p1k/p3p1p1/3P3n/q3P1Q1/1p5P/1PP2R2/1K4R1 w - - 0 30",
            Eval::mate_in(9),
            11,
        );
    }

    #[test]
    /// Test that the engine can use a draw by repetition to escape being mated.
    fn escape_by_draw() {
        let mut g = Game::from_fen("2k5/6R1/7Q/8/8/8/8/1K6 w - - 0 1").unwrap();

        g.make_move(Move::normal(Square::H6, Square::F6));
        g.make_move(Move::normal(Square::C8, Square::B8));
        g.make_move(Move::normal(Square::F6, Square::H6));
        g.make_move(Move::normal(Square::B8, Square::C8));

        g.make_move(Move::normal(Square::H6, Square::F6));
        g.make_move(Move::normal(Square::C8, Square::B8));
        g.make_move(Move::normal(Square::F6, Square::H6));

        assert!(has_moves(&g));
        assert!(!g.drawn_by_repetition(0));
        // black can now play Kc8 to escape

        let config = SearchConfig {
            depth: 4,
            ..Default::default()
        };

        let info = search(
            g.clone(),
            4,
            &TTable::with_size(1),
            &config,
            &SearchLimit::infinite(),
            true,
            Eval::MIN,
            Eval::MAX,
        )
        .unwrap();

        // validate principal variation
        for &m in &info.pv {
            println!("{m}");
            assert!(is_legal(m, &g));
            g.make_move(m);
        }

        assert_eq!(info.eval, Eval::DRAW);
        assert_eq!(info.pv[0], Move::normal(Square::B8, Square::C8));
    }

    #[test]
    /// Test that the transposition table contains an entry for the root node of the search.
    fn ttable_populated() {
        let ttable = TTable::with_size(1);
        let g = Game::new();
        let depth = 5;

        let search_info = search(
            g.clone(),
            depth,
            &ttable,
            &SearchConfig {
                depth: 5,
                ..Default::default()
            },
            &SearchLimit::infinite(),
            true,
            Eval::MIN,
            Eval::MAX,
        )
        .unwrap();

        let entry = ttable.get(g.meta().hash).entry().unwrap();

        // println!("{entry:?}");
        // println!("{search_info:?}");
        assert_eq!(entry.depth, i8::try_from(depth).unwrap());
        assert_eq!(entry.best_move, search_info.pv[0]);
        assert_eq!(entry.lower_bound, entry.upper_bound);
    }
}
