use crate::base::Color;
use crate::base::Eval;
use crate::base::Game;
use crate::base::Move;
use crate::base::Piece;
use crate::engine::greedy;
use crate::engine::pst::{ENDGAME_VALUE, MIDGAME_VALUE};

use std::cmp::max;

use super::evaluate::blend_eval;

/// Create an estimate for how good a move is.
/// # Panics
/// if the given move is illegal.
pub fn candidacy(g: &mut Game, _mgen: &MoveGenerator, m: Move) -> Eval {
    let b = g.board();
    let mover_type = b.type_at_square(m.from_square()).unwrap();
    let promote_type = m.promote_type();
    let capture_type = b.type_at_square(m.to_square());

    let pos_from_sq = match b.player_to_move {
        Color::White => m.from_square(),
        Color::Black => m.from_square().opposite(),
    };
    let pos_to_sq = match b.player_to_move {
        Color::White => m.to_square(),
        Color::Black => m.to_square().opposite(),
    };
    let pos_gain_type = match promote_type {
        None => mover_type,
        Some(p) => p,
    };

    let mg_pos_loss = MIDGAME_VALUE[mover_type as usize][pos_from_sq as usize];
    let eg_pos_loss = ENDGAME_VALUE[mover_type as usize][pos_from_sq as usize];
    let mg_pos_gain = MIDGAME_VALUE[pos_gain_type as usize][pos_to_sq as usize];
    let eg_pos_gain = ENDGAME_VALUE[pos_gain_type as usize][pos_to_sq as usize];

    let (mg_capture_gain, eg_capture_gain) = match capture_type {
        None => (Eval::DRAW, Eval::DRAW),
        Some(ct) => (
            MIDGAME_VALUE[ct as usize][pos_to_sq as usize],
            ENDGAME_VALUE[ct as usize][pos_to_sq as usize],
        ),
    };

    let mg_delta = mg_pos_gain + mg_capture_gain - mg_pos_loss;
    let eg_delta = eg_pos_gain + eg_capture_gain - eg_pos_loss;
    let pos_delta = blend_eval(g.len(), mg_delta, eg_delta);

    // Best case, we keep the piece we captured
    let mut best_case_material = match capture_type {
        Some(p) => greedy::piece_value(p),
        None => Eval::DRAW,
    };
    if promote_type != None {
        best_case_material +=
            greedy::piece_value(promote_type.unwrap()) - greedy::piece_value(Piece::Pawn);
    }
    //Worst case, we lose the piece we moved
    let worst_case_material = best_case_material - greedy::piece_value(mover_type);

    pos_delta + max(worst_case_material, Eval::DRAW)
}
