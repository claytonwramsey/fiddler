use std::time::Instant;

use super::{movegen::get_moves, Position};

#[allow(dead_code)]
/// Perform a performance test on the move generator and print out facts. The
/// input fen is the FEN of the board to start from, and the depth is the depth
/// from which to generate moves.
pub fn perft(fen: &str, depth: u8) -> u64 {
    let pos = Position::from_fen(fen, Position::no_eval).unwrap();
    let tic = Instant::now();
    let num_nodes = perft_search(&pos, depth);
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
fn perft_search(pos: &Position, depth: u8) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = get_moves(pos);
    let mut total = 0;
    let mut pcopy;
    for m in moves {
        pcopy = *pos;
        pcopy.make_move(m, Position::NO_DELTA);
        total += perft_search(&pcopy, depth - 1);
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fens::BOARD_START_FEN;

    #[test]
    fn test_perft_start_position() {
        perft_assistant(
            BOARD_START_FEN,
            &[1, 20, 400, 8_902, 197_281, 4_865_609, 119_060_324],
        );
    }

    #[test]
    fn test_perft_kiwipete() {
        perft_assistant(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
            &[1, 48, 2039, 97_862, 4_085_603, 193_690_690],
        );
    }

    #[test]
    fn test_perft_unbalanced() {
        perft_assistant(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            &[1, 6, 264, 9_467, 422_333, 15_833_292],
        )
    }

    #[test]
    fn test_perft_edwards() {
        perft_assistant(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            &[1, 44, 1_486, 62_379, 2_103_487, 89_941_194],
        );
    }

    fn perft_assistant(fen: &str, node_counts: &[u64]) {
        for (i, num) in node_counts.iter().enumerate() {
            assert_eq!(*num, perft(fen, i as u8));
        }
    }
}
