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

//! Move candidacy. 
//! 
//! Because it's important that good moves are sorted first, a good candidacy 
//! function can dramatically improve an engine's performance. 
//! 
//! Fiddler's candidacy function is relatively simple: it calculates the move's 
//! effect on evaluation, and then hedges its bets by marking captures using 
//! high-value pieces as poor.

use crate::evaluate::{phase_of, value_delta};

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
    worst_case_delta -= mover_value;
    worst_case_delta.blend(phase)
}
