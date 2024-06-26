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
    game::Game,
    movegen::{bishop_attacks, rook_attacks, KING_ATTACKS, KNIGHT_ATTACKS, PAWN_ATTACKS},
    Bitboard, Color, Piece,
};

use super::Score;

/// The maximum number of squares that any piece can attack, plus 1.
pub const MAX_MOBILITY: usize = 28;

/// The value of having a piece have a certain number of squares attacked.
pub const ATTACKS_VALUE: [[Score; MAX_MOBILITY]; Piece::NUM] = unsafe {
    transmute([
        [
            // N
            [3i16, -13i16],
            [-3, -36],
            [-3, -37],
            [2, -31],
            [9, -27],
            [24, -10],
            [27, -7],
            [38, 5],
            [35, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
        ],
        [
            // B
            [-14, -55],
            [-12, -57],
            [-2, -47],
            [5, -38],
            [13, -30],
            [20, -22],
            [24, -19],
            [25, -17],
            [33, -8],
            [32, -12],
            [35, -8],
            [35, -9],
            [39, -7],
            [44, -9],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
        ],
        [
            // R
            [-28, -20],
            [-26, -18],
            [-20, -11],
            [-13, -3],
            [-10, -1],
            [-8, 1],
            [-6, 2],
            [0, 9],
            [1, 10],
            [1, 9],
            [2, 7],
            [4, 8],
            [9, 11],
            [10, 13],
            [10, 13],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
        ],
        [
            // Q
            [0, -2],
            [-7, -10],
            [-4, -5],
            [-7, -7],
            [-6, -7],
            [-9, -10],
            [-4, -5],
            [-7, -8],
            [-7, -8],
            [-4, -5],
            [0, -1],
            [0, -1],
            [0, 0],
            [0, 0],
            [0, -1],
            [0, 0],
            [0, 0],
            [1, 0],
            [1, 0],
            [2, 0],
            [2, -1],
            [2, -2],
            [9, -3],
            [15, -2],
            [8, -3],
            [22, -2],
            [1, -2],
            [9, -10],
        ],
        [
            // P
            [-3, 0],
            [-1, 3],
            [-3, 1],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
        ],
        [
            // K
            [9, 1],
            [-18, -31],
            [-13, -23],
            [-6, -12],
            [-2, -3],
            [-2, 2],
            [9, 16],
            [13, 24],
            [14, 21],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
        ],
    ])
};

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
///
/// # Panics
///
/// This function may panic if `g` is in an invalid state.
pub fn evaluate(g: &Game) -> Score {
    let white = g.white();
    let black = g.black();
    let not_white = !white;
    let not_black = !black;
    let occupancy = white | black;
    let mut score = Score::DRAW;

    // count knight moves
    let knights = g.knights();
    // pinned knights can't move and so we don't bother counting them
    for sq in knights & white {
        score += for_piece(Piece::Knight, KNIGHT_ATTACKS[sq as usize] & not_white);
    }
    for sq in knights & black {
        score -= for_piece(Piece::Knight, KNIGHT_ATTACKS[sq as usize] & not_black);
    }

    // count bishop moves
    let bishops = g.bishops();
    for sq in bishops & white {
        score += for_piece(Piece::Bishop, bishop_attacks(occupancy, sq) & not_white);
    }
    for sq in bishops & black {
        score -= for_piece(Piece::Bishop, bishop_attacks(occupancy, sq) & not_black);
    }

    // count rook moves
    let rooks = g.rooks();
    for sq in rooks & white {
        score += for_piece(Piece::Rook, rook_attacks(occupancy, sq) & not_white);
    }
    for sq in rooks & black {
        score -= for_piece(Piece::Rook, rook_attacks(occupancy, sq) & not_black);
    }

    // count queen moves
    let queens = g.queens();
    for sq in queens & white {
        let attacks = rook_attacks(occupancy, sq) | bishop_attacks(occupancy, sq);
        score += for_piece(Piece::Queen, attacks & not_white);
    }
    for sq in rooks & black {
        let attacks = rook_attacks(occupancy, sq) | bishop_attacks(occupancy, sq);
        score -= for_piece(Piece::Queen, attacks & not_black);
    }

    // count net pawn moves
    // pawns can't capture by pushing, so we only examine their capture squares
    let pawns = g.pawns();
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
        KING_ATTACKS[g.king_sq(Color::White) as usize] & not_white,
    );
    score -= for_piece(
        Piece::King,
        KING_ATTACKS[g.king_sq(Color::Black) as usize] & not_black,
    );

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_kings() {
        let g = Game::from_fen("8/8/5k2/8/8/8/2K5/8 w - - 0 1").unwrap();
        assert_eq!(evaluate(&g), Score::DRAW);
    }

    #[test]
    fn incomplete_mobility() {
        let g = Game::from_fen("8/8/5k2/8/8/8/8/K7 w - - 0 1").unwrap();
        assert_eq!(
            evaluate(&g),
            ATTACKS_VALUE[Piece::King as usize][3] - ATTACKS_VALUE[Piece::King as usize][8]
        );
    }
}
