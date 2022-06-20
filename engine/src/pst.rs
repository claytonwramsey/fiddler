/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

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

//! A module containing the information for Piece-Square Tables (PSTs). A PST
//! is given for both the early and endgame.

use fiddler_base::{Board, Color, Eval, Move, Piece, Score, Square};

/// A lookup table for piece values. The outer index is the type of the piece
/// (in order of Pawn, Knight, Bishop, Rook, Queen, and King)
/// and the inner index is the square of the piece (from White's point of view)
/// , starting with A1 as the first index, then continuing on to B1, C1, and so
/// on until H8 as index 63.
type Pst = [[Score; 64]; Piece::NUM_TYPES];

/// A PST which is given in millipawns.
type CentiPst = [[(i16, i16); 64]; Piece::NUM_TYPES];

/// Evaluate a board based on its PST value. This is slow, so under most
/// conditions it is recommended to use `value_delta()` instead if you are making
/// moves. The first value in the return type is the midgame difference, and
/// the second is the endgame difference.
pub fn pst_evaluate(board: &Board) -> Score {
    let mut score = (Eval::DRAW, Eval::DRAW);

    for pt in Piece::ALL_TYPES {
        for sq in board[pt] & board[Color::White] {
            score.0 += PST[pt as usize][sq as usize].0;
            score.1 += PST[pt as usize][sq as usize].1;
        }
        for sq in board[pt] & board[Color::Black] {
            //Invert the square that Black is on, since positional values are
            //flipped (as pawns move the other way, etc)
            let alt_sq = sq.opposite();

            score.0 -= PST[pt as usize][alt_sq as usize].0;
            score.1 -= PST[pt as usize][alt_sq as usize].1;
        }
    }

    score
}

/// Get the difference in PST value which would be generated by making the move
/// `m` on `board`. The first value in the return tuple is the midgame
/// difference, and the second is the endgame difference. `pst_delta` will
/// reflect how the position improves for the player making the move,
/// independed of if the player is white or black.
///
/// # Panics
///
/// `pst_delta` will panic if the given move is invalid.
pub fn pst_delta(board: &Board, m: Move) -> Score {
    let from_sq = m.from_square();
    let to_sq = m.to_square();
    let mover_type = board.type_at_square(m.from_square()).unwrap();
    let mover_idx = mover_type as usize;
    let end_type = match m.promote_type() {
        Some(pt) => pt,
        None => mover_type,
    };
    let end_idx = end_type as usize;
    let (from_alt, to_alt) = match board.player_to_move {
        Color::White => (from_sq, to_sq),
        Color::Black => (from_sq.opposite(), to_sq.opposite()),
    };
    let (from_idx, to_idx) = (from_alt as usize, to_alt as usize);

    // you always lose the value of the square you moved from
    let mut delta = (
        PST[end_idx][to_idx].0 - PST[mover_idx][from_idx].0,
        PST[end_idx][to_idx].1 - PST[mover_idx][from_idx].1,
    );

    if board[!board.player_to_move].contains(m.to_square()) {
        // conventional capture
        let to_opposite_idx = to_alt.opposite() as usize;
        let capturee_idx = board.type_at_square(to_sq).unwrap() as usize;
        delta.0 += PST[capturee_idx][to_opposite_idx].0;
        delta.1 += PST[capturee_idx][to_opposite_idx].1;
    }

    if m.is_en_passant() {
        let to_opposite_idx = (to_alt - Color::White.pawn_direction()).opposite() as usize;
        delta.0 += PST[Piece::Pawn as usize][to_opposite_idx].0;
        delta.1 += PST[Piece::Pawn as usize][to_opposite_idx].1;
    }

    if m.is_castle() {
        let is_queen_castle = to_sq.file() == 2;
        let (rook_from_idx, rook_to_idx) = match is_queen_castle {
            true => (Square::A1 as usize, Square::D1 as usize),
            false => (Square::H1 as usize, Square::F1 as usize),
        };

        delta.0 +=
            PST[Piece::Rook as usize][rook_to_idx].0 - PST[Piece::Rook as usize][rook_from_idx].0;
        delta.1 +=
            PST[Piece::Rook as usize][rook_to_idx].1 - PST[Piece::Rook as usize][rook_from_idx].1;
    }

    (delta.0, delta.1)
}

