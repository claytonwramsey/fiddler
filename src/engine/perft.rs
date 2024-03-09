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

//! Implementation of a perft routine, used for verifying the correctness of a search.
//!
//! This implementation is not interruptible, so is perhaps not fully compliant with the UCI
//! standard, but `perft` is already a non-standard extension, so we simply prefer to be as closely
//! aligned to Stockfish as possible.

use std::time::Instant;

use crate::base::{
    game::Game,
    movegen::{get_moves, make_move_vec, GenMode},
};

#[allow(clippy::cast_precision_loss, clippy::similar_names)]
/// Perform a performance test on the move generator.
/// Returns the number of independent paths to a leaf reachable in `depth` plies from a board
/// with starting position `g`.
pub fn perft(g: &mut Game, depth: u8) -> u64 {
    /// The core search algorithm for perft.
    fn helper<const DIVIDE: bool>(g: &mut Game, depth: u8) -> u64 {
        let mut total = 0;
        if depth == 1 {
            get_moves::<{ GenMode::All }>(g, |m| {
                if DIVIDE {
                    println!("{}: 1", m.to_uci());
                }
                total += 1;
            });
        } else {
            // to prevent a violation of Rust's aliasing rules, we can't use a callback here.
            // instead, we can just collect the moves into a vector.
            for m in make_move_vec::<{ GenMode::All }>(g) {
                g.make_move(m);
                let count = helper::<false>(g, depth - 1);
                if DIVIDE {
                    println!("{}: {count}", m.to_uci());
                }
                g.undo().unwrap();
                total += count;
            }
        };

        total
    }

    let start = Instant::now();
    let n = if depth == 0 {
        1
    } else {
        helper::<true>(g, depth)
    };
    let end = Instant::now();
    let time_ms = (end - start).as_millis();
    let nps = 1000 * u128::from(n) / if time_ms == 0 { 1 } else { time_ms };

    println!();
    println!("Nodes searched: {n}");
    println!();
    println!("info depth {depth} nodes {n} time {time_ms} nps {nps}");

    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::cast_possible_truncation)]
    #[cfg_attr(miri, ignore)]
    fn perft_assistant(fen: &str, node_counts: &[u64]) {
        for (i, num) in node_counts.iter().enumerate() {
            let mut g = Game::from_fen(fen).unwrap();
            assert_eq!(*num, perft(&mut g, i as u8));
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Test the perft values for the board starting position.
    fn start_position() {
        perft_assistant(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &[1, 20, 400, 8_902, 197_281, 4_865_609, 119_060_324],
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Test the perft values for the
    /// [Kiwipete](https://www.chessprogramming.org/Perft_Results#Position_2)
    /// position.
    fn kiwipete() {
        perft_assistant(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            &[1, 48, 2039, 97_862, 4_085_603, 193_690_690],
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn endgame() {
        // https://www.chessprogramming.org/Perft_Results#Position_3
        perft_assistant(
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            &[1, 14, 191, 2_812, 43_238, 674_624, 11_030_083, 178_633_661],
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    /// Test the perft values for an unbalanced position.
    /// Uses results from
    /// [the Chess Programming wiki](https://www.chessprogramming.org/Perft_Results#Position_4).
    fn unbalanced() {
        perft_assistant(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            &[1, 6, 264, 9_467, 422_333, 15_833_292, 706_045_033],
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn edwards() {
        // https://www.chessprogramming.org/Perft_Results#Position_5
        perft_assistant(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            &[1, 44, 1_486, 62_379, 2_103_487, 89_941_194],
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn edwards2() {
        // https://www.chessprogramming.org/Perft_Results#Position_6
        perft_assistant(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            &[1, 46, 2_079, 89_890, 3_894_594, 164_075_551],
        );
    }
}
