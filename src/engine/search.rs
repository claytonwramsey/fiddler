use crate::base::algebraic::algebraic_from_move;
use crate::base::{Game, Move, MoveGenerator};
use crate::engine::positional::positional_evaluate;
use crate::engine::transposition::{EvalData, TTable};
use crate::engine::{Eval, EvaluationFn, MoveCandidacyFn};
use crate::Engine;

use std::cmp::{max, min};
use std::collections::HashMap;
use std::time::Instant;

use super::TimeoutCondition;

const MAX_TRANSPOSITION_DEPTH: i8 = 5;

#[allow(unused)]
///
/// The number of moves which are searched to a full depth before applying Late
/// Move Evaluation.
///
const NUM_EARLY_MOVES: u8 = 4;

///
/// A chess engine which uses Principal Variation Search.
///
pub struct PVSearch {
    ///
    /// The depth at which this algorithm will evaluate a position.
    ///
    depth: i8,
    ///
    /// The function used to evaluate the quality of a position.
    ///
    pub evaluator: EvaluationFn,
    ///
    /// The function used to determine which moves should be explored first.
    ///
    pub candidator: MoveCandidacyFn,
    ///
    /// The transposition table.
    ///
    ttable: TTable,
    ///
    /// The set of "killer" moves. Each index corresponds to a depth (0 is most
    /// shallow, etc).
    ///
    killer_moves: Vec<Move>,
    ///
    /// The cumulative number of nodes evaluated in this evaluation event.
    ///
    num_nodes_evaluated: u64,
    ///
    /// The cumulative number of transpositions.
    ///
    num_transpositions: u64,
}

