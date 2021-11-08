use crate::game::Game;

use crate::engine::eval::Eval;
use crate::movegen::MoveGenerator;
use crate::r#move::{Move, BAD_MOVE};

use std::collections::HashMap;

pub mod eval;

/**
 * An `Engine` is something that can evaluate a `Game`, and give moves which it
 * thinks are good. All the public methods require it to be mutable so that the
 * engine can alter its internal state (such as with transposition tables) to
 * update its internal data.
 */
pub trait Engine {
    /**
     * Construct a new engine.
     */
    fn new() -> Self;

    /**
     * Evaluate the position of the given game. `g` is only given as mutable to
     * allow this method access to the ability to make and undo moves, but `g`
     * should be the same before and after its use.
     */
    fn evaluate(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Eval;

    /**
     * Get what this engine believes to be the best move in the given position.
     * `g` is only given as mutable to allow this method access to the ability
     * to make and undo moves, but `g` should be the same before and after its
     * use.
     */
    fn get_best_move(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Move {
        self.get_evals(g, mgen)
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(k, _)| k)
            .unwrap_or(BAD_MOVE)
    }

    /**
     * Get the evaluation of each move in this position. `g` is only given as
     * mutable to allow this method access to the ability to make and undo
     * moves, but `g` should be the same before and after its use.
     */
    fn get_evals(&mut self, g: &mut Game, mgen: &MoveGenerator) -> HashMap<Move, Eval> {
        let moves = g.get_moves(mgen);
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
        }
        return evals;
    }
}
