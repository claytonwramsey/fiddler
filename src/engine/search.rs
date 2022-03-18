use crate::base::algebraic::algebraic_from_move;
use crate::base::Eval;
use crate::base::{Game, Move, MoveGenerator};
use crate::engine::evaluate::evaluate;
use crate::engine::transposition::{EvalData, TTable};

use std::cmp::{max, min};
use std::sync::{Arc, RwLock};

use super::{SearchResult, SearchError};
use super::candidacy::candidacy;
use super::config::SearchConfig;
use super::limit::{ArcLimit, SearchLimit};

#[derive(Clone, Debug)]
/// A chess engine which uses Principal Variation Search.
pub struct PVSearch {
    /// The transposition table.
    pub ttable: Arc<TTable>,

    /// The set of "killer" moves. Each index corresponds to a depth (0 is most
    /// shallow, etc).
    killer_moves: Vec<Move>,

    /// The cumulative number of nodes evaluated in this evaluation event since
    /// the search limit was last updated.
    num_nodes_evaluated: u64,

    /// The cumulative number of transpositions.
    num_transpositions: u64,

    /// The configuration of this search.
    pub config: SearchConfig,

    /// The limit to this search.
    pub limit: ArcLimit,
}

/// The output type of a search. An `Err` may be given if, for instance,
/// the search times out.
type PVSResult = Result<(Move, Eval), SearchError>;

impl PVSearch {

