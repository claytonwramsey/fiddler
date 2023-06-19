use super::*;

#[test]
/// Test that we can play Qf3+, the critical move in the Fried Liver opening.
fn best_queen_fried_liver() {
    let m = Move::normal(Square::D1, Square::F3);
    // the fried liver position, before Qf3+
    let g = Game::from_fen("r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7").unwrap();
    let moves = make_move_vec::<{ GenMode::All }>(&g);
    assert!(moves.contains(&m));
    for m in moves {
        assert!(is_legal(m, &g));
    }
}

#[test]
/// Test that capturing a pawn is parsed correctly.
fn pawn_capture_generated() {
    // check that exf5 is generated
    let g =
        Game::from_fen("rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2").unwrap();
    let m = Move::normal(Square::E4, Square::F5);
    for m in make_move_vec::<{ GenMode::All }>(&g) {
        println!("{m}");
        assert!(is_legal(m, &g));
    }
    assert!(make_move_vec::<{ GenMode::All }>(&g).contains(&m));
    assert!(make_move_vec::<{ GenMode::Captures }>(&g).contains(&m));
}

#[test]
/// The pawn is checking the king. Is move enumeration correct?
fn enumerate_pawn_checking_king() {
    let g = Game::from_fen("r1bq1b1r/ppp2kpp/2n5/3n4/2B5/8/PPP1pPPP/RN1Q1K1R w - - 0 10").unwrap();

    get_moves::<{ GenMode::All }>(&g, |m| assert!(is_legal(m, &g)));
}

#[test]
/// Check that the king has exactly one move in this position.
fn king_has_only_one_move() {
    let g = Game::from_fen("2k5/4R3/8/5K2/3R4/8/8/8 b - - 2 2").unwrap();
    assert!(has_moves(&g));
    assert!(make_move_vec::<{ GenMode::All }>(&g).len() == 1);
    assert!(is_legal(Move::normal(Square::C8, Square::B8), &g));
}

#[test]
/// Test that queenside castling actually works.
fn queenside_castle() {
    let g = Game::from_fen("r3kb1r/ppp1p1pp/2nq1n2/1B1p4/3P4/2N2Q2/PPP2PPP/R1B1K2R b KQkq - 0 8")
        .unwrap();
    let m = Move::castling(Square::E8, Square::C8);
    assert!(make_move_vec::<{ GenMode::All }>(&g).contains(&m));
    assert!(is_legal(m, &g));
}

#[test]
/// Test that Black cannot castle because there is a knight in the way.
fn no_queenside_castle_through_knight() {
    let g =
        Game::from_fen("rn2kbnr/ppp1pppp/3q4/3p4/6b1/8/PPPPPPPP/RNBQKBNR b KQkq - 5 4").unwrap();
    let m = Move::castling(Square::E8, Square::C8);
    assert!(!make_move_vec::<{ GenMode::All }>(&g).contains(&m));

    assert!(!is_legal(m, &g));
}

#[test]
/// Test that a king can escape check without capturing the checker.
fn king_escape_without_capture() {
    let g =
        Game::from_fen("r2q1b1r/ppp3pp/2n1kn2/4p3/8/2N4Q/PPPP1PPP/R1B1K2R b KQ - 1 10").unwrap();
    let moves = make_move_vec::<{ GenMode::All }>(&g);
    let expected_moves = vec![
        Move::normal(Square::E6, Square::D6),
        Move::normal(Square::E6, Square::F7),
        Move::normal(Square::E6, Square::E7),
        Move::normal(Square::F6, Square::G4),
    ];
    for m in &moves {
        assert!(expected_moves.contains(m));
        assert!(is_legal(*m, &g));
    }
    for em in &expected_moves {
        assert!(moves.contains(em));
        assert!(is_legal(*em, &g));
    }
}

