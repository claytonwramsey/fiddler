//! Evaluation of positions based on the mobility of pieces.
//!
//! We count mobility by examining the number of squares that a piece can move to according to
//! pseudo-legal moves.
//! This means that captures and empty squares visible to a piece (independent of whether it is
//! pinned) count towards its mobility score.
//!
//! For each piece, for each number of squares attacked, a unique mobility bonus is given.
//! This prevents pieces from being placed uselessly in the name of being able to see more squares.

use std::mem::transmute;

use crate::base::{
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
            (-1i16, -10i16),
            (-2, -37),
            (-1, -37),
            (4, -31),
            (11, -27),
            (25, -11),
            (28, -8),
            (38, 3),
            (36, 0),
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
            (-14, -55),
            (-11, -57),
            (-1, -47),
            (6, -38),
            (14, -30),
            (20, -23),
            (24, -20),
            (25, -18),
            (33, -10),
            (32, -13),
            (36, -9),
            (35, -10),
            (39, -7),
            (44, -9),
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
            (-28, -19),
            (-26, -18),
            (-21, -12),
            (-13, -3),
            (-11, -2),
            (-8, 0),
            (-6, 1),
            (-1, 8),
            (0, 10),
            (1, 8),
            (3, 7),
            (4, 8),
            (10, 11),
            (11, 13),
            (10, 14),
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
            (0, -2),
            (-7, -10),
            (-4, -6),
            (-7, -8),
            (-7, -7),
            (-9, -11),
            (-4, -6),
            (-7, -9),
            (-7, -9),
            (-4, -6),
            (-1, -1),
            (0, -2),
            (0, 0),
            (0, 0),
            (-1, -2),
            (0, -1),
            (0, -1),
            (1, -1),
            (1, -1),
            (2, 0),
            (2, -2),
            (3, -3),
            (11, -4),
            (16, -3),
            (9, -4),
            (23, -3),
            (1, -2),
            (9, -10),
        ],
        [
            // P
            (-4, 0),
            (-2, 2),
            (-4, 1),
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
            (10, 1),
            (-16, -32),
            (-11, -22),
            (-4, -11),
            (-1, -3),
            (-3, 2),
            (8, 16),
            (11, 24),
            (10, 20),
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
/// Returns the score associated with `pt` attacking all the squares in `attacks`.
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
