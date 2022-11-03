//! Evaluation of positions based on the mobility of pieces.
//!
//! We count mobility by examining the number of squares that a piece can move
//! to according to pseudo-legal moves.
//! This means that captures and empty squares visible to a piece (independent
//! of whether it is pinned) count towards its mobility score.
//!
//! For each piece, for each number of squares attacked, a unique mobility bonus
//! is given.
//! This prevents pieces from being placed uselessly in the name of being able
//! to see more squares.

use std::mem::transmute;

use fiddler_base::{
    movegen::{KING_MOVES, KNIGHT_MOVES, PAWN_ATTACKS},
    Bitboard, Board, Color, Piece, MAGIC,
};

use super::Score;

/// The maximum number of squares that any piece can attack, plus 1.
pub const MAX_MOBILITY: usize = 28;

/// The value of having a piece have a certain number of squares attacked.
pub const ATTACKS_VALUE: [[Score; MAX_MOBILITY]; Piece::NUM] = unsafe {
    transmute([
        [
            // N
            (-4i16, 0i16),
            (-14, 0),
            (-8, -10),
            (-5, -16),
            (7, -21),
            (11, -7),
            (12, -11),
            (16, 2),
            (17, -4),
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
            (-31, 0),
            (-15, -7),
            (-6, -17),
            (-2, -20),
            (3, -16),
            (6, -11),
            (11, -11),
            (12, -10),
            (16, -2),
            (19, -9),
            (19, 0),
            (20, -4),
            (4, 0),
            (5, 0),
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
            (-29, 0),
            (-20, 0),
            (-19, 0),
            (-16, 0),
            (-13, -2),
            (-10, -4),
            (-7, -2),
            (-3, 2),
            (-1, 1),
            (3, -1),
            (8, -4),
            (11, -2),
            (17, 1),
            (20, 2),
            (18, 4),
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
            (-2, 0),
            (-4, 0),
            (-6, 0),
            (-7, 0),
            (-7, 0),
            (-3, 0),
            (-2, 0),
            (-5, 0),
            (-4, 0),
            (-2, 0),
            (-2, 0),
            (-1, 0),
            (-3, 3),
            (-2, 0),
            (-3, -2),
            (-3, 0),
            (-2, -1),
            (0, -1),
            (0, -1),
            (0, 0),
            (0, -1),
            (0, 0),
            (3, -1),
            (2, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ],
        [
            // P
            (-8, 3),
            (-10, 5),
            (-14, 2),
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
            (13, 0),
            (15, -8),
            (7, -26),
            (1, -11),
            (-6, 0),
            (-13, 3),
            (-3, 9),
            (-5, 17),
            (-4, 15),
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
    ])
};

#[inline(always)]
#[must_use]
/// Helper function for computing mobility scores of a piece.
///
/// Inputs:
/// * `pt`: the type of the piece being scored.
/// * `attacks`: the squares that the piece is attacking.
///
/// Returns the score associated with `pt` attacking all the squares in
/// `attacks`.
pub const fn for_piece(pt: Piece, attacks: Bitboard) -> Score {
    ATTACKS_VALUE[pt as usize][attacks.len() as usize]
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
    // pinned knights can't move and so we don't bother counting them
    for sq in knights & white {
        score += for_piece(Piece::Knight, KNIGHT_MOVES[sq as usize] & not_white);
    }
    for sq in knights & black {
        score -= for_piece(Piece::Knight, KNIGHT_MOVES[sq as usize] & not_black);
    }

    // count bishop moves
    let bishops = b[Piece::Bishop];
    for sq in bishops & white {
        score += for_piece(
            Piece::Bishop,
            MAGIC.bishop_attacks(occupancy, sq) & not_white,
        );
    }
    for sq in bishops & black {
        score -= for_piece(
            Piece::Bishop,
            MAGIC.bishop_attacks(occupancy, sq) & not_black,
        );
    }

    // count rook moves
    let rooks = b[Piece::Rook];
    for sq in rooks & white {
        score += for_piece(Piece::Rook, MAGIC.rook_attacks(occupancy, sq) & not_white);
    }
    for sq in rooks & black {
        score -= for_piece(Piece::Rook, MAGIC.rook_attacks(occupancy, sq) & not_black);
    }

    // count queen moves
    let queens = b[Piece::Queen];
    for sq in queens & white {
        let attacks = MAGIC.rook_attacks(occupancy, sq) | MAGIC.bishop_attacks(occupancy, sq);
        score += for_piece(Piece::Queen, attacks & not_white);
    }
    for sq in rooks & black {
        let attacks = MAGIC.rook_attacks(occupancy, sq) | MAGIC.bishop_attacks(occupancy, sq);
        score -= for_piece(Piece::Queen, attacks & not_black);
    }

    // count net pawn moves
    // pawns can't capture by pushing, so we only examine their capture squares
    let pawns = b[Piece::Pawn];
    for sq in pawns & white {
        score += for_piece(
            Piece::Pawn,
            PAWN_ATTACKS[Color::White as usize][sq as usize] & not_white,
        );
    }
    for sq in pawns & black {
        score -= for_piece(
            Piece::Pawn,
            PAWN_ATTACKS[Color::Black as usize][sq as usize] & not_black,
        );
    }

    score += for_piece(
        Piece::King,
        KING_MOVES[b.king_sqs[Color::White as usize] as usize] & not_white,
    );
    score -= for_piece(
        Piece::King,
        KING_MOVES[b.king_sqs[Color::Black as usize] as usize] & not_black,
    );

    score
}
