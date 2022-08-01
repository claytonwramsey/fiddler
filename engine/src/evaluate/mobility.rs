//! Evaluation of positions based on the mobility of pieces.

use fiddler_base::{
    movegen::{KING_MOVES, KNIGHT_MOVES, PAWN_ATTACKS},
    Board, Color, Piece, MAGIC,
};

use super::{Eval, Score};

/// The maximum number of squares that any piece can attack, plus 1.
pub const MAX_MOBILITY: usize = 28;

/// The value of having a piece have a certain number of squares attacked.
pub const ATTACKS_VALUE: [[Score; MAX_MOBILITY]; Piece::NUM] = expand_attacks(&[
    [
        // N
        (-1, 0),
        (-12, 0),
        (-15, -2),
        (-12, -5),
        (3, -8),
        (12, -1),
        (12, -3),
        (18, 6),
        (17, 1),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // B
        (-38, 0),
        (-19, -1),
        (-9, -3),
        (-5, -5),
        (1, -4),
        (9, -2),
        (12, -2),
        (13, -2),
        (19, 2),
        (19, 0),
        (11, 2),
        (8, 0),
        (1, 0),
        (1, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // R
        (-19, 0),
        (-12, 0),
        (-11, 0),
        (-10, 0),
        (-6, 0),
        (-6, 0),
        (-4, 0),
        (-1, 4),
        (0, 5),
        (5, 2),
        (8, -2),
        (9, 0),
        (15, 3),
        (11, 2),
        (11, 8),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // Q
        (-8, 0),
        (-7, 0),
        (-5, 0),
        (-6, 0),
        (-6, 0),
        (0, 0),
        (0, 0),
        (-4, 0),
        (-4, 0),
        (-1, 0),
        (0, 0),
        (1, 1),
        (0, 2),
        (1, 1),
        (0, 0),
        (0, 2),
        (3, 2),
        (3, 2),
        (4, 1),
        (4, 2),
        (2, 0),
        (2, 0),
        (2, 0),
        (1, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // P
        (-12, 6),
        (-12, 18),
        (-14, 16),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // K
        (0, 0),
        (12, -1),
        (3, -7),
        (-1, -11),
        (-10, -5),
        (-13, -6),
        (2, 4),
        (3, 16),
        (5, 11),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
]);

/// Helper function to make the process of writing down attack values more easy.
const fn expand_attacks(
    vals: &[[(i16, i16); MAX_MOBILITY]; Piece::NUM],
) -> [[Score; MAX_MOBILITY]; Piece::NUM] {
    let mut out = [[Score::DRAW; MAX_MOBILITY]; 6];

    let mut pt_idx = 0;
    while pt_idx < Piece::NUM {
        let mut mobility_idx = 0;
        while mobility_idx < MAX_MOBILITY {
            let (mg, eg) = vals[pt_idx][mobility_idx];
            out[pt_idx][mobility_idx] = Score::new(Eval(mg), Eval(eg));
            mobility_idx += 1;
        }
        pt_idx += 1;
    }

    out
}

#[must_use]
/// Get the mobility evaluation of a board.
pub fn evaluate(b: &Board) -> Score {
    let white = b[Color::White];
    let black = b[Color::Black];
    let not_white = !white;
    let not_black = !black;
    let occupancy = white | black;
    let mut score = Score::DRAW;

    // count knight moves
    let knights = b[Piece::Knight];
    for sq in knights & white {
        score += ATTACKS_VALUE[Piece::Knight as usize]
            [usize::from((KNIGHT_MOVES[sq as usize] & not_white).len())];
    }
    for sq in knights & black {
        score -= ATTACKS_VALUE[Piece::Knight as usize]
            [usize::from((KNIGHT_MOVES[sq as usize] & not_black).len())];
    }

    // count bishop moves
    let bishops = b[Piece::Bishop];
    for sq in bishops & white {
        score += ATTACKS_VALUE[Piece::Bishop as usize]
            [usize::from((MAGIC.bishop_attacks(occupancy, sq) & not_white).len())];
    }
    for sq in bishops & black {
        score -= ATTACKS_VALUE[Piece::Bishop as usize]
            [usize::from((MAGIC.bishop_attacks(occupancy, sq) & not_black).len())];
    }

    // count rook moves
    let rooks = b[Piece::Rook];
    for sq in rooks & white {
        score += ATTACKS_VALUE[Piece::Rook as usize]
            [usize::from((MAGIC.rook_attacks(occupancy, sq) & not_white).len())];
    }
    for sq in rooks & black {
        score -= ATTACKS_VALUE[Piece::Rook as usize]
            [usize::from((MAGIC.rook_attacks(occupancy, sq) & not_black).len())];
    }

    // count queen moves
    let queens = b[Piece::Queen];
    for sq in queens & white {
        let attacks = MAGIC.rook_attacks(occupancy, sq) | MAGIC.bishop_attacks(occupancy, sq);
        score += ATTACKS_VALUE[Piece::Queen as usize][usize::from((attacks & not_white).len())];
    }
    for sq in rooks & black {
        let attacks = MAGIC.rook_attacks(occupancy, sq) | MAGIC.bishop_attacks(occupancy, sq);
        score -= ATTACKS_VALUE[Piece::Queen as usize][usize::from((attacks & not_black).len())];
    }

    // count net pawn moves
    // pawns can't capture by pushing, so we only examine their capture squares
    let pawns = b[Piece::Pawn];
    for sq in pawns & white {
        score += ATTACKS_VALUE[Piece::Pawn as usize]
            [usize::from((PAWN_ATTACKS[Color::White as usize][sq as usize] & not_white).len())];
    }
    for sq in pawns & black {
        score -= ATTACKS_VALUE[Piece::Pawn as usize]
            [usize::from((PAWN_ATTACKS[Color::White as usize][sq as usize] & not_black).len())];
    }

    score += ATTACKS_VALUE[Piece::King as usize]
        [usize::from((KING_MOVES[b.king_sqs[Color::White as usize] as usize] & not_white).len())];
    score -= ATTACKS_VALUE[Piece::King as usize]
        [usize::from((KING_MOVES[b.king_sqs[Color::Black as usize] as usize] & not_black).len())];

    score
}
