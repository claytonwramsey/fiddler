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
            (-12i16, -4i16),
            (-1, -39),
            (5, -40),
            (9, -33),
            (24, -34),
            (30, -16),
            (31, -17),
            (36, -5),
            (39, -10),
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
            (-18, -50),
            (0, -68),
            (8, -54),
            (14, -42),
            (20, -31),
            (23, -25),
            (28, -23),
            (30, -21),
            (34, -13),
            (38, -18),
            (39, -11),
            (42, -15),
            (40, -8),
            (45, -10),
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
            (-35, -12),
            (-25, -20),
            (-22, -14),
            (-19, -2),
            (-15, -4),
            (-13, 0),
            (-9, 2),
            (-6, 6),
            (-3, 8),
            (1, 6),
            (6, 6),
            (10, 7),
            (16, 11),
            (20, 13),
            (15, 13),
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
            (-4, -13),
            (-7, -5),
            (-8, -10),
            (-9, -8),
            (-4, -18),
            (-2, -11),
            (-5, -15),
            (-4, -14),
            (-1, -11),
            (-1, -4),
            (0, -3),
            (-1, -2),
            (0, -5),
            (0, -8),
            (0, -5),
            (2, -6),
            (3, -6),
            (4, -6),
            (6, -5),
            (4, -6),
            (6, -7),
            (16, -7),
            (20, -5),
            (12, -6),
            (24, -5),
            (2, -4),
            (9, -10),
        ],
        [
            // P
            (-5, 1),
            (-7, 3),
            (-11, 0),
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
            (22, -6),
            (20, -62),
            (11, -30),
            (4, -5),
            (-4, 5),
            (-15, 11),
            (-7, 24),
            (-12, 32),
            (-15, 29),
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
