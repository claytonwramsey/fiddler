use std::cmp::{max, min};

use fiddler_base::{Bitboard, Board, Color, Eval, Game, Piece, Score};

use super::material;

/// The value of having your own pawn doubled.
pub const DOUBLED_PAWN_VALUE: Score = (Eval::centipawns(-33), Eval::centipawns(-29));
/// The value of having a rook with no same-colored pawns in front of it which
/// are not advanced past the 3rd rank.
pub const OPEN_ROOK_VALUE: Score = (Eval::centipawns(41), Eval::centipawns(12));

/// Evaluate a quiet position.
pub fn evaluate(g: &mut Game) -> Eval {
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

    let (mut mg_eval, mut eg_eval) = pos.pst_val;
    let material = material::evaluate(b);
    mg_eval += material.0;
    eg_eval += material.1;

    // Add losses due to doubled pawns
    let ndoubled = net_doubled_pawns(b);
    mg_eval += DOUBLED_PAWN_VALUE.0 * ndoubled;
    eg_eval += DOUBLED_PAWN_VALUE.1 * ndoubled;

    // Add gains from open rooks
    let nopen = net_open_rooks(b);
    mg_eval += OPEN_ROOK_VALUE.0 * nopen;
    eg_eval += OPEN_ROOK_VALUE.1 * nopen;

    blend_eval(g.board(), (mg_eval, eg_eval))
}

/// Count the number of "open" rooks (i.e., those which are not blocked by
/// unadvanced pawns) in a position. The number is a net value, so it will be
/// negative if Black has more open rooks than White.
pub fn net_open_rooks(b: &Board) -> i8 {
    const A_FILE_MASK: Bitboard = Bitboard::new(0x0101010101010101);
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
        let pawns_in_col = (pawns & white) & (A_FILE_MASK << brook_sq.file());
        let important_pawns = ABOVE_RANK3 & pawns_in_col;
        // check that the lowest-rank pawn that could block the rook is behind
        // the rook
        if important_pawns.trailing_zeros() > brook_sq as u32 {
            net_open_rooks -= 1;
        }
    }

    net_open_rooks
}

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
            total += material::value(pt).0 * b[pt].count_ones();
        }
        total
    };
    let bounded_npm = max(MG_LIMIT, min(EG_LIMIT, mg_npm));

    (bounded_npm - EG_LIMIT).float_val() / (MG_LIMIT - EG_LIMIT).float_val()
}

#[inline(always)]
/// Blend the evaluation of a position between the midgame and endgame.
pub fn blend_eval(b: &Board, score: Score) -> Eval {
    phase_blend(phase_of(b), score)
}

pub fn phase_blend(phase: f32, score: Score) -> Eval {
    score.0 * phase + score.1 * (1. - phase)
}
