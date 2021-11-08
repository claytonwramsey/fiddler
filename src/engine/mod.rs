use crate::game::Game;

use crate::engine::eval::Eval;
use crate::r#move::Move;
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
    fn evaluate(&mut self, g: &Game, mgen: &MoveGenerator) -> Eval;

    /**
     * Get what this engine believes to be the best move in the given position.
     */
    fn get_best_move(&mut self, g: &Game, mgen: &MoveGenerator) -> Move;

    /**
     * Get the evaluation of each move in this position.
     */
    fn get_evals(&mut self, g: &Game, mgen: &MoveGenerator) -> HashMap<Move, Eval>;
}