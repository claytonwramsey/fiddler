use crate::base::Board;
use crate::base::Color;
use crate::base::Eval;
use crate::base::Piece;

/// Get the value of one piece by its type.
pub fn piece_value(pt: Piece) -> Eval {
    Eval::pawns(match pt {
        Piece::Pawn => 1.0,
        Piece::Knight => 2.9,
        Piece::Bishop => 3.1,
        Piece::Rook => 5.0,
        Piece::Queen => 9.0,
        _ => 0.0,
    })
}

/// Evaluate a position solely by the amount of material available.
pub fn greedy_evaluate(b: &Board) -> Eval {
    let mut eval = Eval::DRAW;

    let white_occupancy = b[Color::White];
    let black_occupancy = b[Color::Black];

    for pt in Piece::ALL_TYPES {
        // Total the quantity of white and black pieces of this type, and
        // multiply their individual value to get the net effect on the eval.
        let pt_squares = b[pt];
        let white_diff = (white_occupancy & pt_squares).0.count_ones() as i32
            - (black_occupancy & pt_squares).0.count_ones() as i32;
        eval += piece_value(pt) * white_diff;
    }

    eval
}