#[test]
/// Test that Black can promote a piece (on e1).
fn black_can_promote() {
    let g = Game::from_fen("8/8/5k2/3K4/8/8/4p3/8 b - - 0 1").unwrap();
    let moves = make_move_vec::<{ GenMode::All }>(&g);
    for m in &moves {
        assert!(is_legal(*m, &g));
    }
    assert!(moves.contains(&Move::promoting(Square::E2, Square::E1, Piece::Queen)));
}

#[test]
/// Test that pawns cannot "wrap around" the side of the board.
fn no_wraparound() {
    let g =
        Game::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBPPP3/q4N2/Pp4PP/R2Q1RK1 b kq - 0 1").unwrap();

    let moves = make_move_vec::<{ GenMode::All }>(&g);
    let m = Move::normal(Square::H7, Square::A7);
    assert!(!(moves.contains(&m)));
    assert!(!is_legal(m, &g));
}

#[test]
/// Test that a move incorrectly flagged as en passant is illegal, even if it is an otherwise
/// normal capture.
fn en_passant_illegal() {
    let g =
        Game::from_fen("r6r/3n1pk1/p4p2/3p4/2p1p1q1/1P2P1P1/P1PP1P1P/R1B1R1K1 b - - 0 25").unwrap();
    let m = Move::en_passant(Square::C4, Square::B3);

    assert!(!is_legal(m, &g));
    assert!(!make_move_vec::<{ GenMode::All }>(&g).contains(&m));
    assert!(!make_move_vec::<{ GenMode::Captures }>(&g).contains(&m));
}

#[test]
/// Test that a pawn cannot capture by en passant if doing so would put the king in check.
fn en_passant_pinned() {
    let g = Game::from_fen("8/2p5/3p4/KPr5/2R1Pp1k/8/6P1/8 b - e3 0 2").unwrap();
    let moves = make_move_vec::<{ GenMode::All }>(&g);
    let m = Move::en_passant(Square::F4, Square::E3);
    assert!(!moves.contains(&m));
    assert!(!is_legal(m, &g));
}

#[test]
/// Test that a move must be tagged as en passant to be considered legal to escape check.
fn en_passant_tagged() {
    let g = Game::from_fen("2B1kb2/pp2pp2/7p/1PpQP3/2nK4/8/P1r4R/R7 w - c6 0 27").unwrap();

    let m = Move::normal(Square::B5, Square::C6);
    assert!(!is_legal(m, &g));
    assert!(!make_move_vec::<{ GenMode::All }>(&g).contains(&m));
}
#[test]
/// Test that a pinned piece cannot make a capture if it does not defend against the pin.
fn pinned_knight_capture() {
    let g =
        Game::from_fen("r2q1b1r/ppp2kpp/2n5/3npb2/2B5/2N5/PPPP1PPP/R1BQ1RK1 b - - 3 8").unwrap();
    let illegal_move = Move::normal(Square::D5, Square::C3);

    assert!(!make_move_vec::<{ GenMode::All }>(&g).contains(&illegal_move));
    assert!(!make_move_vec::<{ GenMode::Captures }>(&g).contains(&illegal_move));
    assert!(!is_legal(illegal_move, &g));
}

#[test]
/// Test that en passant moves are generated correctly.
fn en_passant_generated() {
    // exf6 is en passant
    let g =
        Game::from_fen("rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3").unwrap();

    let m = Move::en_passant(Square::E5, Square::F6);

    assert!(make_move_vec::<{ GenMode::All }>(&g).contains(&m));
    assert!(make_move_vec::<{ GenMode::Captures }>(&g).contains(&m));
    assert!(is_legal(m, &g));
}

#[test]
/// Test that a player can en passant out of check if it results in a checking pawn being captured.
fn en_passant_out_of_check() {
    // bxc6 should be legal here
    let g = Game::from_fen("8/8/8/1Ppp3r/1KR2p1k/8/4P1P1/8 w - c6 0 3").unwrap();

    let m = Move::en_passant(Square::B5, Square::C6);

    assert!(make_move_vec::<{ GenMode::All }>(&g).contains(&m));
    assert!(is_legal(m, &g));
    assert!(has_moves(&g));
}

