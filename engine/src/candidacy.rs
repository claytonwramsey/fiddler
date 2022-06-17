use crate::{
    evaluate::{phase_blend, phase_of},
    pst::pst_delta,
};

use super::material;
use fiddler_base::{movegen::NominateMove, Eval, Move, Piece, Position, Score};

use std::cmp::max;

pub struct PstNominate {}

impl NominateMove for PstNominate {
    type Output = (Score, Eval);

    #[inline(always)]
    fn score(m: Move, pos: &Position) -> Self::Output {
        let delta = pst_delta(&pos.board, m);
        (delta, candidacy(pos, m, delta))
    }
}

#[allow(unused)]
/// Create an estimate for how good a move is. `delta` is the PST difference
/// created by this move. Requires that `m` must be a legal move in `pos`.
///
/// # Panics
///
/// This function may panic if the given move is illegal.
pub fn candidacy(pos: &Position, m: Move, delta: Score) -> Eval {
    let b = &pos.board;
    let mover_type = b.type_at_square(m.from_square()).unwrap();
    let capture_type = b.type_at_square(m.to_square());
    let phase = phase_of(b);

    // Best case, we keep the piece we captured
    let mut best_case_material = match capture_type {
        Some(p) => phase_blend(phase, material::value(p)),
        None => Eval::DRAW,
    };
    if let Some(pt) = m.promote_type() {
        let mut prom_score = material::value(pt);
        let pawn_score = material::value(Piece::Pawn);
        prom_score.0 -= pawn_score.0;
        prom_score.1 -= pawn_score.1;
        best_case_material += phase_blend(phase, prom_score);
    }
    //Worst case, we lose the piece we moved
    let worst_case_material = best_case_material - phase_blend(phase, material::value(mover_type));

    phase_blend(phase, delta) + max(worst_case_material, Eval::DRAW)
}
