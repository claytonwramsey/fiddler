use crate::constants::{BLACK, WHITE};
use crate::engine::positional::positional_evaluate;
use crate::engine::{Eval, EvaluationFn};
use crate::Engine;
use crate::Game;
use crate::Board;
use crate::MoveGenerator;

use std::cmp::{max, min};
use std::time::Instant;
use std::collections::HashMap;

/**
 * A stupid-simple engine which will evaluate the entire tree.
 */
pub struct Minimax {
    /**
     * The depth at which this algorithm will evaluate a position.
     */
    pub depth: i8,
    /**
     * The function used to evaluate the quality of a position.
     */
    pub evaluator: EvaluationFn,
    /**
     * The cumulative number of nodes evaluated in this evaluation event.
     */
    num_nodes_evaluated: u64,
    /**
     * The transposition table.
     */
    transpose_table: HashMap<Board, TTableEntry>,
}

/**
 * An entry in the transposition table.
 */
struct TTableEntry {
    /**
     * The depth at which this position was evaluated.
     */
    pub depth: i8,
    /**
     * The evaluation we found at this position.
     */
    pub eval: Eval,
}

impl Minimax {
    /**
     * Evaluate a position at a given depth. The depth is the number of plays to make. Even depths are recommended for fair evaluations.
     */
    pub fn evaluate_at_depth(
        &mut self,
        depth: i8,
        alpha_in: Eval,
        beta_in: Eval,
        g: &mut Game,
        mgen: &MoveGenerator,
    ) -> Eval {
        self.num_nodes_evaluated += 1;
        let b = g.get_board();

        if let Some(v) = self.transpose_table.get(b) {
            if v.depth >= depth {
                return v.eval;
            }
        }

        if depth <= 0 || g.is_game_over(mgen) {
            let eval = (self.evaluator)(g, mgen);
            self.transpose_table.insert(
                *g.get_board(),
                TTableEntry {
                    depth: depth,
                    eval: eval,
                },
            );
            return eval;
        }

        let mut alpha = alpha_in;
        let mut beta = beta_in;

        let player_to_move = b.player_to_move;

        let mut evaluation = match player_to_move {
            WHITE => Eval::MIN,
            BLACK => Eval::MAX,
            _ => Eval(0),
        };

        for m in mgen.get_moves(b) {
            g.make_move(m);
            let eval_for_m = self.evaluate_at_depth(depth - 1, alpha, beta, g, mgen);

            if let Err(_) = g.undo() {
                println!("undo failed despite having move history!");
            }

            //alpha-beta pruning
            if player_to_move == WHITE {
                evaluation = max(evaluation, eval_for_m);
                if evaluation >= beta {
                    break;
                }
                alpha = max(alpha, evaluation);
            } else {
                //black moves on this turn
                evaluation = min(evaluation, eval_for_m);
                if evaluation <= alpha {
                    break;
                }
                beta = min(beta, evaluation);
            }
        }

        evaluation = evaluation.step_back();
        self.transpose_table.insert(
            *g.get_board(),
            TTableEntry {
                depth: depth,
                eval: evaluation,
            },
        );
        return evaluation;
    }

    /**
     * Clear out internal data.
     */
    pub fn clear(&mut self) {
        self.num_nodes_evaluated = 0;
        self.transpose_table.clear();
    }
}

impl Default for Minimax {
    fn default() -> Minimax {
        Minimax {
            depth: 5,
            evaluator: positional_evaluate,
            num_nodes_evaluated: 0,
            transpose_table: HashMap::new(),
        }
    }
}

impl Engine for Minimax {
    #[inline]
    fn evaluate(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Eval {
        self.num_nodes_evaluated = 0;
        let tic = Instant::now();
        let eval = self.evaluate_at_depth(self.depth, Eval::MIN, Eval::MAX, g, mgen);
        let toc = Instant::now();
        let nsecs = (toc - tic).as_secs_f64();
        println!(
            "evaluated {:.0} nodes in {:.0} secs ({:.0} nodes/sec)",
            self.num_nodes_evaluated,
            nsecs,
            self.num_nodes_evaluated as f64 / nsecs
        );
        return eval;
    }
}

#[cfg(test)]
pub mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::fens::*;
    use crate::moves::Move;
    use std::collections::HashMap;

    #[test]
    /**
     * Test Minimax's evaluation of the start position of the game.
     */
    pub fn test_eval_start() {
        let mut g = Game::default();
        let mgen = MoveGenerator::new();
        let mut e = Minimax::default();

        println!("moves with evals are:");
        print_move_map(&e.get_evals(&mut g, &mgen));
    }

    #[test]
    fn test_fried_liver() {
        let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
        let mgen = MoveGenerator::new();
        let mut e = Minimax::default();

        println!("moves with evals are:");
        print_move_map(&e.get_evals(&mut g, &mgen));
    }

    #[test]
    fn test_mate_in_1() {
        test_eval_helper(MATE_IN_1_FEN, Eval::mate_in(1));
    }

    #[test]
    fn test_mate_in_4_ply() {
        test_eval_helper(MATE_IN_4_FEN, Eval::mate_in(4));
    }

    fn test_eval_helper(fen: &str, eval: Eval) {
        let mut g = Game::from_fen(fen).unwrap();
        let mgen = MoveGenerator::new();
        let mut e = Minimax::default();

        assert_eq!(e.evaluate(&mut g, &mgen), eval);
    }
    /**
     * Print a map from moves to evals in a user-readable way.
     */
    fn print_move_map(map: &HashMap<Move, Eval>) {
        for (m, eval) in map {
            println!("{}:{}", m, eval);
        }
    }
}