/// A function used for ergonomics to convert from a table of millipawn values
/// to a table of `Eval`s.
const fn expand_table(centi_table: CentiPst) -> Pst {
    let mut table = [[(Eval::DRAW, Eval::DRAW); 64]; Piece::NUM_TYPES];
    let mut piece_idx = 0;
    // I would use for-loops here, but those are unsupported in const fns.
    while piece_idx < Piece::NUM_TYPES {
        let mut sq_idx = 0;
        while sq_idx < 64 {
            let int_score = centi_table[piece_idx][sq_idx];
            table[piece_idx][sq_idx] =
                (Eval::centipawns(int_score.0), Eval::centipawns(int_score.1));
            sq_idx += 1;
        }
        piece_idx += 1;
    }
    table
}

#[rustfmt::skip] // rustfmt likes to throw a million newlines in this
/// The main piece-square table. Evaluations are paired together as (midgame, 
/// endgame) to improve cache-friendliness. The indexing order of this table 
/// has its primary index as pieces, the secondary index as squares, and the 
/// innermost index as 0 for midgame and 1 for endgame.
pub const PST: Pst = expand_table([
    [ // N
        (-175, -55), (-29, -21), (-68, -17), (-52, -18), (-50, -33), (-52, -24), (-32, -18), (-95, -32),
        (-95, -15), (-67, -7), (-16, 2), (0, 6), (-7, 4), (-24, 3), (-39, -8), (-42, -37),
        (-44, -31), (-7, 10), (38, 2), (13, 1), (12, 1), (41, 21), (9, 5), (-45, -22),
        (-18, -35), (-17, 2), (17, 25), (14, 19), (17, 4), (6, 5), (0, 3), (-10, -21),
        (-3, -27), (17, 4), (40, 6), (59, 23), (44, 12), (71, 23), (15, 4), (33, -23),
        (-12, -21), (32, 4), (-14, 10), (68, -6), (54, 3), (-15, 2), (47, -9), (23, -20),
        (-36, -42), (-20, -32), (69, -9), (-12, 2), (34, 1), (35, -6), (-4, -15), (-9, -33),
        (-89, -43), (-56, -53), (-52, -18), (-40, -26), (-19, -25), (-93, -30), (-43, -38), (-108, -27),
    ],
    [ // B
        (-60, -16), (-19, -3), (-3, -15), (-58, -7), (-56, -4), (-8, -16), (-34, -12), (-62, -1),
        (-25, 0), (5, -7), (0, -5), (-7, 0), (2, 5), (-20, 8), (24, 15), (-21, 1),
        (-6, -1), (6, 13), (1, 13), (38, 6), (32, -2), (-8, 23), (12, 17), (-14, 3),
        (-21, -6), (-17, 0), (32, 14), (8, 4), (0, 3), (25, 4), (-14, -2), (-5, 1),
        (0, -5), (15, 0), (14, 17), (24, 8), (24, 12), (16, 6), (10, 11), (3, -11),
        (-14, -12), (11, 4), (-54, 4), (22, 6), (-1, -3), (-84, -7), (44, 1), (29, -7),
        (-36, 2), (-8, 9), (-4, 4), (-111, 6), (-86, -1), (-11, 0), (-32, 3), (-3, -17),
        (-28, -10), (-44, -2), (-53, 6), (-53, 1), (-30, -12), (-105, -7), (-37, -9), (-8, -17),
    ],
    [ // R
        (-36, 15), (-22, 8), (-8, -2), (2, -4), (-4, 4), (-3, -1), (-8, -1), (-47, 0),
        (-47, -12), (-34, 8), (-26, -3), (-27, 2), (-32, 0), (-23, -3), (-19, 14), (-28, 4),
        (-31, 0), (-20, 3), (-22, 1), (-24, -19), (-30, -4), (-19, 1), (-8, -3), (-24, -8),
        (-12, 0), (-15, 7), (-8, 0), (-22, 0), (-24, -2), (-23, 5), (-26, 1), (-22, -11),
        (-8, 3), (-14, 0), (5, 6), (-2, -4), (-6, 0), (-2, 3), (-3, -20), (-5, 1),
        (1, -11), (14, 0), (14, 4), (14, -11), (2, -14), (23, -18), (28, -9), (16, -3),
        (15, 0), (25, 2), (40, 6), (35, 0), (37, 5), (44, 0), (42, 18), (33, 7),
        (-5, -11), (22, 3), (12, -8), (-4, -14), (-5, -3), (-22, 0), (15, -6), (19, 8),
    ],
    [ // Q
        (-40, -7), (-51, -11), (-47, 0), (15, -2), (-33, -5), (-63, -3), (-9, 2), (-30, -12),
        (-109, 0), (-62, -1), (-4, 17), (-2, -2), (-7, -2), (-7, -6), (-13, 1), (-5, -13),
        (-46, -11), (-2, -6), (-7, 5), (-2, 2), (-5, 12), (10, 4), (2, 7), (-8, -2),
        (-5, -3), (-21, 8), (0, 0), (26, 8), (8, 17), (-1, 0), (5, -4), (7, 10),
        (-20, 0), (-7, -3), (0, 1), (30, 4), (28, -1), (35, 1), (18, -3), (48, 6),
        (-22, -14), (-4, 0), (24, 0), (41, 0), (65, 4), (93, 15), (114, -1), (76, -20),
        (-29, -12), (-20, -4), (8, 3), (18, -7), (35, -17), (108, 7), (63, 3), (95, -16),
        (-6, -19), (17, -10), (25, -9), (19, 6), (52, 2), (53, -4), (44, 0), (52, -23),
    ],
    [ // P
        (-1, -2), (-9, -17), (-9, 8), (-1, -15), (-4, -8), (14, -19), (-5, 3), (-10, 0),
        (-3, 4), (14, 6), (-1, 16), (-38, -9), (2, -10), (32, 11), (34, 16), (-3, 3),
        (-10, 1), (-5, 0), (0, -5), (-9, 6), (6, 3), (-12, -11), (7, -6), (-10, 6),
        (-9, -1), (0, -4), (10, -3), (33, 11), (27, 12), (-4, 0), (-10, -3), (-16, -3),
        (10, -5), (14, 7), (15, 6), (24, 28), (29, 20), (17, 12), (10, -1), (7, 11),
        (41, 10), (49, 10), (30, 17), (59, 19), (50, 19), (46, 21), (51, 19), (40, 0),
        (61, 55), (58, 34), (68, 50), (104, 54), (88, 36), (71, 53), (64, 41), (46, 50),
        (3, 1), (-2, 0), (-8, -4), (12, -1), (-11, 0), (-5, -5), (-1, -5), (0, -4),
    ],
    [ // K
        (-44, -41), (19, -42), (22, -17), (-60, -18), (-6, -41), (-58, -27), (36, -28), (-30, -47),
        (-44, -25), (-19, -32), (-29, -9), (-36, -9), (-33, 0), (-15, -5), (-4, -15), (-32, -15),
        (-45, -40), (-30, -4), (-20, 4), (-23, 27), (-14, 41), (-18, 18), (-18, -14), (-53, -29),
        (-46, -23), (-15, -10), (-4, 39), (0, 40), (4, 45), (-4, 23), (-12, -4), (-46, -16),
        (-18, -29), (15, 3), (27, 19), (15, 44), (21, 31), (19, 28), (15, -3), (-23, -30),
        (-19, -25), (35, -13), (29, 10), (30, 34), (26, 39), (41, 6), (41, -7), (7, -13),
        (-18, -27), (27, -18), (34, 1), (15, 0), (25, 1), (27, 1), (47, -10), (14, -15),
        (-38, -44), (-14, -45), (-19, -27), (-41, -9), (-25, -21), (-15, 8), (32, -37), (-2, -39),
    ],
]);

#[cfg(test)]
mod tests {

    use super::*;
    use fiddler_base::movegen::{get_moves, NoopNominator, ALL};
    use fiddler_base::{Game};

    fn delta_helper(fen: &str) {
        let mut g = Game::from_fen(fen, pst_evaluate).unwrap();
        for (m, _) in get_moves::<ALL, NoopNominator>(g.position()) {
            g.make_move(m, pst_delta(g.board(), m));
            // println!("{g}");
            assert_eq!(g.position().pst_val, pst_evaluate(g.board()));
            g.undo().unwrap();
        }
    }

    #[test]
    /// Test that adding deltas matches the same result as taking the PST value
    /// from scratch.
    fn test_pst_delta_equals_base_result() {
        delta_helper("r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7");
    }

    #[test]
    fn test_delta_captures() {
        delta_helper("r1bq1b1r/ppp2kpp/2n5/3n4/2BPp3/2P5/PP3PPP/RNBQK2R b KQ d3 0 8");
    }

    #[test]
    fn test_delta_promotion() {
        delta_helper("r4bkr/pPpq2pp/2n1b3/3n4/2BPp3/2P5/1P3PPP/RNBQK2R w KQ - 1 13");
    }
}