#[test]
/// Test that the king can actually move (and that `has_moves` reflects that  fact).
fn king_can_move() {
    let g = Game::from_fen("3k4/3R4/1R6/5K2/8/8/8/8 b - - 1 1").unwrap();

    assert!(!make_move_vec::<{ GenMode::All }>(&g).is_empty());
    assert!(!make_move_vec::<{ GenMode::Captures }>(&g).is_empty());
    assert!(!make_move_vec::<{ GenMode::Quiets }>(&g).is_empty());
    assert!(has_moves(&g));
}

#[test]
/// Test that the start position of the game has moves.
fn startpos_has_moves() {
    assert!(has_moves(&Game::default()));
}

/// Tests that mates are correct.
mod mates {
    use super::*;

    /// A helper function for mate move generation testing.
    /// Asserts that `fen` is a position with no legal moves where the player to move is in check.
    fn mate_helper(fen: &str) {
        let g = Game::from_fen(fen).unwrap();

        assert!(!has_moves(&g));
        assert!(make_move_vec::<{ GenMode::All }>(&g).is_empty());
        assert!(make_move_vec::<{ GenMode::Captures }>(&g).is_empty());
        assert!(make_move_vec::<{ GenMode::Quiets }>(&g).is_empty());
        assert!(!g.meta().checkers.is_empty());
    }

    #[test]
    /// Test that a ladder-mated position has no legal moves.
    fn ladder() {
        mate_helper("1R1k4/R7/8/5K2/8/8/8/8 b - - 1 1");
    }

    #[test]
    /// A position where if pawn pushes could be captures, there would not be a mate.
    fn cant_pawn_push() {
        mate_helper("2r2r2/5R2/p2p2pk/3P2Q1/P4n2/7P/1P6/1K4R1 b - - 2 34");
    }

    #[test]
    /// Test that a position where a rook is horizontal to the king is mate.
    fn horizontal_rook() {
        mate_helper("r1b2k1R/3n1p2/p7/3P4/6Qp/2P3b1/6P1/4R2K b - - 0 32");
    }

    #[test]
    /// A mate where the queen is adjacent to the king, and cuts off all escape.
    fn queen_defended() {
        mate_helper("r1b2b1r/ppp2kpp/8/4p3/3n4/2Q5/PP1PqPPP/RNB1K2R w KQ - 4 11");
    }

    #[test]
    fn pinned_horiz_pawn() {
        mate_helper("5r2/3R1pk1/p5R1/7Q/r3p3/7P/8/2K5 b - - 0 37");
    }
}

mod perft {
    use std::time::Instant;