impl PVSearch {
    #[allow(clippy::too_many_arguments)]
    ///
    /// Use Principal Variation Search to evaluate the given game to a depth.
    /// This search uses Negamax, which inverts at every step to save on
    /// branches. This will return a lower bound on the position's value for
    /// the player to move, where said lower bound is exact if it is less than
    /// `beta_in`.
    ///
    pub fn pvs(
        &mut self,
        depth_to_go: i8,
        depth_so_far: i8,
        g: &mut Game,
        mgen: &MoveGenerator,
        alpha_in: Eval,
        beta_in: Eval,
        timeout: &dyn TimeoutCondition,
    ) -> (Move, Eval) {
        self.num_nodes_evaluated += 1;

        if timeout.is_over() {
            return (Move::BAD_MOVE, Eval(0));
        }

        if alpha_in >= Eval::mate_in(1) {
            // we do not need to evaluate this position because we are
            // guaranteed a mate which is as fast or faster elsewhere.
            return (Move::BAD_MOVE, Eval::mate_in(1));
        }

        // Lower bound on evaluation.
        let mut alpha = alpha_in;
        // Upper bound on evaluation.
        let mut beta = beta_in;

        // Retrieve transposition data and use it to improve our estimate on
        // the position
        let mut stored_move = Move::BAD_MOVE;
        if depth_so_far <= MAX_TRANSPOSITION_DEPTH {
            if let Some(edata) = self.ttable[g.get_board()] {
                self.num_transpositions += 1;
                stored_move = edata.critical_move;
                if edata.lower_bound == edata.upper_bound && edata.lower_bound.is_mate() {
                    // searching deeper will not find us an escape from or a
                    // faster mate if the fill tree was searched
                    return (stored_move, edata.lower_bound);
                }
                if edata.depth >= depth_to_go {
                    // this was a deeper search on the position
                    if edata.lower_bound >= beta_in {
                        return (stored_move, edata.lower_bound);
                    }
                    if edata.upper_bound <= alpha_in {
                        return (stored_move, edata.upper_bound);
                    }
                    alpha = max(alpha, edata.lower_bound);
                    beta = min(beta, edata.upper_bound);
                }
            }
        }

        if depth_to_go <= 0 {
            return self.quiesce(depth_to_go, depth_so_far, g, mgen, alpha_in, beta_in);
        }

        let mut moves = g.get_moves(mgen);

        if moves.is_empty() {
            return (
                Move::BAD_MOVE,
                (self.evaluator)(g, mgen) * (1 - 2 * g.get_board().player_to_move as i32),
            );
        }

        // Sort moves so that the most promising move is evaluated first
        let killer_index = depth_so_far as usize;
        let retrieved_killer_move = self.killer_moves[killer_index];
        moves.sort_by_cached_key(|m| {
            if *m == stored_move {
                return Eval::MIN;
            }
            if *m == retrieved_killer_move {
                return Eval::MIN + Eval(1);
            }
            -(self.candidator)(g, mgen, *m)
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
                timeout,
            )
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
            self.killer_moves[killer_index] = critical_move;
            if depth_so_far <= MAX_TRANSPOSITION_DEPTH {
                self.ttable_store(
                    g,
                    depth_to_go,
                    alpha,
                    beta,
                    best_score_this_position,
                    critical_move,
                );
            }
            return (critical_move, alpha);
        }

        let mut num_moves_checked = 1;

        for m in moves_iter {
            let late_move = num_moves_checked > NUM_EARLY_MOVES
                && !g.get_board().is_move_capture(m)
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
                    -alpha.step_forward() - Eval(1),
                    -alpha.step_forward(),
                    timeout,
                )
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
                        timeout,
                    )
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
                self.killer_moves[killer_index] = m;
                break;
            }

            num_moves_checked += 1;
        }

        if depth_so_far <= MAX_TRANSPOSITION_DEPTH {
            self.ttable_store(
                g,
                depth_to_go,
                alpha,
                beta,
                best_score_this_position,
                critical_move,
            );
        }

        (critical_move, alpha)
    }

    #[allow(clippy::too_many_arguments)]
    ///
    /// Use quiescent search (captures only) to evaluate a position as deep as
    /// it needs to go. The given `depth_to_go` does not alter the power of the
    /// search, but serves as a handy tool for the search to understand where
    /// it is.
    ///
    fn quiesce(
        &mut self,
        depth_to_go: i8,
        depth_so_far: i8,
        g: &mut Game,
        mgen: &MoveGenerator,
        alpha_in: Eval,
        beta_in: Eval,
    ) -> (Move, Eval) {
        self.num_nodes_evaluated += 1;

        let player = g.get_board().player_to_move;
        let mut moves = g.get_loud_moves(mgen);

        // capturing is unforced, so we can stop here if the player to move
        // doesn't want to capture.
        let leaf_evaluation = (self.evaluator)(g, mgen);
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
            return (Move::BAD_MOVE, beta);
        }

        moves.sort_by_cached_key(|m| -(self.candidator)(g, mgen, *m));
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
                )
                .1
                .step_back();
            #[allow(unused_must_use)]
            {
                g.undo();
            }

            alpha = max(alpha, score);
            if alpha >= beta {
                // Beta cutoff, we have found a better line somewhere else
                return (critical_move, alpha);
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
                    -alpha.step_forward() - Eval(1),
                    -alpha.step_forward(),
                )
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
                    )
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

        (critical_move, alpha)
    }

    ///
    /// Clear out internal data.
    ///
    pub fn clear(&mut self) {
        self.num_nodes_evaluated = 0;
        self.num_transpositions = 0;
        self.ttable.clear();
    }

    ///
    /// Store data in the transposition table.
    /// `score` is the best score of the position as evaluated, while `alpha`
    /// and `beta` are the upper and lower bounds on the overall position due
    /// to alpha-beta pruning.
    ///
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
            *g.get_board(),
            EvalData {
                depth,
                lower_bound,
                upper_bound,
                critical_move,
            },
        );
    }
}

impl Default for PVSearch {
    fn default() -> PVSearch {
        let mut searcher = PVSearch {
            depth: 0,
            evaluator: positional_evaluate,
            candidator: crate::engine::candidacy::candidacy,
            ttable: TTable::default(),
            killer_moves: Vec::new(),
            num_nodes_evaluated: 0,
            num_transpositions: 0,
        };
        searcher.set_depth(5);
        searcher
    }
}

