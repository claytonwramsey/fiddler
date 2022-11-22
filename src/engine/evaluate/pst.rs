/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Piece-Square Tables (PSTs).
//!
//! A PST is a table with an item for each piece at each square.
//! It grants a fixed value to the evaluation of a position for each piece,
//! granting benefits for being on "good" squares and penalties for pieces on
//! "bad" ones.
//! For instance, a knight is much more valuable near the center, so the PST
//! value for a knight on rank 4 and file 3 is positive.

use std::intrinsics::transmute;

use crate::base::{Board, Color, Move, Piece, Square};

use crate::engine::evaluate::Score;

/// A lookup table for piece values.
/// The outer index is the type of the piece
/// (in order of Pawn, Knight, Bishop, Rook, Queen, and King)
/// and the inner index is the square of the piece (from White's point of view),
/// starting with A1 as the first index, then continuing on to B1, C1, and so
/// on until H8 as index 63.
type Pst = [[Score; 64]; Piece::NUM];

#[must_use]
/// Evaluate a board based on its PST value.
/// This is slow, so under most conditions it is recommended to use
/// `value_delta()` instead if you are making moves.
/// The first value in the return type is the midgame difference, and the second
/// is the endgame difference.
pub fn evaluate(board: &Board) -> Score {
    let mut score = Score::DRAW;

    for pt in Piece::ALL {
        for sq in board[pt] & board[Color::White] {
            score += PST[pt as usize][sq as usize];
        }
        for sq in board[pt] & board[Color::Black] {
            //Invert the square that Black is on, since positional values are
            //flipped (as pawns move the other way, etc)
            let alt_sq = sq.opposite();
            score -= PST[pt as usize][alt_sq as usize];
        }
    }

    score
}

#[must_use]
/// Get the difference in PST value which would be generated by making the move
/// `m` on `board`. The first value in the return tuple is the midgame
/// difference, and the second is the endgame difference. `pst_delta` will
/// reflect how the position improves for the player making the move,
/// independed of if the player is white or black.
///
/// # Panics
///
/// This function will panic if the given move is invalid.
pub fn delta(board: &Board, m: Move) -> Score {
    let from_sq = m.from_square();
    let to_sq = m.to_square();
    let mover_type = board.type_at_square(m.from_square()).unwrap();
    let mover_idx = mover_type as usize;
    let end_type = match m.promote_type() {
        Some(pt) => pt,
        None => mover_type,
    };
    let end_idx = end_type as usize;
    let (from_alt, to_alt) = match board.player {
        Color::White => (from_sq, to_sq),
        Color::Black => (from_sq.opposite(), to_sq.opposite()),
    };
    let (from_idx, to_idx) = (from_alt as usize, to_alt as usize);

    // you always lose the value of the square you moved from
    let mut delta = PST[end_idx][to_idx] - PST[mover_idx][from_idx];

    if board[!board.player].contains(m.to_square()) {
        // conventional capture
        let to_opposite_idx = to_alt.opposite() as usize;
        let capturee_idx = board.type_at_square(to_sq).unwrap() as usize;
        delta += PST[capturee_idx][to_opposite_idx];
    }

    if m.is_en_passant() {
        let to_opposite_idx =
            (to_alt - Color::White.pawn_direction()).opposite() as usize;
        delta += PST[Piece::Pawn as usize][to_opposite_idx];
    }

    if m.is_castle() {
        let is_queen_castle = to_sq.file() == 2;
        let (rook_from_idx, rook_to_idx) = if is_queen_castle {
            (Square::A1 as usize, Square::D1 as usize)
        } else {
            (Square::H1 as usize, Square::F1 as usize)
        };

        delta += PST[Piece::Rook as usize][rook_to_idx]
            - PST[Piece::Rook as usize][rook_from_idx];
    }

    delta
}

