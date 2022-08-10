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

//! Time management heuristics and decision making.
//!
//! In a match, a chess engine is usually given a budget of time for the entire
//! game, and it is the engine's duty to decide how much to use when making each
//! move.
//! More sophisticated engines do an analysis of the position and guess its
//! complexity, giving themselves more time in positions which are more complex.
//! For now, Fiddler is not so intelligent, and instead rations time to itself
//! indiscriminately.

use std::cmp::min;

use fiddler_base::Color;

#[must_use]
#[allow(
    clippy::module_name_repetitions,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
/// Decide how much time to search a position, given UCI information about the
/// time remaining.
///
/// Inputs:
/// * `movestogo`: the number of moves remaining until the next increment.
/// *`increment`: the time increment that each player will get after
///     they play a move, measured in milliseconds.
/// * `remaining`: the remaining time that each player has, measured in
///     milliseconds.
/// * `player`: the color of the player for whom we are making the timing
///     decision.
pub fn get_search_time(
    movestogo: Option<u8>,
    increment: (u32, u32),
    remaining: (u32, u32),
    player: Color,
) -> u32 {
    // for now, simply try to exhaust our remaining time to the increment, with
    // a little buffer time.
    let (our_inc, our_remaining) = match player {
        Color::White => (increment.0, remaining.0),
        Color::Black => (increment.1, remaining.1),
    };

    let rem_float = our_remaining as f32;
    if let Some(moves) = movestogo {
        min(
            800 * our_remaining / (1000 * u32::from(moves)) + our_inc,
            (0.85 * rem_float) as u32,
        )
    } else {
        // use a fraction of our remaining time.
        min(our_remaining / 80 + our_inc, (0.9 * rem_float) as u32)
    }
}
