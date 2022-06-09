use fiddler_base::{Board, Color, Eval, Piece, Score};

/// Get the value of one piece by its type.
pub const fn value(pt: Piece) -> Score {
    match pt {
        Piece::Knight => Eval::score(291, 286),
        Piece::Bishop => Eval::score(334, 328),
        Piece::Rook => Eval::score(453, 457),
        Piece::Queen => Eval::score(972, 970),
        Piece::Pawn => Eval::score(102, 99),
        Piece::King => Eval::score(0, 0),
    }
}

/// Evaluate a position solely by the amount of material available.
pub fn evaluate(b: &Board) -> Score {
    let mut score = Eval::score(0, 0);

    let white_occupancy = b[Color::White];
    let black_occupancy = b[Color::Black];

    for pt in Piece::ALL_TYPES {
        // Total the quantity of white and black pieces of this type, and
        // multiply their individual value to get the net effect on the eval.
        let pt_squares = b[pt];
        let white_diff = (white_occupancy & pt_squares).count_ones() as i16
            - (black_occupancy & pt_squares).count_ones() as i16;
        let val = value(pt);
        score.0 += val.0 * white_diff;
        score.1 += val.1 * white_diff;
    }

    score
}