#[rustfmt::skip] // rustfmt likes to throw a million newlines in this
/// The main piece-square table. Evaluations are paired together as (midgame, 
/// endgame) to improve cache-friendliness. The indexing order of this table 
/// has its primary index as pieces, the secondary index as squares, and the 
/// innermost index as 0 for midgame and 1 for endgame.
pub const PST: Pst = unsafe { transmute([
    [ // N
        (-120i16, -35i16), (1, -15), (-37, -9), (-12, 2), (1, 0), (-22, -6), (0, -7), (-59, -29), 
        (-58, -48), (-44, -28), (-17, -9), (4, 3), (5, 6), (-11, -34), (-29, -30), (-10, -53), 
        (-16, -10), (-14, -10), (0, 0), (6, 9), (9, 0), (11, -2), (6, -11), (-13, -24), 
        (-14, -4), (-18, -7), (3, 5), (3, 8), (8, 6), (0, 8), (19, -2), (0, -13), 
        (-3, 3), (6, 7), (10, 14), (28, 11), (14, 10), (34, 12), (4, 4), (9, 0), 
        (-37, -6), (16, 1), (4, 16), (25, 2), (38, -1), (34, -1), (35, -15), (-1, -27), 
        (-30, -8), (-17, 7), (25, -14), (-7, 5), (0, -11), (8, -19), (-3, -13), (-21, -30), 
        (-100, -26), (-52, -34), (-40, 6), (-29, -5), (7, -7), (-73, -16), (-37, -39), (-94, -38), 
    ],
    [ // B
        (-30, -13), (0, -13), (0, -2), (-12, 0), (-9, -5), (-11, -3), (-29, -5), (-35, 0), 
        (-52, -23), (0, -12), (-9, -6), (-5, 3), (3, 4), (0, -13), (17, -19), (-36, -16), 
        (-14, -10), (-1, -9), (7, 0), (4, 6), (2, 4), (8, 2), (0, -8), (-14, -18), 
        (-12, -6), (-10, -11), (2, 2), (9, 3), (6, -8), (-4, -1), (-3, -16), (6, -10), 
        (-18, 6), (-1, 4), (-5, 7), (13, 6), (16, -1), (12, 4), (-5, -1), (-6, 1), 
        (-22, 0), (0, -4), (8, 0), (7, -6), (10, -7), (18, 0), (11, -7), (-14, 2), 
        (-15, -6), (-13, 4), (-21, 10), (-36, -18), (-18, 0), (19, -10), (3, -11), (-46, -16), 
        (-27, -15), (-22, -15), (-53, -18), (-36, 3), (-30, -5), (-64, -5), (-30, -12), (-11, -32), 
    ],
    [ // R
        (0, -7), (-1, -5), (0, 0), (2, -2), (3, -5), (7, -9), (-17, -4), (-17, -7), 
        (-21, -25), (-12, -16), (-7, -8), (-1, -1), (2, -7), (1, -19), (3, -29), (-31, -24), 
        (-23, -13), (-23, -9), (-13, -7), (-6, -7), (-5, -12), (-4, -13), (-2, -20), (-25, -27), 
        (-19, -13), (-22, -2), (-7, -3), (0, -7), (0, -10), (-13, -11), (-1, -14), (-8, -22), 
        (-11, -9), (-22, -12), (0, 0), (-6, -8), (-8, -11), (7, -14), (-13, -18), (-10, -19), 
        (-8, -4), (0, 1), (0, -4), (4, 0), (-1, -3), (25, -15), (18, -12), (3, -14), 
        (4, 3), (7, 7), (26, 9), (29, 4), (21, -6), (41, 0), (26, -5), (2, -7), 
        (-3, 8), (17, -1), (11, 4), (24, 1), (27, 0), (0, 0), (0, -12), (-2, 0), 
    ],
    [ // Q
        (-20, -15), (-19, 1), (-9, -2), (4, -7), (-11, 0), (-23, -1), (-9, -3), (-34, -2), 
        (-40, -6), (-40, 2), (-5, 8), (-3, 0), (7, 0), (-2, -5), (-14, -2), (4, -25), 
        (-25, -7), (-6, -1), (-9, 2), (-1, 6), (-4, 10), (9, -2), (1, -2), (12, -11), 
        (-19, 1), (-23, 8), (-7, 3), (-4, 12), (1, 20), (0, 9), (1, 1), (1, -4), 
        (-26, -4), (-22, 1), (-14, 10), (0, 11), (14, 4), (18, -1), (6, 0), (0, -7), 
        (-28, -8), (-22, -3), (-5, -2), (9, 10), (37, -5), (40, -7), (48, -13), (30, -28), 
        (-45, -5), (-37, 5), (3, 3), (16, 7), (8, 0), (34, -8), (31, -14), (15, -24), 
        (-30, -15), (0, 3), (11, 2), (0, 6), (38, -1), (9, -7), (18, -10), (24, -23), 
    ],
    [ // P
        (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), 
        (-8, -1), (5, 3), (-16, 0), (-12, 3), (0, 9), (14, 0), (21, 3), (-11, -4), 
        (-8, -9), (-5, 2), (-5, -6), (-6, 0), (1, -2), (0, -7), (7, -2), (-4, -14), 
        (-9, 4), (-3, 10), (0, 0), (10, -6), (10, -5), (-2, -5), (-4, 2), (-14, -10), 
        (-3, 23), (11, 20), (2, 10), (16, 5), (10, 0), (7, 4), (8, 11), (-10, 8), 
        (13, 66), (33, 72), (30, 55), (39, 51), (50, 27), (45, 34), (36, 53), (11, 50), 
        (16, 121), (25, 115), (46, 104), (80, 110), (101, 106), (60, 54), (36, 69), (-2, 112), 
        (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), 
    ],
    [ // K
        (-47, -24), (9, -26), (4, -14), (-23, -17), (-11, -14), (-23, -4), (20, -17), (-30, -11), 
        (-8, -18), (14, -26), (10, -19), (-15, -13), (-1, -8), (1, -10), (25, -19), (-5, -7), 
        (-11, -15), (19, -22), (10, -8), (0, -4), (-7, 1), (-2, 0), (2, -5), (-17, -3), 
        (-32, -13), (8, -18), (9, 0), (-4, 3), (-8, 4), (-2, 3), (-19, -7), (-43, -2), 
        (-24, -1), (22, 3), (19, 4), (0, 7), (-1, 6), (16, 7), (13, 6), (-8, 5), 
        (4, 5), (30, 0), (38, 5), (9, -1), (13, 0), (42, 17), (34, 12), (4, 22), 
        (1, -9), (23, 6), (15, 0), (15, 1), (18, 1), (25, 10), (26, 4), (22, 5), 
        (-32, -35), (-2, -34), (-6, -18), (-28, -5), (-21, 2), (-17, 13), (30, 7), (0, -10), 
    ],
]) };

