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
        Move, Piece,
    },
    engine::{
        evaluate::{calculate_phase, eval_nl_delta},
        transposition::BoundType,
    },
};

use super::{
    evaluate::{cumulative_init, mg_npm, Eval, Score},
    pick::TaggedMove,
    transposition::TTEntry,
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
    let root_mg_npm = mg_npm(&searcher.game);
    let mut initial_state = NodeState {
        depth_since_root: 0,
        cumulative_score: cumulative_init(&searcher.game),
        mg_npm: root_mg_npm,
        phase: calculate_phase(root_mg_npm),
        line: Vec::new(),
    };

    let eval = searcher.pvs::<true, true, true>(depth as i8, alpha, beta, &mut initial_state)?;

    Ok(SearchInfo {
        pv: initial_state.line,
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
    killer_moves: Vec<Option<Move>>,
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
struct NodeState {
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
    /// When this state is passed, the line should be empty.
    line: Vec<Move>,
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
            killer_moves: vec![None; usize::from(u8::MAX) + 1],
            num_nodes_evaluated: 0,
            nodes_since_limit_update: 0,
            config,
            limit,
            is_main,
            selective_depth: 0,
        }
    }

    #[inline(never)]
    #[allow(clippy::too_many_lines)]
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
    /// This function will return an error if the search times out before it is able to complete.
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
                return Ok(lower_bound);
            }
            alpha = lower_bound;
        }

        let upper_bound = Eval::mate_in(1 + state.depth_since_root);
        if upper_bound < beta {
            if upper_bound <= alpha {
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
            // required so that movepicker only needs to know about current position, and not about
            // history
            return Ok(Eval::DRAW);
        }

        // Retrieve transposition data and use it to improve our estimate on the position
        let mut tt_move = None;
        let mut tt_guard = self.ttable.get(self.game.meta().hash);
        if let Some(entry) = tt_guard.entry() {
            let m = entry.best_move;
            if m.map_or(true, |m| is_legal(m, &self.game)) {
                tt_move = m;
                // check if we can cutoff due to transposition table
                if !PV && entry.depth >= depth {
                    let value = entry.value.step_back_by(state.depth_since_root);
                    let cutoff = match entry.bound_type() {
                        BoundType::Exact => true,
                        BoundType::Lower => beta <= value,
                        BoundType::Upper => value <= alpha,
                    };

                    if cutoff {
                        return Ok(value);
                    }
                }
            }
        }

        // Use null-move pruning to construct a best-guess lower bound on the position.
        // Do not prune in principal variation nodes or if the previous move was a null-move.
        // (~45 Elo)
        if !PV // do not prune in PV lines
            && depth >= 4 // must have some amount of depth left to search
            && beta < Eval::MAX // static evaluation must be good
            && self.game.meta().checkers.is_empty() // cannot nullmove out of check
            // prevent zugzwang
            && (!self.game[Piece::Pawn] & self.game[self.game.meta().player]).more_than_one()
            && self.game.moves.last().is_some()
        {
            unsafe {
                // SAFETY: The king is not in check and the most recent move is not a null-move.
                self.game.null_move();

                let null_depth = depth - 4;

                let mut null_state = NodeState {
                    depth_since_root: state.depth_since_root + 1,
                    cumulative_score: -state.cumulative_score,
                    mg_npm: state.mg_npm,
                    phase: state.phase,
                    line: Vec::new(),
                };
                let null_score = -self.pvs::<false, false, REDUCE>(
                    null_depth,
                    -beta,
                    -beta + Eval::centipawns(1),
                    &mut null_state,
                )?;

                self.game.undo_null();

                if null_score >= beta {
                    return Ok(null_score);
                }
            }
        }

        let mut moves_iter = MovePicker::new(
            tt_move,
            self.killer_moves
                .get(state.depth_since_root as usize)
                .copied()
                .flatten(),
        );
        let mut best_move = None;
        let mut best_score = Eval::MIN;

        // The number of moves checked. If this is zero after the move search loop, no moves were
        // played.
        let mut move_count = 0;
        // Whether we were able to overwrite alpha by searching moves.
        let mut overwrote_alpha = false;
        // The principal variation line, following the best move.
        while let Some(tm) = moves_iter.next(&self.game, state.mg_npm) {
            move_count += 1;

            let mut new_state = NodeState {
                depth_since_root: state.depth_since_root + 1,
                cumulative_score: -state.cumulative_score - eval_nl_delta(tm.m, &self.game),
                mg_npm: tm.new_mg_npm,
                phase: tm.phase,
                line: Vec::new(),
            };

            self.game.make_move(tm.m);

            let mut score = Eval::MIN;

            if !PV || move_count > 1 {
                // For moves which are not the first move searched at a PV node, or for moves which
                // are not in a PV node, perform a zero-window search of the position.

                // Late move reduction:
                // search positions which are unlikely to be the PV at a lower depth.
                const REDUCTION_TABLE: [i8; 5] = [1, 1, 1, 2, 3];
                let lmr_depth_change = if REDUCE {
                    *REDUCTION_TABLE
                        .get(move_count)
                        .unwrap_or(REDUCTION_TABLE.last().unwrap())
                } else {
                    1
                };

                let depth_to_search = depth - lmr_depth_change;

                score = -self.pvs::<false, false, REDUCE>(
                    depth_to_search,
                    -alpha - Eval::centipawns(1),
                    -alpha,
                    &mut new_state,
                )?;

                // if the LMR search causes an alpha cutoff, ZW search again at full depth.
                if score > alpha && lmr_depth_change != 1 {
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
                best_move = Some(tm.m);

                if score > alpha {
                    // if this move was better than what we've seen before, write it as the
                    // principal variation
                    if PV {
                        write_line(&mut state.line, tm.m, &new_state.line);
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
            best_score = if self.game.meta().checkers.is_empty() {
                // stalemated
                Eval::DRAW
            } else {
                // mated
                lower_bound
            };
        }

        debug_assert!(Eval::MIN < best_score && best_score < Eval::MAX);

        tt_guard.save(
            depth,
            best_move,
            best_score.step_forward_by(state.depth_since_root),
            if best_score >= beta {
                BoundType::Lower
            } else if PV && overwrote_alpha {
                BoundType::Exact
            } else {
                BoundType::Upper
            },
        );

        Ok(best_score)
    }

    #[inline(never)]
    #[allow(clippy::too_many_lines)]
    /// Use quiescent search (captures only) to evaluate a position as deep as it needs to go until
    /// all loud moves are exhausted.
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
            return Ok(Eval::DRAW);
        }

        if !has_moves(&self.game) {
            return Ok(if self.game.meta().checkers.is_empty() {
                Eval::DRAW
            } else {
                Eval::BLACK_MATE
            });
        }

        let player = self.game.meta().player;

        let mut tt_guard = self.ttable.get(self.game.meta().hash);
        if let Some(entry) = tt_guard.entry() {
            if !PV
                && entry.depth >= TTEntry::DEPTH_CAPTURES
                && entry.best_move.map_or(true, |m| is_legal(m, &self.game))
            {
                let value = entry.value.step_back_by(state.depth_since_root);
                let cutoff = match entry.bound_type() {
                    BoundType::Exact => true,
                    BoundType::Lower => beta <= value,
                    BoundType::Upper => value <= alpha,
                };

                if cutoff {
                    return Ok(value);
                }
            }
        }
        // capturing is unforced, so we can stop here if the player to move doesn't want to capture.
        let mut score =
            leaf_evaluate(&self.game, state.cumulative_score, state.phase).in_perspective(player);
        // println!("{}: {score}", self.game);
        if alpha < score {
            if beta <= score {
                // store in the transposition table since we won't be able to use the call at the
                // end
                tt_guard.save(
                    TTEntry::DEPTH_CAPTURES,
                    None,
                    score.step_forward_by(state.depth_since_root),
                    BoundType::Lower,
                );
                // beta cutoff, this line would not be selected because there is a better option
                // somewhere else
                return Ok(score);
            }

            alpha = score;
        }

        let mut best_score = score;
        let mut best_move = None;

        // get captures and sort in descending order of quality
        let mut moves = Vec::with_capacity(10);
        get_moves::<{ GenMode::Captures }>(&self.game, |m| {
            moves.push(TaggedMove::new(&self.game, m, state.mg_npm));
        });
        moves.sort_by_cached_key(|tm| -tm.quality);

        for tm in moves {
            let mut new_state = NodeState {
                depth_since_root: state.depth_since_root + 1,
                cumulative_score: -state.cumulative_score - eval_nl_delta(tm.m, &self.game),
                mg_npm: tm.new_mg_npm,
                phase: tm.phase,
                line: Vec::new(),
            };

            self.game.make_move(tm.m);
            // zero-window search
            score = -self.quiesce::<false>(-alpha - Eval::centipawns(1), -alpha, &mut new_state)?;
            if PV && alpha < score && score < beta {
                // zero-window search failed high, so there is a better option in this tree.
                score = -self.quiesce::<PV>(-beta, -alpha, &mut new_state)?;
            }

            let undo_result = self.game.undo();
            // in test mode, verify that we did correctly undo a move
            debug_assert!(undo_result.is_ok());

            if score > best_score {
                best_score = score;
                if alpha < score {
                    best_move = Some(tm.m);
                    if PV {
                        write_line(&mut state.line, tm.m, &new_state.line);
                    }

                    if beta <= score {
                        // Beta cutoff, we have ound a better line somewhere else
                        self.killer_moves[state.depth_since_root as usize] = Some(tm.m);
                        break;
                    }

                    alpha = score;
                }
            }
        }

        tt_guard.save(
            TTEntry::DEPTH_CAPTURES,
            best_move,
            best_score.step_forward_by(state.depth_since_root),
            if best_score >= beta {
                BoundType::Upper
            } else {
                BoundType::Lower
            },
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
        eval_helper("3k4/R7/8/5K2/3R4/8/8/8 b - - 0 1", -Eval::mate_in(4), 8);
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
        assert_eq!(entry.best_move.unwrap(), search_info.pv[0]);
        assert_eq!(entry.bound_type(), BoundType::Exact);
    }
}
