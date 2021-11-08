use crate::game::Game;

use crate::engine::eval::Eval;
use crate::r#move::{Move, BAD_MOVE};
use crate::movegen::MoveGenerator;

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
     * Evaluate the position of the given game.
     */
    fn evaluate(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Eval;

    /**
     * Get what this engine believes to be the best move in the given position.
     */
    fn get_best_move(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Move {
        self.get_evals(g, mgen).unwrap_or(HashMap::new())
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(k, _)| k)
            .unwrap_or(BAD_MOVE)
    }

    /**
     * Get the evaluation of each move in this position. Although g is given as mutable, 
     */
    fn get_evals(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Result<HashMap<Move, Eval>, &'static str> {
        let moves = g.get_moves(mgen);
        let mut evals = HashMap::new();
        for m in moves {
            g.make_move(m);
            let ev = self.evaluate(g, mgen);
            evals.insert(m, ev);

            //this should never fail since we just made a move, but who knows?
            g.undo()?;
        }
        Ok(evals)
    }
}