#[cfg(test)]
mod tests {

    use super::*;
    use crate::base::{game::Game, movegen::GenMode};

    fn delta_helper(fen: &str) {
        let mut g = Game::from_fen(fen).unwrap();
        let orig_eval = evaluate(g.board());
        for (m, _) in g.get_moves::<{ GenMode::All }>() {
            let new_eval = match g.board().player {
                Color::White => orig_eval + delta(g.board(), m),
                Color::Black => orig_eval - delta(g.board(), m),
            };
            g.make_move(m, &());
            // println!("{g}");
            assert_eq!(new_eval, evaluate(g.board()));
            g.undo().unwrap();
        }
    }

    #[test]
    /// Test that adding deltas matches the same result as taking the PST value
    /// from scratch.
    fn pst_delta_equals_base_result() {
        delta_helper(
            "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
        );
    }

    #[test]
    fn delta_captures() {
        delta_helper(
            "r1bq1b1r/ppp2kpp/2n5/3n4/2BPp3/2P5/PP3PPP/RNBQK2R b KQ d3 0 8",
        );
    }

    #[test]
    fn delta_promotion() {
        delta_helper(
            "r4bkr/pPpq2pp/2n1b3/3n4/2BPp3/2P5/1P3PPP/RNBQK2R w KQ - 1 13",
        );
    }
}
