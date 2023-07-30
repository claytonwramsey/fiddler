/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

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

//! Material values for each piece.
//!
//! Every piece is assigned a numeric value in centipawns (cp).
//! Under normal conditions, a centipawn defined as 100cp; however, the consequences of tuning have
//! yielded values for pawns slightly off of that mark.
//!
//! In traditional chess, pawns are worth 100cp, knights and bishops are worth 300cp, rooks are
//! worth 500cp, and queens are worth 900cp each.
//! However, any chess player worth their salt might tell you that bishops are a little more
//! valuable than knights.
//! Empirically, the engine agrees.

use crate::{
    base::{game::Game, Move, Piece},
    engine::evaluate::Score,
};

#[must_use]
/// Get the value of one piece by its type.
pub const fn value(pt: Piece) -> Score {
    match pt {
        Piece::Knight => Score::centipawns(244, 241),
        Piece::Bishop => Score::centipawns(261, 263),
        Piece::Rook => Score::centipawns(436, 396),
        Piece::Queen => Score::centipawns(887, 820),
        Piece::Pawn => Score::centipawns(96, 94),
        Piece::King => Score::DRAW,
    }
}

#[must_use]
/// Compute the effect that a move will have on the total material evaluation of the board it will
/// be played on, in the perspective of the player to move.
pub fn delta(g: &Game, m: Move) -> Score {
    // material only ever changes value based on captures and promotions, so this is easy
    let mut gain = if m.is_en_passant() {
        value(Piece::Pawn)
    } else {
        g[m.destination()].map_or(Score::DRAW, |(pt, _)| value(pt))
    };

    if let Some(promote_type) = m.promote_type() {
        // we already checked that m is a promotion, so we can trust that it has a promotion
        gain += value(promote_type);
        gain -= value(Piece::Pawn);
    }

    // we need not put this delta in perspective, that is somebody else's job
    gain
}

#[must_use]
#[allow(clippy::cast_possible_wrap)]
/// Evaluate a position solely by the amount of material available.
/// Returns a larger value for positions favoring the player to move and a lesser one for those
/// which are worse for the player to move.
pub fn evaluate(g: &Game) -> Score {
    let mut score = Score::centipawns(0, 0);

    let player = g.meta().player;
    let allies = g.by_color(player);
    let enemies = g.by_color(!player);

    for pt in Piece::ALL {
        // Total the quantity of white and black pieces of this type, and multiply their individual
        // value to get the net effect on the eval.
        let pt_squares = g.by_piece(pt);
        let diff = (allies & pt_squares).len() as i8 - (enemies & pt_squares).len() as i8;
        score += value(pt) * diff;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::movegen::{make_move_vec, GenMode};

    /// Helper function to verify that the implementation of [`delta`] is correct.
    ///
    /// For each move reachable from a board with start position `fen`, this will assert that the
    /// result of [`evaluate`] is equal to the sum of the original evaluation and the computed
    /// delta for the move.
    fn delta_helper(fen: &str) {
        let mut game = Game::from_fen(fen).unwrap();
        let orig_eval = evaluate(&game);
        for m in make_move_vec::<{ GenMode::All }>(&game) {
            let d = delta(&game, m);
            println!("m {m}, original {orig_eval}, delta {d}");
            let new_eval = -d - orig_eval;
            game.make_move(m);
            assert_eq!(evaluate(&game), new_eval);
            game.undo().unwrap();
        }
    }

    #[test]
    fn delta_captures() {
        delta_helper("r1bq1b1r/ppp2kpp/2n5/3n4/2BPp3/2P5/PP3PPP/RNBQK2R b KQ d3 0 8");
    }

    #[test]
    fn delta_promotion() {
        // undoubling capture promotion is possible
        delta_helper("r4bkr/pPpq2pp/2n1b3/3n4/2BPp3/2P5/1P3PPP/RNBQK2R w KQ - 1 13");
    }
}