    /// Construct a new PVSearch using a given transposition table, 
    /// configuration, and limit.
    pub fn new(ttable: Arc<TTable>, config: SearchConfig, limit: ArcLimit) -> PVSearch {
        PVSearch { 
            ttable, 
            killer_moves: vec![Move::BAD_MOVE; config.depth as usize],
            num_nodes_evaluated: 0, 
            num_transpositions: 0, 
            config, 
            limit,
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
        mgen: &MoveGenerator,
        alpha_in: Eval,
        beta_in: Eval,
    ) -> PVSResult {
        if self.is_over()? {
            return Err(SearchError::TimeoutError);
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
        let mut stored_move = Move::BAD_MOVE;
        if depth_so_far <= self.config.max_transposition_depth {
            if let Some(edata) = self.ttable.get(g.board().hash) {
                self.num_transpositions += 1;
                stored_move = edata.critical_move;
                if edata.lower_bound == edata.upper_bound && edata.lower_bound.is_mate() {
                    // searching deeper will not find us an escape from or a
                    // faster mate if the fill tree was searched
                    return Ok((stored_move, edata.lower_bound));
                }
                if edata.depth >= depth_to_go {
                    // this was a deeper search on the position
                    if edata.lower_bound >= beta_in {
                        return Ok((stored_move, edata.lower_bound));
                    }
                    if edata.upper_bound <= alpha_in {
                        return Ok((stored_move, edata.upper_bound));
                    }
                    alpha = max(alpha, edata.lower_bound);
                    beta = min(beta, edata.upper_bound);
                }
            }
        }

        if depth_to_go <= 0 {
            return self.quiesce(depth_to_go, depth_so_far, g, mgen, alpha_in, beta_in);
        }

        self.increment_nodes()?;

        let mut moves = g.get_moves(mgen);

        if moves.is_empty() {
            return Ok((
                Move::BAD_MOVE,
                evaluate(g, mgen) * (1 - 2 * g.board().player_to_move as i32),
            ));
        }

        // Sort moves so that the most promising move is evaluated first
        let killer_index = depth_so_far as usize;
        let can_use_killers = depth_so_far < self.config.depth;
        let mut retrieved_killer_move = Move::BAD_MOVE;
        if can_use_killers {
            retrieved_killer_move = self.killer_moves[killer_index];
        }
        moves.sort_by_cached_key(|m| {
            if *m == stored_move {
                return Eval::MIN;
            }
            if *m == retrieved_killer_move {
                return Eval::MIN + Eval::millipawns(1);
            }
            -candidacy(g, mgen, *m)
        });

        let mut moves_iter = moves.into_iter();

        // This should always have a move since this was not a "terminal"
        // position of the game
        let mut critical_move = moves_iter.next().unwrap();

        g.make_move(critical_move);
        let mut score = -self
            .pvs(
                depth_to_go - 1,
                depth_so_far + 1,
                g,
                mgen,
                -beta.step_forward(),
                -alpha.step_forward(),
            )?
            .1
            .step_back();
        #[allow(unused_must_use)]
        {
            g.undo();
        }
        let mut best_score_this_position = score;

        alpha = max(alpha, score);
        if alpha >= beta {
            // Beta cutoff, we have found a better line somewhere else
            if can_use_killers {
                self.killer_moves[killer_index] = critical_move;
            }
            if depth_so_far <= self.config.max_transposition_depth {
                self.ttable_store(
                    g,
                    depth_to_go,
                    alpha,
                    beta,
                    best_score_this_position,
                    critical_move,
                );
            }
            return Ok((critical_move, alpha));
        }

        let mut num_moves_checked = 1;

        for m in moves_iter {
            let late_move = num_moves_checked > self.config.num_early_moves
                && !g.board().is_move_capture(m)
                && m.promote_type().is_none();
            g.make_move(m);
            // zero-window search
            let depth_to_search = match late_move {
                true => depth_to_go - 2,
                false => depth_to_go - 1,
            };
            score = -self
                .pvs(
                    depth_to_search,
                    depth_so_far + 1,
                    g,
                    mgen,
                    -alpha.step_forward() - Eval::millipawns(1),
                    -alpha.step_forward(),
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
                        mgen,
                        -beta.step_forward(),
                        position_lower_bound,
                    )?
                    .1
                    .step_back();
            }
            #[allow(unused_must_use)]
            {
                g.undo();
            }
            if score > best_score_this_position {
                critical_move = m;
                best_score_this_position = score;
                alpha = max(score, alpha);
            }
            if alpha >= beta {
                // Beta cutoff, we have  found a better line somewhere else
                if can_use_killers {
                    self.killer_moves[killer_index] = m;
                }
                break;
            }

            num_moves_checked += 1;
        }

        if depth_so_far <= self.config.max_transposition_depth {
            self.ttable_store(
                g,
                depth_to_go,
                alpha,
                beta,
                best_score_this_position,
                critical_move,
            );
        }

        Ok((critical_move, alpha))
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
        mgen: &MoveGenerator,
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
        if g.board().is_king_checked(mgen) {
            return self.pvs(1, depth_so_far, g, mgen, alpha_in, beta_in);
        }

        self.increment_nodes()?;

        let mut moves = g.get_loud_moves(mgen);

        // capturing is unforced, so we can stop here if the player to move
        // doesn't want to capture.
        let leaf_evaluation = evaluate(g, mgen);
        // (1 - 2 * us) will cause the evaluation to be positive for
        // whichever player is moving. This will cascade up the Negamax
        // inversions to make the final result at the top correct.
        // This step must also be done at the top level so that positions
        // with Black to move are evaluated as negative when faced
        // outwardly.
        let mut score = leaf_evaluation * (1 - 2 * player as i32);
        let mut alpha = alpha_in;
        let beta = beta_in;

        alpha = max(score, alpha);
        if alpha >= beta {
            // beta cutoff, this line would not be selected because there is a
            // better option somewhere else
            return Ok((Move::BAD_MOVE, alpha));
        }

        moves.sort_by_cached_key(|m| -candidacy(g, mgen, *m));
        let mut moves_iter = moves.into_iter();
        let mut critical_move = Move::BAD_MOVE;
        // we must wrap with an if in case there are no captures
        if let Some(critical_move) = moves_iter.next() {
            g.make_move(critical_move);
            score = -self
                .quiesce(
                    depth_to_go - 1,
                    depth_so_far + 1,
                    g,
                    mgen,
                    -beta.step_forward(),
                    -alpha.step_forward(),
                )?
                .1
                .step_back();
            #[allow(unused_must_use)]
            {
                g.undo();
            }

            alpha = max(alpha, score);
            if alpha >= beta {
                // Beta cutoff, we have found a better line somewhere else
                return Ok((critical_move, alpha));
            }
        }

        for m in moves_iter {
            g.make_move(m);
            // zero-window search
            score = -self
                .quiesce(
                    depth_to_go - 1,
                    depth_so_far + 1,
                    g,
                    mgen,
                    -alpha.step_forward() - Eval::millipawns(1),
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
                        mgen,
                        -beta.step_forward(),
                        -score.step_forward(),
                    )?
                    .1
                    .step_back();
                critical_move = m;
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

        Ok((critical_move, alpha))
    }

    /// Clear out internal data.
    pub fn clear(&mut self) {
        self.num_nodes_evaluated = 0;
        self.num_transpositions = 0;
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
        critical_move: Move,
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
            *g.board(),
            EvalData {
                depth,
                lower_bound,
                upper_bound,
                critical_move,
            },
        );
    }

    /// Set the search depth of the engine. This is preferred over strictly
    /// mutating the engine, as the depth may alter some data structures used
    /// by the engine.
    pub fn set_depth(&mut self, depth: u8) {
        self.config.depth = depth;
        for _ in 0..depth {
            self.killer_moves.push(Move::BAD_MOVE);
        }
    }

    #[inline]
    /// Evaluate the given game. Return a pair containing the best move and its 
    /// evaluation, as well as the depth to which the evaluation was searched.
    pub fn evaluate(&mut self, g: &Game, mgen: &MoveGenerator) -> SearchResult {
        self.num_nodes_evaluated = 0;
        self.num_transpositions = 0;
        let mut gcopy = g.clone();
        let mut result = (Move::BAD_MOVE, Eval::DRAW);
        let mut highest_successful_depth = 0;
        for iter_depth in 1..=self.config.depth {
            match self.pvs(
                iter_depth as i8, 
                0, 
                &mut gcopy, 
                mgen, 
                Eval::MIN, 
                Eval::MAX
            ) {
                Ok(search_result) => {
                    result = (
                        search_result.0,
                        search_result.1 * (1 - 2 * gcopy.board().player_to_move as i32)
                    );
                    highest_successful_depth = iter_depth;
                    if iter_depth > 0 {
                        println!("depth {iter_depth} gives {}", algebraic_from_move(result.0, g.board(), mgen));
                    }
                },
                Err(e) => match e {
                    SearchError::TimeoutError => break,
                    SearchError::PoisonError => return Err(e),
                    SearchError::JoinError => panic!("how did a single-threaded process have a join error?!"),
                }
            };
        }

        if result.0 == Move::BAD_MOVE {
            // search timed out before it could come up with any good moves.
            return Err(SearchError::TimeoutError);
        }

        Ok((result.0, result.1, highest_successful_depth))
    }

    #[inline]
    /// Helper function to check whether our search limit has decided that we
    /// are done searching.
    fn is_over(&self) -> Result<bool, SearchError> {
        Ok(self.limit.read().map_err(|_| SearchError::PoisonError)?.is_over())
    }

    #[inline]
    /// Increment the number of nodes searched, copying over the value into the
    /// search limit if it is too high.
    fn increment_nodes(&mut self) -> Result<(), SearchError> {
        self.num_nodes_evaluated += 1;
        if self.num_nodes_evaluated > self.config.limit_update_increment {
            self.update_node_limits()?;
        }
        Ok(())
    }

    #[inline]
    /// Copy over the number of nodes evaluated by this search into the limit 
    /// structure, and zero out our number.
    fn update_node_limits(&mut self) -> Result<(), SearchError> {
        self.limit
                .write()
                .map_err(|_| SearchError::PoisonError)?
                .add_nodes(self.num_nodes_evaluated);
        self.num_nodes_evaluated = 0;
        Ok(())
    }
}

impl Default for PVSearch {
    fn default() -> PVSearch {
        let mut searcher = PVSearch {
            ttable: Arc::new(TTable::default()),
            killer_moves: Vec::new(),
            num_nodes_evaluated: 0,
            num_transpositions: 0,
            config: SearchConfig {
                depth: 0,
                max_transposition_depth: 8,
                num_early_moves: 4,
                limit_update_increment: 100,
            },
            limit: Arc::new(RwLock::new(SearchLimit::new())),
        };
        searcher.set_depth(5);
        searcher
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::base::moves::Move;
    use crate::base::square::*;
    use crate::fens::*;

    #[test]
    /// Test PVSearch's evaluation of the start position of the game.
    pub fn test_eval_start() {
        let g = Game::default();
        let mgen = MoveGenerator::default();
        let mut e = PVSearch::default();
        e.set_depth(11); // this prevents taking too long on searches

        let result = e.evaluate(&g, &mgen);
        println!("best move: {} [{}]",
            result.unwrap().0,
            result.unwrap().1
        );
    }

    #[test]
    /// A test on the evaluation of the game in the fried liver position. The
    /// only winning move for White is Qd3+.
    fn test_fried_liver() {
        let g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
        let mgen = MoveGenerator::default();
        let mut e = PVSearch::default();
        e.set_depth(6); // this prevents taking too long on searches

        assert_eq!(
            e.evaluate(&g, &mgen).unwrap().0,
            Move::normal(Square::D1, Square::F3)
        );
    }

    #[test]
    /// A test that the engine can find a mate in 1 move.
    fn test_mate_in_1() {
        test_eval_helper(MATE_IN_1_FEN, Eval::mate_in(1), 2);
    }

    #[test]
    /// A test that shows the engine can find a mate in 4 plies, given enough
    /// depth.
    fn test_mate_in_4_ply() {
        test_eval_helper(MATE_IN_4_FEN, Eval::mate_in(4), 5);
    }

    #[test]
    /// A test for a puzzle made by Ian. White has mate in 5 with Rxf7+.
    fn test_my_special_puzzle() {
        test_eval_helper(MY_PUZZLE_FEN, Eval::mate_in(9), 9);
    }

    /// A helper function which ensures that the evaluation of a position is
    /// equal to what we expect it to be. It will check both a normal search
    /// and a search without the transposition table.
    fn test_eval_helper(fen: &str, eval: Eval, depth: u8) {
        let g = Game::from_fen(fen).unwrap();
        let mgen = MoveGenerator::default();
        let mut e = PVSearch::default();
        e.set_depth(depth);

        assert_eq!(e.evaluate(&g, &mgen).unwrap().1, eval);
        e.config.max_transposition_depth = 0;
        e.clear();
        assert_eq!(e.evaluate(&g, &mgen).unwrap().1, eval);
    }
}
