use crate::base::algebraic::algebraic_from_move;
use crate::base::util::opposite_color;
use crate::base::{Game, Move, MoveGenerator};
use crate::engine::positional::positional_evaluate;
use crate::engine::transposition::{TTable, EvalData};
use crate::engine::{Eval, EvaluationFn, MoveCandidacyFn};
use crate::Engine;

use std::cmp::{max, min};
use std::collections::HashMap;
use std::time::Instant;

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
    ///
    /// Use Principal Variation Search to evaluate the givne game to a depth.
    /// This search uses Negamax, which inverts at every step to save on
    /// branches. This will return a lower bound on the position's value for 
    /// the player to move, where said lower bound is exact if it is less than 
    /// `beta_in`.
    ///
    pub fn pvs(
        &mut self,
        depth: i8,
        g: &mut Game,
        mgen: &MoveGenerator,
        alpha_in: Eval,
        beta_in: Eval,
    ) -> Eval {
        self.num_nodes_evaluated += 1;

        // Lower bound on evaluation.
        let mut alpha = alpha_in;
        // Upper bound on evaluation. 
        let mut beta = beta_in;

        // Retrieve transposition data and use it to improve our estimate on 
        // the position
        let mut stored_move = Move::BAD_MOVE;
        if let Some(edata) = self.ttable[g.get_board()] {
            self.num_transpositions += 1;
            stored_move = edata.critical_move;
            // this was a deeper search on the position
            if edata.depth >= depth {
                if edata.lower_bound >= beta_in {
                    return edata.lower_bound;
                }
                if edata.upper_bound <= alpha_in {
                    return edata.upper_bound;
                }
                alpha = max(alpha, edata.lower_bound);
                beta = min(beta, edata.upper_bound);
            }
        }

        if depth == 0 || g.is_game_over(mgen) {
            // (1 - 2 * us) will cause the evaluation to be positive for
            // whichever player is moving. This will cascade up the Negamax
            // inversions to make the final result at the top correct.
            // This step must also be done at the top level so that positions
            // with Black to move are evaluated as negative when faced
            // outwardly.
            return self.quiesce(depth, g, mgen, alpha_in, beta_in);
        }

        let mut moves = g.get_moves(mgen);

        // Sort moves so that the most promising move is evaluated first
        let killer_index = (self.depth - depth) as usize;
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
        let mut score = -self.pvs(
                depth - 1,
                g,
                mgen,
                -beta.step_forward(),
                -alpha.step_forward(),
            )
            .step_back();
        #[allow(unused_must_use)] {
            g.undo();
        }
        let mut best_score_this_position = score;

        alpha = max(alpha, score);
        if alpha >= beta {
            // Beta cutoff, we have found a better line somewhere else
            self.killer_moves[killer_index] = critical_move;
            self.ttable.store(*g.get_board(), EvalData {
                depth: depth,
                lower_bound: best_score_this_position,
                upper_bound: Eval::MAX,
                critical_move: critical_move
            });
            return alpha;
        }

        for m in moves_iter {
            g.make_move(m);
            // zero-window search
            score = -self
                .pvs(
                    depth - 1,
                    g,
                    mgen,
                    -alpha.step_forward() - Eval(1),
                    -alpha.step_forward(),
                )
                .step_back();
            if alpha < score && score < beta {
                // zero-window search failed high, so there is a better option
                // in this tree. we already have a score from before that we
                // can use as a lower bound in this search.
                score = -self
                    .pvs(
                        depth - 1,
                        g,
                        mgen,
                        -beta.step_forward(),
                        -score.step_forward(),
                    )
                    .step_back();
            }
            #[allow(unused_must_use)] {
                g.undo();
            }
            if score >= best_score_this_position {
                critical_move = m;
                best_score_this_position = score;
                alpha = max(score, alpha);
            }
            if alpha >= beta {
                // Beta cutoff, we have  found a better line somewhere else
                self.killer_moves[killer_index] = m;
                break;
            }
        }

        let upper_bound = match best_score_this_position < beta {
            true => best_score_this_position,
            false => Eval::MAX,
        };
        let lower_bound = match alpha < best_score_this_position {
            true => best_score_this_position,
            false => Eval::MIN,
        };
        self.ttable.store(*g.get_board(), EvalData {
            depth: depth,
            lower_bound: lower_bound,
            upper_bound: upper_bound,
            critical_move: critical_move
        });
        return alpha;
    }

    ///
    /// Use quiescent search (captures only) to evaluate a position as deep as
    /// it needs to go.
    ///
    fn quiesce(
        &mut self,
        depth: i8,
        g: &mut Game,
        mgen: &MoveGenerator,
        alpha_in: Eval,
        beta_in: Eval,
    ) -> Eval {
        self.num_nodes_evaluated += 1;

        let player = g.get_board().player_to_move;
        let mut moves = g.get_moves(mgen);

        // capturing is unforced, so we can stop here if the player to move
        // doesn't want to capture.
        let leaf_evaluation = (self.evaluator)(g, &moves, mgen);
        let mut score = leaf_evaluation * (1 - 2 * player as i32);
        let mut alpha = alpha_in;
        let beta = beta_in;

        alpha = max(score, alpha);
        if alpha >= beta {
            // beta cutoff, this line would not be selected because there is a
            // better option somewhere else
            return alpha;
        }

        let enemy_occupancy = g.get_board().get_color_occupancy(opposite_color(player));
        moves = moves
            .into_iter()
            .filter(|m| enemy_occupancy.contains(m.to_square()))
            .collect::<Vec<Move>>();
        moves.sort_by_cached_key(|m| -(self.candidator)(g, mgen, *m));
        let mut moves_iter = moves.into_iter();

        // we must wrap with an if in case there are no captures
        if let Some(first_move) = moves_iter.next() {
            g.make_move(first_move);
            score = -self
                .quiesce(
                    depth - 1,
                    g,
                    mgen,
                    -beta.step_forward(),
                    -alpha.step_forward(),
                )
                .step_back();
            #[allow(unused_must_use)] {
                g.undo();
            }

            alpha = max(alpha, score.step_back());
            if alpha >= beta {
                // Beta cutoff, we have found a better line somewhere else
                return alpha;
            }
        }

        for m in moves_iter {
            g.make_move(m);
            // zero-window search
            score = -self
                .quiesce(
                    depth - 1,
                    g,
                    mgen,
                    -alpha.step_forward() - Eval(1),
                    -alpha.step_forward(),
                )
                .step_back();
            if alpha < score && score < beta {
                // zero-window search failed high, so there is a better option
                // in this tree. we already have a score from before that we
                // can use as a lower bound in this search.
                score = -self
                    .quiesce(
                        depth - 1,
                        g,
                        mgen,
                        -beta.step_forward(),
                        -score.step_forward(),
                    )
                    .step_back();
            }
            #[allow(unused_must_use)] {
                g.undo();
            }
            alpha = max(alpha, score);
            if alpha >= beta {
                // Beta cutoff, we have  found a better line somewhere else
                break;
            }
        }
        return alpha;
    }

    ///
    /// Clear out internal data.
    ///
    pub fn clear(&mut self) {
        self.num_nodes_evaluated = 0;
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
    fn evaluate(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Eval {
        self.num_nodes_evaluated = 0;
        self.num_transpositions = 0;
        let tic = Instant::now();
        let eval = self.pvs(self.depth, g, mgen, Eval::MIN, Eval::MAX)
            * (1 - 2 * g.get_board().player_to_move as i32);
        let toc = Instant::now();
        let nsecs = (toc - tic).as_secs_f64();
        println!(
            "evaluated {:.0} nodes in {:.0} secs ({:.0} nodes/sec) with {:0} transpositions",
            self.num_nodes_evaluated,
            nsecs,
            self.num_nodes_evaluated as f64 / nsecs,
            self.num_transpositions,
        );
        return eval;
    }

    fn get_evals(&mut self, g: &mut Game, mgen: &MoveGenerator) -> HashMap<Move, Eval> {
        let mut moves = g.get_moves(mgen);
        //negate because sort is ascending
        moves.sort_by_cached_key(|m| -(self.candidator)(g, mgen, *m));
        let mut evals = HashMap::new();
        for m in moves {
            g.make_move(m);
            let ev = self.evaluate(g, mgen);

            //this should never fail since we just made a move, but who knows?
            if let Ok(_) = g.undo() {
                evals.insert(m, ev);
            } else {
                println!("somehow, undoing failed on a game");
            }
            println!("{}: {}", algebraic_from_move(m, g.get_board(), mgen), ev);
        }
        return evals;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::base::fens::*;
    use crate::base::moves::Move;
    use std::collections::HashMap;

    #[test]
    ///
    /// Test PVSearch's evaluation of the start position of the game.
    ///
    pub fn test_eval_start() {
        let mut g = Game::default();
        let mgen = MoveGenerator::new();
        let mut e = PVSearch::default();
        e.set_depth(5); // this prevents taking too long on searches

        println!("moves with evals are:");
        e.get_evals(&mut g, &mgen);
    }

    #[test]
    ///
    /// A test on the evaluation of the game in the fried liver position. The
    /// only winning move for White is Qd3+.
    ///
    fn test_fried_liver() {
        let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
        let mgen = MoveGenerator::new();
        let mut e = PVSearch::default();
        e.set_depth(6); // this prevents taking too long on searches

        e.get_evals(&mut g, &mgen);
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
    /// A test for a puzzle made by Ian.
    ///
    fn test_my_special_puzzle() {
        let mut g = Game::from_fen(MY_PUZZLE_FEN).unwrap();
        let mgen = MoveGenerator::new();
        let mut e = PVSearch::default();
        e.set_depth(5);

        e.get_evals(&mut g, &mgen);
    }

    ///
    /// A helper function which ensures that the evaluation of a position is
    /// equal to what we expect it to be.
    ///
    fn test_eval_helper(fen: &str, eval: Eval, depth: usize) {
        let mut g = Game::from_fen(fen).unwrap();
        let mgen = MoveGenerator::new();
        let mut e = PVSearch::default();
        e.set_depth(depth);

        assert_eq!(e.evaluate(&mut g, &mgen), eval);
    }

    #[allow(unused)]
    ///
    /// Print a map from moves to evals in a user-readable way.
    ///
    fn print_move_map(map: &HashMap<Move, Eval>) {
        for (m, eval) in map {
            println!("{}:{}", m, eval);
        }
    }
}
