use std::cmp::max;
use std::cmp::min;

use fiddler_base::Bitboard;
use fiddler_base::Board;
use fiddler_base::Color;
use fiddler_base::Eval;
use fiddler_base::Game;
use fiddler_base::Piece;

use super::greedy::greedy_evaluate;
use super::greedy::piece_value;

/// The value of having an opponent's pawn doubled.
const DOUBLED_PAWN_VALUE: Eval = Eval::centipawns(10);

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
    let material = greedy_evaluate(b);
    mg_eval += material;
    eg_eval += material;

    // Add losses due to doubled pawns
    let white_occupancy = b[Color::White];
    let pawns = b[Piece::Pawn];
    let mut col_mask = Bitboard::new(0x0101010101010101);
    for _ in 0..8 {
        let col_pawns = pawns & col_mask;

        // all ones on the A column, shifted left by the col
        let num_black_doubled_pawns = match ((!white_occupancy) & col_pawns).count_ones() {
            0 => 0,
            x => x - 1,
        };
        let num_white_doubled_pawns = match (white_occupancy & col_pawns).count_ones() {
            0 => 0,
            x => x - 1,
        };

        eg_eval += DOUBLED_PAWN_VALUE * num_black_doubled_pawns;
        eg_eval -= DOUBLED_PAWN_VALUE * num_white_doubled_pawns;

        col_mask <<= 1;
    }

    blend_eval(g.board(), mg_eval, eg_eval)
}

/// Get a blending float describing the current phase of the game. Will range
/// from 0 (full endgame) to 1 (full midgame).
pub fn phase_of(b: &Board) -> f32 {
    const MG_LIMIT: Eval = Eval::centipawns(2200);
    const EG_LIMIT: Eval = Eval::centipawns(1000);
    let npm = {
        let mut total = Eval::DRAW;
        for pt in Piece::NON_PAWN_TYPES {
            total += piece_value(pt) * b[pt].count_ones();
        }
        total
    };
    let bounded_npm = max(MG_LIMIT, min(EG_LIMIT, npm));

    (bounded_npm - EG_LIMIT).float_val() / (MG_LIMIT - EG_LIMIT).float_val()
}

#[inline]
/// Blend the evaluation of a position between the midgame and endgame.
pub fn blend_eval(b: &Board, mg_eval: Eval, eg_eval: Eval) -> Eval {
    let phase = phase_of(b);
    mg_eval * phase + eg_eval * (1. - phase)
}
