use crate::engine::greedy::greedy_evaluate;
use crate::engine::Eval;
use crate::Game;
use crate::MoveGenerator;

/**
 * Evaluate a position by both its material and the positional value of the
 * position.
 */
pub fn positional_evaluate(g: &mut Game, mgen: &MoveGenerator) -> Eval {
    let starting_eval = greedy_evaluate(g, mgen);
    if starting_eval.is_mate() {
        return starting_eval;
    }

    return starting_eval;
}