impl Engine for PVSearch {
    fn set_depth(&mut self, depth: usize) {
        self.depth = depth as i8;
        for _ in 0..depth {
            self.killer_moves.push(Move::BAD_MOVE);
        }
    }

    #[inline]
    fn evaluate(
        &mut self,
        g: &mut Game,
        mgen: &MoveGenerator,
        timeout: &dyn TimeoutCondition,
    ) -> Eval {
        self.num_nodes_evaluated = 0;
        self.num_transpositions = 0;
        let tic = Instant::now();
        let iter_min = min(4, self.depth);
        let mut iter_depth = iter_min;
        let mut eval = Eval(0);
        let mut highest_successful_depth = 0;
        let mut successful_nodes_evaluated = 0;
        while iter_depth <= self.depth && !timeout.is_over() {
            let mut search_result = self.pvs(iter_depth, 0, g, mgen, Eval::MIN, Eval::MAX, timeout);
            search_result.1 *= 1 - 2 * g.get_board().player_to_move as i32;
            if !timeout.is_over() {
                highest_successful_depth = iter_depth;
                eval = search_result.1;
                successful_nodes_evaluated = self.num_nodes_evaluated;
            }
            iter_depth += 1;
        }
        let toc = Instant::now();
        let nsecs = (toc - tic).as_secs_f64();
        println!(
            "evaluated {:.0} nodes in {:.0} secs ({:.0} nodes/sec) with {:0} transpositions; branch factor {:.2}",
            self.num_nodes_evaluated,
            nsecs,
            self.num_nodes_evaluated as f64 / nsecs,
            self.num_transpositions,
            branch_factor(highest_successful_depth, successful_nodes_evaluated)
        );

        eval
    }

    fn get_evals(
        &mut self,
        g: &mut Game,
        mgen: &MoveGenerator,
        timeout: &dyn TimeoutCondition,
    ) -> HashMap<Move, Eval> {
        let mut moves = g.get_moves(mgen);
        //negate because sort is ascending
        moves.sort_by_cached_key(|m| -(self.candidator)(g, mgen, *m));
        let mut evals = HashMap::new();
        for m in moves {
            g.make_move(m);
            let ev = self.evaluate(g, mgen, timeout);

            //this should never fail since we just made a move, but who knows?
            if g.undo().is_ok() {
                evals.insert(m, ev);
            } else {
                println!("somehow, undoing failed on a game");
            }
            println!("{}: {ev}", algebraic_from_move(m, g.get_board(), mgen));
        }

        evals
    }

    fn get_best_move(
        &mut self,
        g: &mut Game,
        mgen: &MoveGenerator,
        timeout: &dyn TimeoutCondition,
    ) -> Move {
        self.num_nodes_evaluated = 0;
        self.num_transpositions = 0;
        let tic = Instant::now();
        let iter_min = min(4, self.depth);

        let mut best_move = Move::BAD_MOVE;
        let mut eval;
        let mut eval_uncalibrated;
        let mut iter_depth = iter_min;
        let mut highest_successful_depth = 0;
        let mut successful_nodes_evaluated = 0;
        while iter_depth <= self.depth && !timeout.is_over() {
            let result = self.pvs(iter_depth, 0, g, mgen, Eval::MIN, Eval::MAX, timeout);
            if !timeout.is_over() {
                highest_successful_depth = iter_depth;
                successful_nodes_evaluated = self.num_nodes_evaluated;
                best_move = result.0;
                eval_uncalibrated = result.1;
                eval = eval_uncalibrated * (1 - 2 * g.get_board().player_to_move as i32);
                println!(
                    "depth {iter_depth} gives {}: {eval}",
                    algebraic_from_move(best_move, g.get_board(), mgen)
                );
            }
            iter_depth += 1;
        }
        let toc = Instant::now();
        let nsecs = (toc - tic).as_secs_f64();
        // Note that the print statements in iterative deepening take a
        // significant amount of time.
        println!(
            "evaluated {:.0} nodes in {:.0} secs ({:.0} nodes/sec) with {:0} transpositions; branch factor {:.2}",
            self.num_nodes_evaluated,
            nsecs,
            self.num_nodes_evaluated as f64 / nsecs,
            self.num_transpositions,
            branch_factor(highest_successful_depth, successful_nodes_evaluated),
        );

        best_move
    }
}

