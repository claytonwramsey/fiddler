use crate::engine::{Engine, Eval, EvaluationFn, MIN_EVAL, MAX_EVAL};
use crate::engine::greedy::greedy_evaluate;
use crate::constants::{WHITE, BLACK};
use crate::game::Game;
use crate::movegen::MoveGenerator;

use std::cmp::{max, min};

/**
 * A stupid-simple engine which will evaluate the entire tree.
 */
pub struct Minimax {
    depth: i8,
    evaluator: EvaluationFn,
}

impl Minimax {
    /**
     * Evaluate a position at a given depth. The depth is the number of plays to make. Even depths are recommended for fair evaluations.
     */
    pub fn evaluate_at_depth(&self, depth: i8, g: &mut Game, mgen: &MoveGenerator) -> Eval {
        if depth <= 0  || g.is_game_over(mgen) {
            return (self.evaluator)(g, mgen);
        }

        let player_to_move = g.get_board().player_to_move;

        let mut evaluation = match player_to_move {
            WHITE => MIN_EVAL,
            BLACK => MAX_EVAL,
            _ => Eval(0),
        };

        for m in mgen.get_moves(g.get_board()) {
            g.make_move(m);
            let eval_for_m = self.evaluate_at_depth(depth - 1, g, mgen);
            evaluation = match player_to_move {
                WHITE => max(eval_for_m, evaluation),
                BLACK => min(eval_for_m, evaluation),
                _ => evaluation,
            };
        }

        return evaluation;
    }
}

impl Default for Minimax {
    fn default() -> Minimax {
        Minimax {
            depth: 13,
            evaluator: greedy_evaluate,
        }
    }
}

impl Engine for Minimax {
    #[inline]
    fn evaluate(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Eval {
        self.evaluate_at_depth(self.depth, g, mgen)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    /**
     * Test Minimax's evaluation of the start position of the game.
     */
    fn test_eval_start() {
        let mut g = Game::default();
        let mgen = MoveGenerator::new();
        let mut e = Minimax::default();

        println!("moves with evals are:");
        for (m, eval) in e.get_evals(&mut g, &mgen) {
            println!("{}:{}", m, eval);
        }
    }
}