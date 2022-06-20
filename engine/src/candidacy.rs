use crate::{
    evaluate::{phase_blend, phase_of, value_delta},
};

use super::material;
use fiddler_base::{movegen::NominateMove, Eval, Move, Position, Score};
pub struct PstNominate {}

impl NominateMove for PstNominate {
    type Output = (Score, Eval);

    #[inline(always)]
    fn score(m: Move, pos: &Position) -> Self::Output {
        let delta = value_delta(&pos.board, m);
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
    let phase = phase_of(b);

    // Worst case, we don't keep the piece we captured
    let mut worst_case_delta = delta;
    let mover_value = material::value(mover_type);
    worst_case_delta.0 -= mover_value.0;
    worst_case_delta.1 -= mover_value.1;
    phase_blend(phase, worst_case_delta)
}