#[inline]
///
/// Compute the effective branch factor given a given search depth and a number
/// of nodes evaluated.
///
fn branch_factor(depth: i8, num_nodes: u64) -> f64 {
    (num_nodes as f64).powf(1f64 / (depth as f64))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::base::moves::Move;
    use crate::base::square::*;
    use crate::engine::NoTimeout;
    use crate::fens::*;
    use std::collections::HashMap;

    #[test]
    ///
    /// Test PVSearch's evaluation of the start position of the game.
    ///
    pub fn test_eval_start() {
        let mut g = Game::default();
        let mgen = MoveGenerator::default();
        let mut e = PVSearch::default();
        e.set_depth(5); // this prevents taking too long on searches

        println!("moves with evals are:");
        e.get_evals(&mut g, &mgen, &NoTimeout);
    }

    #[test]
    ///
    /// Try finding the best starting move in the game.
    ///
    pub fn test_get_starting_move() {
        let mut g = Game::default();
        let mgen = MoveGenerator::default();
        let mut e = PVSearch::default();
        e.set_depth(8);

        e.get_best_move(&mut g, &mgen, &NoTimeout);
    }

    #[test]
    ///
    /// A test on the evaluation of the game in the fried liver position. The
    /// only winning move for White is Qd3+.
    ///
    fn test_fried_liver() {
        let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
        let mgen = MoveGenerator::default();
        let mut e = PVSearch::default();
        e.set_depth(6); // this prevents taking too long on searches

        assert_eq!(
            e.get_best_move(&mut g, &mgen, &NoTimeout),
            Move::normal(Square::D1, Square::F3)
        );
    }

    #[test]
    ///
    /// A test that the engine can find a mate in 1 move.
    ///
    fn test_mate_in_1() {
        test_eval_helper(MATE_IN_1_FEN, Eval::mate_in(1), 2);
    }

    #[test]
    ///
    /// A test that shows the engine can find a mate in 4 plies, given enough
    /// depth.
    ///
    fn test_mate_in_4_ply() {
        test_eval_helper(MATE_IN_4_FEN, Eval::mate_in(4), 5);
    }

    #[test]
    ///
    /// A test for a puzzle made by Ian. White has mate in 5 with Rxf7+.
    ///
    fn test_my_special_puzzle() {
        let mut g = Game::from_fen(MY_PUZZLE_FEN).unwrap();
        let mgen = MoveGenerator::default();
        let mut e = PVSearch::default();
        e.set_depth(9);

        assert_eq!(
            e.get_best_move(&mut g, &mgen, &NoTimeout),
            Move::normal(Square::F2, Square::F7)
        );
    }

    ///
    /// A helper function which ensures that the evaluation of a position is
    /// equal to what we expect it to be.
    ///
    fn test_eval_helper(fen: &str, eval: Eval, depth: usize) {
        let mut g = Game::from_fen(fen).unwrap();
        let mgen = MoveGenerator::default();
        let mut e = PVSearch::default();
        e.set_depth(depth);

        assert_eq!(e.evaluate(&mut g, &mgen, &NoTimeout), eval);
    }

    #[allow(unused)]
    ///
    /// Print a map from moves to evals in a user-readable way.
    ///
    fn print_move_map(map: &HashMap<Move, Eval>) {
        for (m, eval) in map {
            println!("{m}:{eval}");
        }
    }
}
