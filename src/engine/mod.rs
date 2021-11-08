use crate::game::Game;

use crate::engine::eval::Eval;
use crate::r#move::Move;
use crate::movegen::MoveGenerator;

use std::collections::HashMap;

pub mod eval;

/**
 * The public functions that we can trust every Engine to have.
 */
pub trait Engine {
    /**
     * Construct a new engine.
     */
    fn new() -> Self;

    /**
     * Evaluate the position of the given game.
     */
    fn evaluate(g: &Game, mgen: &MoveGenerator) -> Eval;

    /**
     * Get what this engine believes to be the best move in the given position.
     */
    fn get_best_move(g: &Game, mgen: &MoveGenerator) -> Move;

    /**
     * Get the evaluation of each move in this position.
     */
    fn get_evals(g: &Game, mgen: &MoveGenerator) -> HashMap<Move, Eval>;
}