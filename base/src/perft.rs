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

//! Performance testing, or "perft." Perft is used for verifying the correctness
//! of move generation and benchmarking the speed of both move generation and
//! the make/unmake processes.

use std::time::Instant;

use crate::{movegen::{NoopNominator, ALL}, Score};

use super::{movegen::get_moves, Position};

#[allow(dead_code)]
/// Perform a performance test on the move generator and print out facts. The
/// input fen is the FEN of the board to start from, and the depth is the depth
/// from which to generate moves.
///
/// # Panics
///
/// This function will panic if `fen` is not a legal board.
pub fn perft(fen: &str, depth: u8) -> u64 {
    let pos = Position::from_fen(fen, Position::no_eval).unwrap();
    let tic = Instant::now();
    let num_nodes = perft_search(&pos, depth, true);
    let toc = Instant::now();
    let time = toc - tic;
    let speed = (num_nodes as f64) / time.as_secs_f64();
    println!(
        "time {:.2} secs, num nodes {num_nodes}: {speed:.0} nodes/sec",
        time.as_secs_f64()
    );

    num_nodes
}

/// The core search algorithm for perft.
fn perft_search(pos: &Position, depth: u8, divide: bool) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = get_moves::<ALL, NoopNominator>(pos);
    let mut total = 0;
    let mut pcopy;
    for m in moves {
        pcopy = *pos;
        pcopy.make_move(m.0, Score::DRAW);
        let perft_count = perft_search(&pcopy, depth - 1, false);
        if divide {
            println!("{}, {perft_count}", m.0);
        }
        total += perft_count;
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test the perft values for the board starting position.
    fn perft_start_position() {
        perft_assistant(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &[1, 20, 400, 8_902, 197_281, 4_865_609, 119_060_324],
        );
    }

    #[test]
    /// Test the perft values for the
    /// [Kiwipete](https://www.chessprogramming.org/Perft_Results#Position_2)
    /// position.
    fn perft_kiwipete() {
        perft_assistant(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
            &[1, 48, 2039, 97_862, 4_085_603, 193_690_690],
        );
    }

    #[test]
    fn perft_endgame() {
        // https://www.chessprogramming.org/Perft_Results#Position_3
        perft_assistant(
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - ",
            &[1, 14, 191, 2_812, 43_238, 674_624, 11_030_083, 178_633_661],
        );
    }

    #[test]
    /// Test the perft values for an unbalanced position. Uses results from
    /// [the CPW wiki](https://www.chessprogramming.org/Perft_Results#Position_4).
    fn perft_unbalanced() {
        perft_assistant(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            &[1, 6, 264, 9_467, 422_333, 15_833_292],
        )
    }

    #[test]
    fn perft_edwards() {
        // https://www.chessprogramming.org/Perft_Results#Position_5
        perft_assistant(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            &[1, 44, 1_486, 62_379, 2_103_487, 89_941_194],
        );
    }

    #[test]
    fn perft_edwards2() {
        // https://www.chessprogramming.org/Perft_Results#Position_6
        perft_assistant(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            &[1, 46, 2_079, 89_890, 3_894_594, 164_075_551],
        );
    }

    fn perft_assistant(fen: &str, node_counts: &[u64]) {
        for (i, num) in node_counts.iter().enumerate() {
            assert_eq!(*num, perft(fen, i as u8));
        }
    }
}
