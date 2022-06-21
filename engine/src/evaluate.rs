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

//! Static evaluation of positions.
//! 
//! Of all the parts of a chess engine, static evaluation is arguably the most 
//! important. Every leaf of the search is statically evaluated, and based on 
//! the comparisons of each evaluation, the full minimax search is achieved.
//! 
//! Fiddler uses a classical approach to static evaluation: the final evaluation 
//! is the sum of a number of rules. Each rule contributes a quantity to the 
//! evaluation.
//! 
//! Also like other engines, Fiddler uses a "tapered" evaluation: rules are 
//! given different weights at different phases of the game. To prevent sharp 
//! changes in evaluation as the phase blends, a "midgame" and "endgame" 
//! evaluation is created, and then the final evaluation is a linear combination 
//! of those two.
//! 
//! More uniquely, Fiddler is obsessed with cumulative evaluation. Often, 
//! learning facts about a board is lengthy and difficult (in computer time - it 
//! takes nanoseconds in wall time). However, it is generally easy to guess what 
//! effect a move will have on the static evaluation of a position. We therefore 
//! tag moves with their effect on the evaluation, allowing us to cheaply 
//! evaluate the final leaf position.

use std::cmp::{max, min};

use fiddler_base::{Bitboard, Board, Color, Eval, Game, Move, Piece, Score};

use crate::{
    material::material_delta,
    pst::{pst_delta, pst_evaluate},
};

use super::material;

/// Mask containing ones along the A file. Bitshifting left by a number from 0
/// through 7 will cause it to become a mask for each file.
const A_FILE_MASK: Bitboard = Bitboard::new(0x0101010101010101);

/// The value of having your own pawn doubled.
pub const DOUBLED_PAWN_VALUE: Score = Score::centipawns(-33, -31);
/// The value of having a rook with no same-colored pawns in front of it which
/// are not advanced past the 3rd rank.
pub const OPEN_ROOK_VALUE: Score = Score::centipawns(7, 15);

/// Evaluate a leaf position on a game whose cumulative values have been
/// computed correctly.
pub fn leaf_evaluate(g: &Game) -> Eval {
    let b = g.board();

    match g.is_over() {
        (true, Some(_)) => {
            return match b.player_to_move {
                Color::Black => Eval::mate_in(0),
                Color::White => -Eval::mate_in(0),
            }
        }
        (true, None) => {
            return Eval::DRAW;
        }
        _ => {}
    };

    let pos = g.position();
    let b = &pos.board;
    let leaf_val = leaf_rules(b);

    (leaf_val + pos.score).blend(phase_of(g.board()))
}

/// Compute the change in scoring that a move made on a board will cause. Used
/// in tandem with `leaf_evaluate()`.
pub fn value_delta(b: &Board, m: Move) -> Score {
    pst_delta(b, m) + material_delta(b, m)
}

/// Compute a static, cumulative-invariant evaluation of a position. It is much
/// faster in search to use cumulative evaluation, but this should be used when
/// importing positions. Static evaluation will not include the leaf rules (such
/// as number of doubled pawns), as this will be handled by `leaf_evaluate` at
/// the end of the search tree.
pub fn static_evaluate(b: &Board) -> Score {
    material::evaluate(b) + pst_evaluate(b)
    // leaf evaluations do not count here
}

/// Get the score gained from evaluations that are only performed at the leaf.
fn leaf_rules(b: &Board) -> Score {
    // Add losses due to doubled pawns
    let mut score = DOUBLED_PAWN_VALUE * net_doubled_pawns(b);

    // Add gains from open rooks
    score += OPEN_ROOK_VALUE * net_open_rooks(b);

    score
}

