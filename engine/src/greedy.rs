use fiddler_base::{Board, Color, Eval, Piece};

/// Get the value of one piece by its type.
pub fn piece_value(pt: Piece) -> Eval {
    Eval::centipawns(match pt {
        Piece::Knight => 291,
        Piece::Bishop => 333,
        Piece::Rook => 453,
        Piece::Queen => 972,
        Piece::Pawn => 102,
        Piece::King => 0,
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
        let white_diff = (white_occupancy & pt_squares).count_ones() as i16
            - (black_occupancy & pt_squares).count_ones() as i16;
        eval += piece_value(pt) * white_diff;
    }

    eval
}