    use super::*;

    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::similar_names)]
    /// Perform a performance test on the move generator.
    /// Returns the number of independent paths to a leaf reachable in `depth` plies from a board
    /// with starting position `fen`.
    fn perft(fen: &str, depth: u8) -> u64 {
        /// The core search algorithm for perft.
        fn helper<const DIVIDE: bool>(g: &mut Game, depth: u8) -> u64 {
            let mut total = 0;
            if depth == 1 {
                get_moves::<{ GenMode::All }>(g, |_| total += 1);
            } else {
                // to prevent a violation of Rust's aliasing rules, we can't use a callback here.
                // instead, we can just collect the moves into a vector.
                for m in make_move_vec::<{ GenMode::All }>(g) {
                    g.make_move(m);
                    let count = helper::<false>(g, depth - 1);
                    if DIVIDE {
                        println!("{m:?}: {count}");
                    }
                    g.undo().unwrap();
                    total += count;
                }
            };

            total
        }

        let mut g = Game::from_fen(fen).unwrap();
        let tic = Instant::now();
        let num_nodes = if depth == 0 {
            1
        } else {
            helper::<true>(&mut g, depth)
        };
        let toc = Instant::now();
        let time = toc - tic;
        let speed = (num_nodes as f64) / time.as_secs_f64();
        println!(
            "time {:.2} secs, num nodes {num_nodes}: {speed:.0} nodes/sec",
            time.as_secs_f64()
        );

        num_nodes
    }

    #[allow(clippy::cast_possible_truncation)]
    fn perft_assistant(fen: &str, node_counts: &[u64]) {
        for (i, num) in node_counts.iter().enumerate() {
            assert_eq!(*num, perft(fen, i as u8));
        }
    }

    #[test]
    /// Test the perft values for the board starting position.
    fn start_position() {
        perft_assistant(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &[1, 20, 400, 8_902, 197_281, 4_865_609, 119_060_324],
        );
    }

    #[test]
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
    fn endgame() {
        // https://www.chessprogramming.org/Perft_Results#Position_3
        perft_assistant(
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            &[1, 14, 191, 2_812, 43_238, 674_624, 11_030_083, 178_633_661],
        );
    }

    #[test]
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
    fn edwards() {
        // https://www.chessprogramming.org/Perft_Results#Position_5
        perft_assistant(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            &[1, 44, 1_486, 62_379, 2_103_487, 89_941_194],
        );
    }

    #[test]
    fn edwards2() {
        // https://www.chessprogramming.org/Perft_Results#Position_6
        perft_assistant(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            &[1, 46, 2_079, 89_890, 3_894_594, 164_075_551],
        );
    }
}

mod sliding {
    use super::*;

    #[test]
    fn rook_mask() {
        // println!("{:064b}", get_rook_mask(A1).0);
        assert_eq!(
            get_rook_mask(Square::A1),
            Bitboard::new(0x0001_0101_0101_017E)
        );

        // println!("{:064b}", get_rook_mask(E1).0);
        assert_eq!(
            get_rook_mask(Square::E1),
            Bitboard::new(0x0010_1010_1010_106E)
        );

        // println!("{:064b}", get_rook_mask(E5).0);
        assert_eq!(
            get_rook_mask(Square::E5),
            Bitboard::new(0x0010_106E_1010_1000)
        );
    }

    #[test]
    fn bishop_mask() {
        // println!("{:064b}", get_bishop_mask(A1).0);
        assert_eq!(
            get_bishop_mask(Square::A1),
            Bitboard::new(0x0040_2010_0804_0200)
        );

        // println!("{:064b}", get_bishop_mask(E1).0);
        assert_eq!(
            get_bishop_mask(Square::E1),
            Bitboard::new(0x0000_0000_0244_2800)
        );

        // println!("{:064b}", get_bishop_mask(E5).0);
        assert_eq!(
            get_bishop_mask(Square::E5),
            Bitboard::new(0x0044_2800_2844_0200)
        );
    }

    #[test]
    fn valid_index_to_occupancy() {
        let mask = Bitboard::new(0b1111);
        for i in 0..16 {
            let occu = index_to_occupancy(i, mask);
            assert_eq!(occu, Bitboard::new(i as u64));
        }
    }

    #[test]
    /// Test that bishop magic move generation is correct for every possible occupancy bitboard.
    fn all_bishop_attacks() {
        for sq in Bitboard::ALL {
            let mask = get_bishop_mask(sq);
            for i in 0..(1 << mask.len()) {
                let occupancy = index_to_occupancy(i, mask);
                let attacks = directional_attacks(sq, &Direction::BISHOP_DIRECTIONS, occupancy);
                assert_eq!(attacks, bishop_moves(occupancy, sq));
            }
        }
    }

    #[test]
    /// Test that rook magic move generation is correct for every possible occupancy bitboard.
    fn all_rook_attacks() {
        for sq in Bitboard::ALL {
            let mask = get_rook_mask(sq);
            for i in 0..(1 << mask.len()) {
                let occupancy = index_to_occupancy(i, mask);
                let attacks = directional_attacks(sq, &Direction::ROOK_DIRECTIONS, occupancy);
                assert_eq!(attacks, rook_moves(occupancy, sq));
            }
        }
    }
}
