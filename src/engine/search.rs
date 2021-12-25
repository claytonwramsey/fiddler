use crate::base::algebraic::algebraic_from_move;
use crate::base::constants::{WHITE};
use crate::base::util::opposite_color;
use crate::base::{Game, Move, MoveGenerator, PieceType, Square};
use crate::engine::positional::positional_evaluate;
use crate::engine::transposition::{TTable};
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
    pub depth: i8,
    ///
    /// The function used to evaluate the quality of a position.
    ///
    pub evaluator: EvaluationFn,
    ///
    /// The function used to determine which moves should be explored first.
    ///
    pub candidator: MoveCandidacyFn,
    #[allow(unused)]
    ///
    /// The transposition table.
    ///
    transpose_table: TTable,
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
    /// This search uses Negamax,  
    ///
    pub fn pvs(&mut self, depth: i8, g: &mut Game, mgen: &MoveGenerator, alpha_in: Eval, beta_in: Eval) -> Eval {
        self.num_nodes_evaluated += 1;
        let us = g.get_board().player_to_move;

        if depth == 0 || g.is_game_over(mgen) {
            // (1 - 2 * us) will cause the evaluation to be positive for 
            // whichever player is moving. This will cascade up the Negamax 
            // inversions to make the final result at the top correct.
            return (self.evaluator)(g, mgen) * (1 - 2 * us as i32);
        }

        let mut moves = g.get_moves(mgen);

        // Sort moves so that the most promising move is evaluated first
        moves.sort_by_cached_key(|m| -(self.candidator)(g, mgen, *m));

        //println!("{}", g);
        let mut moves_iter = moves.into_iter();

        // Lower bound on evaluation. Will be
        let beta = beta_in;
        let mut alpha = alpha_in;

        let first_move = moves_iter.next().unwrap();
        g.make_move(first_move);
        let mut score = -self.pvs(depth - 1, g, mgen, -beta, -alpha).step_back();
        g.undo().unwrap();

        alpha = max(alpha, score);
        if alpha >= beta {
            // Beta cutoff, we have  found a better line somewhere else
            return alpha;
        }

        for m in moves_iter {
            g.make_move(m);
            // zero-window search
            score = -self.pvs(depth - 1, g, mgen, -alpha - Eval(1), -alpha).step_back();
            if alpha < score && score < beta {
                // zero-window search failed high, so there is a better option 
                // in this tree
                score = -self.pvs(depth - 1, g, mgen, -beta, -score);
            }
            g.undo().unwrap();
            alpha = max(alpha, score);
            if alpha >= beta {
                // Beta cutoff, we have  found a better line somewhere else
                break;
            }
        }

        return alpha;
    }

    #[allow(dead_code)]
    ///
    /// Perform a quiescent (captures-only) search of the remaining moves.
    ///
    fn quiesce(
        &mut self,
        g: &mut Game,
        mgen: &MoveGenerator,
        alpha_in: Eval,
        beta_in: Eval,
    ) -> Eval {
        self.num_nodes_evaluated += 1;

        let player = g.get_board().player_to_move;
        let enemy_occupancy = g.get_board().get_color_occupancy(opposite_color(player));
        let king_square = Square::from(g.get_board().get_type_and_color(PieceType::KING, player));
        let currently_in_check =
            mgen.is_square_attacked_by(g.get_board(), king_square, opposite_color(player));
        let mut moves: Vec<Move> = g.get_moves(mgen);

        if !currently_in_check {
            moves = moves
                .into_iter()
                .filter(|m| enemy_occupancy.contains(m.to_square()))
                .collect();
        }

        if moves.len() == 0 {
            return (self.evaluator)(g, mgen);
        }

        moves.sort_by_cached_key(|m| -(self.candidator)(g, mgen, *m));

        let mut alpha = alpha_in;
        let mut beta = beta_in;

        let mut evaluation = match player {
            WHITE => Eval::MIN,
            _ => Eval::MAX,
        };

        for mov in moves {
            g.make_move(mov);
            let eval_for_mov = self.quiesce(g, mgen, alpha, beta);

            g.undo().ok();

            //alpha-beta pruning
            if player == WHITE {
                evaluation = max(evaluation, eval_for_mov);
                if evaluation >= beta {
                    break;
                }
                alpha = max(alpha, evaluation);
            } else {
                //black moves on this turn
                evaluation = min(evaluation, eval_for_mov);
                if evaluation <= alpha {
                    break;
                }
                beta = min(beta, evaluation);
            }
        }

        return evaluation;
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
        PVSearch {
            depth: 5,
            evaluator: positional_evaluate,
            candidator: crate::engine::candidacy::candidacy,
            transpose_table: TTable::default(),
            num_nodes_evaluated: 0,
            num_transpositions: 0,
        }
    }
}

impl Engine for PVSearch {
    #[inline]
    fn evaluate(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Eval {
        self.num_nodes_evaluated = 0;
        self.num_transpositions = 0;
        let tic = Instant::now();
        let eval = self.pvs(self.depth, g, mgen, Eval::MIN, Eval::MAX);
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
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
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

        println!("moves with evals are:");
        e.get_evals(&mut g, &mgen);
    }

    #[test]
    fn test_fried_liver() {
        let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
        let mgen = MoveGenerator::new();
        let mut e = PVSearch::default();

        e.get_evals(&mut g, &mgen);
    }

    #[test]
    fn test_mate_in_1() {
        test_eval_helper(MATE_IN_1_FEN, Eval::mate_in(1));
    }

    #[test]
    fn test_mate_in_4_ply() {
        test_eval_helper(MATE_IN_4_FEN, Eval::mate_in(4));
    }

    #[test]
    fn test_my_special_puzzle() {
        let mut g = Game::from_fen(MY_PUZZLE_FEN).unwrap();
        let mgen = MoveGenerator::new();
        let mut e = PVSearch::default();

        e.get_evals(&mut g, &mgen);
    }

    fn test_eval_helper(fen: &str, eval: Eval) {
        let mut g = Game::from_fen(fen).unwrap();
        let mgen = MoveGenerator::new();
        let mut e = PVSearch::default();

        assert_eq!(e.evaluate(&mut g, &mgen), eval);
    }

    #[allow(dead_code)]
    ///
    /// Print a map from moves to evals in a user-readable way.
    ///
    fn print_move_map(map: &HashMap<Move, Eval>) {
        for (m, eval) in map {
            println!("{}:{}", m, eval);
        }
    }
}
