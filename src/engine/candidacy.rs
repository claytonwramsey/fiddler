use crate::base::Game;
use crate::base::Move;
use crate::base::MoveGenerator;
use crate::base::Piece;
use crate::engine::{greedy, positional, Eval};
use std::cmp::max;

///
/// Create an estimate for how good a move is.
/// # Panics
/// if the given move is illegal.
///
pub fn candidacy(g: &mut Game, _mgen: &MoveGenerator, m: Move) -> Eval {
    let b = g.get_board();
    let mover_type = b.type_at_square(m.from_square()).unwrap();
    let promote_type = m.promote_type();
    let capture_type = b.type_at_square(m.to_square());

    let positional_loss = positional::value_at_square(mover_type, m.from_square());
    let positional_gain = match promote_type {
        None => positional::value_at_square(mover_type, m.to_square()),
        Some(p) => positional::value_at_square(p, m.to_square()),
    };
    let positional_capture = match capture_type {
        Some(p) => positional::value_at_square(p, m.to_square()),
        None => Eval(0),
    };

    let positional_delta = positional_gain + positional_capture - positional_loss;

    // Best case, we keep the piece we captured
    let mut best_case_material = match capture_type {
        Some(p) => greedy::piece_value(p),
        None => Eval(0),
    };
    if promote_type != None {
        best_case_material +=
            greedy::piece_value(promote_type.unwrap()) - greedy::piece_value(Piece::Pawn);
    }
    //Worst case, we lose the piece we moved
    let worst_case_material = best_case_material - greedy::piece_value(mover_type);

    positional_delta + max(worst_case_material, Eval(0))
}
