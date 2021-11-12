use crate::engine::{greedy, positional, Eval};
use crate::Game;
use crate::Move;
use crate::MoveGenerator;
use std::cmp::max;

pub fn candidacy(g: &mut Game, _mgen: &MoveGenerator, m: Move) -> Eval {
    let b = g.get_board();
    let mover_type = b.type_at_square(m.from_square());
    let capture_type = b.type_at_square(m.to_square());

    let positional_loss = positional::value_at_square(mover_type, m.from_square());
    let positional_gain = positional::value_at_square(mover_type, m.to_square());
    let positional_capture =
        positional::value_at_square(b.type_at_square(m.to_square()), m.to_square());

    let positional_delta = positional_gain + positional_capture - positional_loss;

    // Best case, we keep the piece we captured
    let best_case_material = greedy::piece_value(capture_type);
    //Worst case, we lose the piece we moved
    let worst_case_material = best_case_material - greedy::piece_value(mover_type);

    return positional_delta + max(worst_case_material, Eval(0));
}
