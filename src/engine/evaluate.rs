use crate::base::Piece;
use crate::base::Bitboard;
use crate::base::Color;
use crate::base::Eval;
use crate::base::Game;
use crate::engine::greedy::greedy_evaluate;

use crate::engine::pst::{ENDGAME_VALUE, MIDGAME_VALUE};

/// The value of having an opponent's pawn doubled.
const DOUBLED_PAWN_VALUE: Eval = Eval::millipawns(100);

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

    let b = g.board();

    // the evaluation of the position as a pure midgame.
    let mut mg_eval = greedy_evaluate(b);

    // the evaluation of the position as a pure endgame.
    let mut eg_eval = mg_eval;

    for pt in [Piece::Pawn, Piece::Bishop, Piece::Knight, Piece::King] {
        for sq in b[pt] & b[Color::White] {
            mg_eval += MIDGAME_VALUE[pt as usize][sq as usize];
            eg_eval += ENDGAME_VALUE[pt as usize][sq as usize];
        }
        for sq in b[pt] & b[Color::Black] {
            //Invert the square that Black is on, since positional values are
            //flipped (as pawns move the other way, etc)
            let alt_sq = sq.opposite();

            mg_eval -= MIDGAME_VALUE[pt as usize][alt_sq as usize];
            eg_eval -= ENDGAME_VALUE[pt as usize][alt_sq as usize];
        }
    }

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
