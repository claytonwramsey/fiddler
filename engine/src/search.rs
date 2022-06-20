use fiddler_base::{
    movegen::{get_moves, CAPTURES},
    Eval, Game, Move,
};

use crate::candidacy::PstNominate;

use super::{
    config::SearchConfig,
    evaluate::leaf_evaluate,
    limit::SearchLimit,
    pick::MovePicker,
    transposition::{EvalData, TTable},
};

use std::sync::Arc;
use std::{
    cmp::{max, min},
    sync::PoisonError,
};

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
    #[inline(always)]
    fn from(_: PoisonError<T>) -> Self {
        SearchError::Poison
    }
}

/// The result of performing a search. The `Ok` version contains data on the
/// search, while the `Err` version contains a reason why the search failed.
pub type SearchResult = Result<SearchInfo, SearchError>;

/// The output type of a search. An `Err` may be given if, for instance,
/// the search times out.
type PVSResult = Result<(Move, Eval), SearchError>;

#[inline(always)]
/// Evaluate the given game. Return a pair containing the best move and its
/// evaluation, as well as the depth to which the evaluation was searched. The
/// evaluation will be from the player's perspective, i.e. inverted if the
/// player to move is Black.
///
/// `g` is the game which will be evaluated.
///
/// `ttable` is a reference counter to the shared transposition table.
///
/// `config` is the configuration of this search.
///
/// `limit` is the search limiter, and will be interiorly mutated by this
/// function.
///
/// `is_main` determines whether or not this search is the "main" search or a
/// subjugate thread, and determines responsibilities as such.
pub fn search(
    mut g: Game,
    depth: u8,
    ttable: Arc<TTable>,
    config: &SearchConfig,
    limit: Arc<SearchLimit>,
    is_main: bool,
) -> SearchResult {
    let mut searcher = PVSearch::new(ttable, config, limit, is_main);

    let (m, eval) = searcher.pvs(depth as i8, 0, &mut g, Eval::MIN, Eval::MAX, true)?;

    Ok(SearchInfo {
        best_move: m,
        eval,
        num_transpositions: searcher.num_transpositions,
        num_nodes_evaluated: searcher.num_nodes_evaluated,
        highest_successful_depth: depth,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Information about the search which will be returned at the end of a search.
pub struct SearchInfo {
    /// The best move in the position.
    pub best_move: Move,
    /// The evaluation of the position.
    pub eval: Eval,
    /// The number of times a transposition table get was successful.
    pub num_transpositions: u64,
    /// The number of nodes evaluated in this search.
    pub num_nodes_evaluated: u64,
    /// The highest depth at which this search succeeded.
    pub highest_successful_depth: u8,
}

impl SearchInfo {
    /// Unify with another `SearchInfo`, selecting the most accurate evaluation
    /// (by depth) and summing the number of transpositions and nodes evaluated.
    pub fn unify_with(&mut self, other: &SearchInfo) {
        if other.highest_successful_depth > self.highest_successful_depth {
            self.best_move = other.best_move;
            self.eval = other.eval;
            self.highest_successful_depth = other.highest_successful_depth;
        }
        self.num_nodes_evaluated += other.num_nodes_evaluated;
        self.num_transpositions += other.num_transpositions;
    }
}

#[derive(Clone, Debug)]
/// A structure containing data which is shared across function calls to a
/// principal variation search.
struct PVSearch<'a> {
    /// The transposition table.
    ttable: Arc<TTable>,
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
    limit: Arc<SearchLimit>,
    /// Whether this search is the main search.
    is_main: bool,
}

impl<'a> PVSearch<'a> {
    /// Construct a new PVSearch using a given transposition table,
    /// configuration, and limit. `is_main` is whether the thread is a main
    /// search, responsible for certain synchronization activities.
    pub fn new(
        ttable: Arc<TTable>,
        config: &'a SearchConfig,
        limit: Arc<SearchLimit>,
        is_main: bool,
    ) -> PVSearch {
        PVSearch {
            ttable,
            killer_moves: vec![Move::BAD_MOVE; config.depth as usize],
            num_nodes_evaluated: 0,
            nodes_since_limit_update: 0,
            num_transpositions: 0,
            config,
            limit,
            is_main,
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Use Principal Variation Search to evaluate the given game to a depth.
    /// This search uses Negamax, which inverts at every step to save on
    /// branches. This will return a lower bound on the position's value for
    /// the player to move, where said lower bound is exact if it is less than
    /// `beta_in`. `depth_to_go` is signed because late-move-reduction may
    /// cause it to become negative. In the case where the search returns
    /// `Err`, the moves on `g` will not be correctly undone, so it is strongly
    /// recommended to pass in a reference to a copy of your original game.
    pub fn pvs(
        &mut self,
        depth_to_go: i8,
        depth_so_far: u8,
        g: &mut Game,
        alpha_in: Eval,
        beta_in: Eval,
        allow_reduction: bool,
    ) -> PVSResult {
        if self.limit.is_over() {
            return Err(SearchError::Timeout);
        }

        if self.is_main {
            self.limit.update_time()?;
        }

        if alpha_in >= Eval::mate_in(1) {
            // we do not need to evaluate this position because we are
            // guaranteed a mate which is as fast or faster elsewhere.
            return Ok((Move::BAD_MOVE, Eval::mate_in(2)));
        }

        // Lower bound on evaluation.
        let mut alpha = alpha_in;
        // Upper bound on evaluation.
        let mut beta = beta_in;

        // Retrieve transposition data and use it to improve our estimate on
        // the position
        let mut stored_move = None;
        if depth_so_far <= self.config.max_transposition_depth {
            if let Some(edata) = self.ttable.get(g.board().hash) {
                self.num_transpositions += 1;
                let m = edata.critical_move;
                stored_move = Some(m);
                if edata.lower_bound == edata.upper_bound && edata.lower_bound.is_mate() {
                    // searching deeper will not find us an escape from or a
                    // faster mate if the fill tree was searched
                    return Ok((m, edata.lower_bound));
                }
                if edata.depth >= depth_to_go {
                    // this was a deeper search on the position
                    if edata.lower_bound >= beta_in {
                        return Ok((m, edata.lower_bound));
                    }
                    if edata.upper_bound <= alpha_in {
                        return Ok((m, edata.upper_bound));
                    }
                    alpha = max(alpha, edata.lower_bound);
                    beta = min(beta, edata.upper_bound);
                }
            }
        }

        if depth_to_go <= 0 {
            return self.quiesce(depth_to_go, depth_so_far, g, alpha_in, beta_in);
        }

        self.increment_nodes()?;

        if g.is_drawn_historically() {
            // required so that movepicker only needs to know about current
            // position, and not about history
            return Ok((Move::BAD_MOVE, Eval::DRAW));
        }

        let killer_index = depth_so_far as usize;
        let can_use_killers = depth_so_far < self.config.depth;
        let killer_move = match can_use_killers {
            true => Some(self.killer_moves[killer_index]),
            false => None,
        };

        let mut moves_iter = MovePicker::new(*g.position(), stored_move, killer_move);

        // perform one search to satisfy PVS

        // since no other moves were searched, there must be something left
        // in the move picker for us unless the game is over
        let (m, delta) = match moves_iter.next() {
            Some(x) => x,
            None => {
                return Ok((
                    Move::BAD_MOVE,
                    leaf_evaluate(g) * (1 - 2 * g.board().player_to_move as i16),
                ))
            }
        };
        // best move found so far
        let mut best_move = m;
        g.make_move(m, delta);
        // best score so far
        let mut best_score = -self
            .pvs(
                depth_to_go - 1,
                depth_so_far + 1,
                g,
                -beta.step_forward(),
                -alpha.step_forward(),
                allow_reduction,
            )?
            .1
            .step_back();
        #[allow(unused_must_use)]
        {
            g.undo();
        }
        alpha = max(best_score, alpha);
        if alpha >= beta {
            if can_use_killers {
                self.killer_moves[killer_index] = m;
            }
            if depth_so_far <= self.config.max_transposition_depth {
                self.ttable_store(g, depth_to_go, alpha, beta, best_score, best_move);
            }
            return Ok((best_move, best_score));
        }

        for (idx, (m, delta)) in moves_iter.enumerate() {
            let late_move = idx > self.config.num_early_moves
                && !g.board().is_move_capture(m)
                && m.promote_type().is_none()
                && allow_reduction;
            g.make_move(m, delta);
            // zero-window search
            let depth_to_search = match late_move {
                true => depth_to_go - 2,
                false => depth_to_go - 1,
            };
            let mut score = -self
                .pvs(
                    depth_to_search,
                    depth_so_far + 1,
                    g,
                    -alpha.step_forward() - Eval::centipawns(1),
                    -alpha.step_forward(),
                    allow_reduction,
                )?
                .1
                .step_back();
            if alpha < score && score < beta {
                // zero-window search failed high, so there is a better option
                // in this tree. we already have a score from before that we
                // can use as a lower bound in this search.
                let position_lower_bound = match late_move {
                    // if this was a late move, we can't use the previous
                    // fail-high
                    true => -alpha.step_forward(),
                    false => -score.step_forward(),
                };
                score = -self
                    .pvs(
                        depth_to_go - 1,
                        depth_so_far + 1,
                        g,
                        -beta.step_forward(),
                        position_lower_bound,
                        allow_reduction,
                    )?
                    .1
                    .step_back();
            }
            #[allow(unused_must_use)]
            {
                g.undo();
            }
            if score > best_score {
                best_move = m;
                best_score = score;
                alpha = max(score, alpha);
                if alpha >= beta {
                    // Beta cutoff, we have  found a better line somewhere else
                    if can_use_killers {
                        self.killer_moves[killer_index] = m;
                    }
                    break;
                }
            }
        }

        if depth_so_far <= self.config.max_transposition_depth {
            self.ttable_store(g, depth_to_go, alpha, beta, best_score, best_move);
        }

        Ok((best_move, alpha))
    }

    #[allow(clippy::too_many_arguments)]
    /// Use quiescent search (captures only) to evaluate a position as deep as
    /// it needs to go. The given `depth_to_go` does not alter the power of the
    /// search, but serves as a handy tool for the search to understand where
    /// it is.
    fn quiesce(
        &mut self,
        depth_to_go: i8,
        depth_so_far: u8,
        g: &mut Game,
        alpha_in: Eval,
        beta_in: Eval,
    ) -> PVSResult {
        let player = g.board().player_to_move;

        if alpha_in >= Eval::mate_in(1) {
            // we do not need to evaluate this position because we are
            // guaranteed a mate which is as fast or faster elsewhere.
            return Ok((Move::BAD_MOVE, Eval::mate_in(2)));
        }

        // Any position where the king is in check is nowhere near quiet
        // enough to evaluate.
        if !g.position().check_info.checkers.is_empty() {
            return self.pvs(1, depth_so_far, g, alpha_in, beta_in, false);
        }

        self.increment_nodes()?;

        // capturing is unforced, so we can stop here if the player to move
        // doesn't want to capture.
        let leaf_evaluation = leaf_evaluate(g);
        /*if g.is_over().0 {
            println!("{g}: {leaf_evaluation}");
        }*/
        // Put the score in perspective of the player.
        let mut score = leaf_evaluation.in_perspective(player);
        let mut alpha = alpha_in;
        let beta = beta_in;

        alpha = max(score, alpha);
        if alpha >= beta {
            // beta cutoff, this line would not be selected because there is a
            // better option somewhere else
            return Ok((Move::BAD_MOVE, alpha));
        }

        let mut moves = get_moves::<CAPTURES, PstNominate>(g.position());
        moves.sort_by_cached_key(|&(_, (_, eval))| -eval);
        let mut best_move = Move::BAD_MOVE;

        for (m, (delta, _)) in moves {
            g.make_move(m, delta);
            // zero-window search
            score = -self
                .quiesce(
                    depth_to_go - 1,
                    depth_so_far + 1,
                    g,
                    -alpha.step_forward() - Eval::centipawns(1),
                    -alpha.step_forward(),
                )?
                .1
                .step_back();
            if alpha < score && score < beta {
                // zero-window search failed high, so there is a better option
                // in this tree. we already have a score from before that we
                // can use as a lower bound in this search.
                score = -self
                    .quiesce(
                        depth_to_go - 1,
                        depth_so_far + 1,
                        g,
                        -beta.step_forward(),
                        -score.step_forward(),
                    )?
                    .1
                    .step_back();
                best_move = m;
            }
            #[allow(unused_must_use)]
            {
                g.undo();
            }
            alpha = max(alpha, score);
            if alpha >= beta {
                // Beta cutoff, we have  found a better line somewhere else
                break;
            }
        }

        Ok((best_move, alpha))
    }

    /// Store data in the transposition table.
    /// `score` is the best score of the position as evaluated, while `alpha`
    /// and `beta` are the upper and lower bounds on the overall position due
    /// to alpha-beta pruning.
    fn ttable_store(
        &mut self,
        g: &Game,
        depth: i8,
        alpha: Eval,
        beta: Eval,
        score: Eval,
        best_move: Move,
    ) {
        let upper_bound = match score < beta {
            true => score,
            false => Eval::MAX,
        };
        let lower_bound = match alpha < score {
            true => score,
            false => Eval::MIN,
        };
        self.ttable.store(
            g.board().hash,
            EvalData {
                depth,
                lower_bound,
                upper_bound,
                critical_move: best_move,
            },
        );
    }

    #[inline(always)]
    /// Increment the number of nodes searched, copying over the value into the
    /// search limit if it is too high.
    fn increment_nodes(&mut self) -> Result<(), SearchError> {
        self.num_nodes_evaluated += 1;
        self.nodes_since_limit_update += 1;
        if self.nodes_since_limit_update as u64 > self.config.limit_update_increment {
            self.update_node_limits()?;
        }
        Ok(())
    }

    #[inline(always)]
    /// Copy over the number of nodes evaluated by this search into the limit
    /// structure, and zero out our number.
    fn update_node_limits(&mut self) -> Result<(), SearchError> {
        self.limit.add_nodes(self.nodes_since_limit_update as u64)?;
        self.nodes_since_limit_update = 0;
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::evaluate::static_evaluate;
    use fiddler_base::Move;
    use fiddler_base::Square;

    /// Helper function to search a position at a given depth.
    ///
    /// # Panics
    ///
    /// This function will panic if searching the position fails or the game is
    /// invalid.
    fn search_helper(fen: &str, depth: u8) -> SearchInfo {
        let g = Game::from_fen(fen, static_evaluate).unwrap();
        let config = SearchConfig {
            depth,
            ..Default::default()
        };
        search(
            g,
            depth,
            Arc::new(TTable::default()),
            &config,
            Arc::new(SearchLimit::default()),
            true,
        )
        .unwrap()
    }

    #[test]
    /// Test PVSearch's evaluation of the start position of the game.
    pub fn test_eval_start() {
        let info = search_helper(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            11,
        );
        println!("best move: {} [{}]", info.best_move, info.eval);
    }

    #[test]
    /// A test on the evaluation of the game in the fried liver position. The
    /// only winning move for White is Qd3+.
    fn test_fried_liver() {
        let info = search_helper(
            "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
            10,
        );
        let m = Move::normal(Square::D1, Square::F3);

        assert_eq!(info.best_move, m);
    }

    /// A helper function which ensures that the evaluation of a position is
    /// equal to what we expect it to be. It will check both a normal search
    /// and a search without the transposition table.
    fn test_eval_helper(fen: &str, eval: Eval, depth: u8) {
        assert_eq!(search_helper(fen, depth).eval, eval);
    }

    #[test]
    /// A test that the engine can find a mate in 1 move.
    fn test_mate_in_1() {
        // Rb8# is mate in one
        test_eval_helper("3k4/R7/1R6/5K2/8/8/8/8 w - - 0 1", Eval::mate_in(1), 2);
    }

    #[test]
    /// A test that shows the engine can find a mate in 4 plies, given enough
    /// depth.
    fn test_mate_in_4_ply() {
        // because black, the player to move, is getting mated, the evaluation
        // is negative here
        test_eval_helper("3k4/R7/8/5K2/3R4/8/8/8 b - - 0 1", -Eval::mate_in(4), 5);
    }

    #[test]
    /// A test for a puzzle made by Ian. White has mate in 5 with Rxf7+.
    fn test_mate_in_9_ply() {
        // because capturing a low-value piece is often a "late" move, it is 
        // likely to be reduced in depth
        test_eval_helper(
            "2r2r2/3p1p1k/p3p1p1/3P3n/q3P1Q1/1p5P/1PP2R2/1K4R1 w - - 0 30",
            Eval::mate_in(9),
            11,
        );
    }
}
