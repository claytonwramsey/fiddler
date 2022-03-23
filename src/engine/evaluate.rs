use crate::base::Bitboard;
use crate::base::Color;
use crate::base::Eval;
use crate::base::Game;
use crate::base::Piece;

use super::greedy::greedy_evaluate;

/// The value of having an opponent's pawn doubled.
const DOUBLED_PAWN_VALUE: Eval = Eval::centipawns(10);

/// Evaluate a quiet position.
pub fn evaluate(g: &mut Game) -> Eval {
    let b = g.board();

    match g.is_game_over() {
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

    blend_eval(g.len(), mg_eval, eg_eval)
}

#[inline]
/// Blend the evaluation of a position between the midgame and endgame.
pub fn blend_eval(turn_id: usize, mg_eval: Eval, eg_eval: Eval) -> Eval {
    match turn_id {
        l if l < 20 => mg_eval,
        l if l > 80 => eg_eval,
        _ => {
            let eg_scale_factor = (turn_id as f32 - 20.) / 60.;
            mg_eval * (1. - eg_scale_factor) + eg_eval * eg_scale_factor
        }
    }
}
