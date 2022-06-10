//! Time management heuristics and decision making.

use std::cmp::min;

use fiddler_base::Color;

/// Decide how much time to search a position, given UCI information about the
/// time remaining.
///
/// `movestogo` is the number of moves remaining until the next increment.
///
/// `increment` is the time increment that each player will get after
/// they play a move, measured in milliseconds.
///
/// `remaining` is the remaining time that each player has, measured in
/// milliseconds.
///
/// `player` is the color of the player for whom we are making the timing
/// decision.
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

    if let Some(moves) = movestogo {
        min(900 * our_remaining / (1000 * (moves as u32)) + our_inc, (0.9 * our_remaining as f32) as u32)
    } else {
        // use a fraction of our remaining time.
        min(our_remaining / 80 + our_inc, (0.9 * our_remaining as f32) as u32)
    }
}
