use super::{evaluate::blend_eval, greedy};
use fiddler_base::{Eval, Move, Piece, Position, Score};

use std::cmp::max;

/// Create an estimate for how good a move is. `delta` is the PST difference
/// created by this move.
/// # Panics
/// if the given move is illegal.
pub fn candidacy(pos: &Position, m: Move, delta: Score) -> Eval {
    let b = &pos.board;
    let mover_type = b.type_at_square(m.from_square()).unwrap();
    let promote_type = m.promote_type();
    let capture_type = b.type_at_square(m.to_square());
    let pos_delta = blend_eval(b, delta.0, delta.1);

    // Best case, we keep the piece we captured
    let mut best_case_material = match capture_type {
        Some(p) => greedy::piece_value(p),
        None => Eval::DRAW,
    };
    if promote_type != None {
        best_case_material +=
            greedy::piece_value(promote_type.unwrap()) - greedy::piece_value(Piece::Pawn);
    }
    //Worst case, we lose the piece we moved
    let worst_case_material = best_case_material - greedy::piece_value(mover_type);

    pos_delta + max(worst_case_material, Eval::DRAW)
}