/// Count the number of "open" rooks (i.e., those which are not blocked by
/// unadvanced pawns) in a position. The number is a net value, so it will be
/// negative if Black has more open rooks than White.
pub fn net_open_rooks(b: &Board) -> i8 {
    // Mask for pawns which are above rank 3 (i.e. on the white half of the
    // board).
    const BELOW_RANK3: Bitboard = Bitboard::new(0xFFFFFFFF);
    // Mask for pawns which are on the black half of the board
    const ABOVE_RANK3: Bitboard = Bitboard::new(0x00000000FFFFFFFF);
    let mut net_open_rooks = 0i8;
    let rooks = b[Piece::Rook];
    let pawns = b[Piece::Pawn];
    let white = b[Color::White];
    let black = b[Color::Black];

    // count white rooks
    for wrook_sq in rooks & white {
        if wrook_sq.rank() >= 3 {
            net_open_rooks += 1;
            continue;
        }
        let pawns_in_col = (pawns & white) & (A_FILE_MASK << wrook_sq.file());
        let important_pawns = BELOW_RANK3 & pawns_in_col;
        // check that the forward-most pawn of the important pawns is in front
        // of or behind the rook
        if important_pawns.leading_zeros() > (63 - (wrook_sq as u32)) {
            // all the important pawns are behind the rook
            net_open_rooks += 1;
        }
    }

    // count black rooks
    for brook_sq in rooks & black {
        if brook_sq.rank() <= 4 {
            net_open_rooks -= 1;
            continue;
        }
        let pawns_in_col = (pawns & black) & (A_FILE_MASK << brook_sq.file());
        let important_pawns = ABOVE_RANK3 & pawns_in_col;
        // check that the lowest-rank pawn that could block the rook is behind
        // the rook
        if important_pawns.trailing_zeros() > brook_sq as u32 {
            net_open_rooks -= 1;
        }
    }

    net_open_rooks
}

/// Count the number of doubled pawns, in net. For instance, if White had 1
/// doubled pawn, and Black had 2, this function would return -1.
pub fn net_doubled_pawns(b: &Board) -> i8 {
    let white_occupancy = b[Color::White];
    let pawns = b[Piece::Pawn];
    let mut npawns: i8 = 0;
    let mut col_mask = Bitboard::new(0x0101010101010101);
    for _ in 0..8 {
        let col_pawns = pawns & col_mask;

        // all ones on the A column, shifted left by the col
        let num_black_doubled_pawns = match ((!white_occupancy) & col_pawns).count_ones() {
            0 => 0,
            x => x as i8 - 1,
        };
        let num_white_doubled_pawns = match (white_occupancy & col_pawns).count_ones() {
            0 => 0,
            x => x as i8 - 1,
        };

        npawns -= num_black_doubled_pawns;
        npawns += num_white_doubled_pawns;

        col_mask <<= 1;
    }

    npawns
}

/// Get a blending float describing the current phase of the game. Will range
/// from 0 (full endgame) to 1 (full midgame).
pub fn phase_of(b: &Board) -> f32 {
    const MG_LIMIT: Eval = Eval::centipawns(2500);
    const EG_LIMIT: Eval = Eval::centipawns(1400);
    // amount of non-pawn material in the board, under midgame values
    let mg_npm = {
        let mut total = Eval::DRAW;
        for pt in Piece::NON_PAWN_TYPES {
            total += material::value(pt).mg * b[pt].count_ones();
        }
        total
    };
    let bounded_npm = max(MG_LIMIT, min(EG_LIMIT, mg_npm));

    (bounded_npm - EG_LIMIT).float_val() / (MG_LIMIT - EG_LIMIT).float_val()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fiddler_base::movegen::{get_moves, NoopNominator, ALL};

    fn delta_helper(fen: &str) {
        let mut g = Game::from_fen(fen, static_evaluate).unwrap();
        for (m, _) in get_moves::<ALL, NoopNominator>(g.position()) {
            g.make_move(m, value_delta(g.board(), m));
            // println!("{g}");
            assert_eq!(static_evaluate(g.board()), g.position().score);
            g.undo().unwrap();
        }
    }

    #[test]
    fn test_delta_captures() {
        delta_helper("r1bq1b1r/ppp2kpp/2n5/3n4/2BPp3/2P5/PP3PPP/RNBQK2R b KQ d3 0 8");
    }

    #[test]
    fn test_delta_promotion() {
        // undoubling capture promotion is possible
        delta_helper("r4bkr/pPpq2pp/2n1b3/3n4/2BPp3/2P5/1P3PPP/RNBQK2R w KQ - 1 13");
    }
}
