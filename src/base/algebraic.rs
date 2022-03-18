use crate::base::constants;
use crate::base::Board;
use crate::base::Move;
use crate::base::Piece;
use crate::base::Square;

use std::convert::TryFrom;

use super::movegen::get_moves;
use super::movegen::is_square_attacked_by;

/// Given a `Move` and the `Board` it was played on, construct the
/// algebraic-notation version of the move. Assumes the move was legal.
/// # Panics
/// if the move given is illegal or otherwise invalid.
pub fn algebraic_from_move(m: Move, b: &Board) -> String {
    //longest possible algebraic string would be something along the lines of
    //Qe4xd4# (7 chars)
    //exd8=Q#
    //O-O-O+
    let mut s = String::with_capacity(7);

    if b.is_move_castle(m) {
        if m.to_square().file() > m.from_square().file() {
            //moving right, must be O-O
            s += "O-O";
        } else {
            s += "O-O-O";
        }
    } else {
        let mover_type = b.type_at_square(m.from_square()).unwrap();
        let is_move_capture = b.is_move_capture(m);
        let other_moves = get_moves(b);
        let from_sq = m.from_square();

        // Resolution of un-clarity on mover location
        let mut is_unclear = false;
        let mut is_unclear_rank = false;
        let mut is_unclear_file = false;

        // Type of the piece moving
        if mover_type != Piece::Pawn {
            s += mover_type.code();
        } else if is_move_capture {
            is_unclear = true;
            is_unclear_file = true;
        }

        for other_move in other_moves {
            if other_move != m
                && other_move.to_square() == m.to_square()
                && other_move.from_square() != m.from_square()
                && b.type_at_square(other_move.from_square()).unwrap() == mover_type
            {
                is_unclear = true;
                if other_move.from_square().rank() == from_sq.rank() {
                    is_unclear_file = true;
                }
                if other_move.from_square().file() == from_sq.file() {
                    is_unclear_rank = true;
                }
            }
        }

        if is_unclear {
            if !is_unclear_rank {
                //we can specify the mover by its file
                s += constants::FILE_NAMES[from_sq.file()];
            } else if !is_unclear_file {
                //we can specify the mover by its rank
                s += constants::RANK_NAMES[from_sq.rank()];
            } else {
                //we need the complete square to specify the location of the mover
                s += &from_sq.to_string();
            }
        }

        if is_move_capture {
            s += "x";
        }

        s += &m.to_square().to_string();

        // Add promote types
        if let Some(p) = m.promote_type() {
            s += "=";
            s += p.code();
        }
    }

    // Determine if the move was a check or a mate.
    let mut bcopy = *b;
    let player_color = b.player_to_move;
    let enemy_king_sq = Square::try_from(b[Piece::King] & b[!player_color]).unwrap();
    bcopy.make_move(m);
    if is_square_attacked_by(&bcopy, enemy_king_sq, player_color) {
        if get_moves(&bcopy).is_empty() {
            s += "#";
        } else {
            s += "+";
        }
    }

    s
}

/// Given the string of an algebraic-notation move, get the `Move` which can be
/// played. Will return Err if the string is invalid.
pub fn move_from_algebraic(s: &str, b: &Board) -> Result<Move, &'static str> {
    get_moves(b)
        .into_iter()
        .find(|m| algebraic_from_move(*m, b).as_str() == s)
        .ok_or("not a legal algebraic move")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::square::*;
    use crate::fens::*;
    #[test]
    /// Test that playing e4 can be successfully converted to its algebraic
    /// form.
    fn test_e4_to_algebraic() {
        let b = Board::default();
        let m = Move::new(Square::E2, Square::E4, None);

        assert_eq!(String::from("e4"), algebraic_from_move(m, &b));
    }

    #[test]
    /// Test that a mating move is correctly displayed.
    fn test_mate() {
        let b = Board::from_fen(MATE_IN_1_FEN).unwrap();
        let m = Move::new(Square::B6, Square::B8, None);

        assert_eq!(String::from("Rb8#"), algebraic_from_move(m, &b));
    }

    #[test]
    /// Test that capturing a pawn is parsed correctly.
    fn test_algebraic_from_pawn_capture() {
        let b = Board::from_fen(PAWN_CAPTURE_FEN).unwrap();
        let m = Move::new(Square::E4, Square::F5, None);
        let moves = get_moves(&b);
        for m in moves.iter() {
            println!("{m} ");
        }

        assert_eq!(algebraic_from_move(m, &b), String::from("exf5"));
    }

    #[test]
    /// Test that the opening move e4 can be converted from a string to a move.
    fn test_move_from_e4() {
        let b = Board::default();
        let m = Move::new(Square::E2, Square::E4, None);
        let s = "e4";

        assert_eq!(move_from_algebraic(s, &b), Ok(m));
    }

    #[test]
    /// Test that capturing a pawn is parsed correctly.
    fn test_move_from_pawn_capture() {
        let b = Board::from_fen(PAWN_CAPTURE_FEN).unwrap();
        let m = Move::new(Square::E4, Square::F5, None);
        let s = "exf5";
        let moves = get_moves(&b);
        for m in moves.iter() {
            println!("{m} ");
        }

        assert_eq!(move_from_algebraic(s, &b), Ok(m));
    }

    #[test]
    /// Test that promotions are displayed correctly.
    fn test_promotion() {
        let b = Board::from_fen(WHITE_READY_TO_PROMOTE_FEN).unwrap();
        let m = Move::new(Square::F7, Square::F8, Some(Piece::Queen));
        let s = "f8=Q";
        assert_eq!(algebraic_from_move(m, &b), s);
    }

    #[test]
    /// Test that you get an error out when you give it a bad string.
    fn test_bad_algebraic() {
        let b = Board::default();
        let s = "garbage";

        assert!(move_from_algebraic(s, &b).is_err());
    }
}
