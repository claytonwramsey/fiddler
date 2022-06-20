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

//! Conversion functions between moves and their algebraic notation.

use crate::movegen::{NoopNominator, ALL};

use super::{
    movegen::{get_moves, is_square_attacked_by},
    Move, Piece, Position,
};

/// Given a `Move` and the `Board` it was played on, construct the
/// algebraic-notation version of the move. Assumes the move was legal.
/// # Panics
/// if the move given is illegal or otherwise invalid.
pub fn algebraic_from_move(m: Move, pos: &Position) -> String {
    //longest possible algebraic string would be something along the lines of
    //Qe4xd4# (7 chars)
    //exd8=Q#
    //O-O-O+
    let mut s = String::with_capacity(7);
    let b = &pos.board;
    assert!(b.is_valid());

    if m.is_castle() {
        if m.to_square().file() > m.from_square().file() {
            //moving right, must be O-O
            s += "O-O";
        } else {
            s += "O-O-O";
        }
    } else {
        let mover_type = b.type_at_square(m.from_square()).unwrap();
        let is_move_capture = b.is_move_capture(m);
        let other_moves = get_moves::<ALL, NoopNominator>(pos)
            .into_iter()
            .map(|x| x.0);
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
                s += from_sq.file_name();
            } else if !is_unclear_file {
                //we can specify the mover by its rank
                s = format!("{}{}", s, from_sq.rank() + 1);
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
    let mut poscopy = *pos;
    let player_color = b.player_to_move;
    let enemy_king_sq = pos.king_sqs[!player_color as usize];
    poscopy.make_move(m, Position::NO_DELTA);
    if is_square_attacked_by(&poscopy.board, enemy_king_sq, player_color) {
        if get_moves::<ALL, NoopNominator>(&poscopy).is_empty() {
            s += "#";
        } else {
            s += "+";
        }
    }

    s
}

/// Given the string of an algebraic-notation move, get the `Move` which can be
/// played. Will return Err if the string is invalid.
pub fn move_from_algebraic(s: &str, pos: &Position) -> Result<Move, &'static str> {
    get_moves::<ALL, NoopNominator>(pos)
        .into_iter()
        .map(|x| x.0)
        .find(|m| algebraic_from_move(*m, pos).as_str() == s)
        .ok_or("not a legal algebraic move")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::square::*;

    #[test]
    /// Test that playing e4 can be successfully converted to its algebraic
    /// form.
    fn test_e4_to_algebraic() {
        let pos = Position::default();
        let m = Move::normal(Square::E2, Square::E4);

        assert_eq!("e4", algebraic_from_move(m, &pos));
    }

    #[test]
    /// Test that a mating move is correctly displayed.
    fn test_mate() {
        // Rb8# is the winning move
        let pos =
            Position::from_fen("3k4/R7/1R6/5K2/8/8/8/8 w - - 0 1", Position::no_eval).unwrap();
        let m = Move::normal(Square::B6, Square::B8);

        assert_eq!("Rb8#", algebraic_from_move(m, &pos));
    }

    #[test]
    /// Test that capturing a pawn is parsed correctly.
    fn test_algebraic_from_pawn_capture() {
        // exf5 is legal here
        let pos = Position::from_fen(
            "rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2",
            Position::no_eval,
        )
        .unwrap();
        let m = Move::normal(Square::E4, Square::F5);
        assert_eq!(algebraic_from_move(m, &pos), "exf5");
    }

    #[test]
    /// Test that the opening move e4 can be converted from a string to a move.
    fn test_move_from_e4() {
        let pos = Position::default();
        let m = Move::normal(Square::E2, Square::E4);
        let s = "e4";

        assert_eq!(move_from_algebraic(s, &pos), Ok(m));
    }

    #[test]
    /// Test that capturing a pawn is parsed correctly.
    fn test_move_from_pawn_capture() {
        let pos = Position::from_fen(
            "rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2",
            Position::no_eval,
        )
        .unwrap();
        let m = Move::normal(Square::E4, Square::F5);
        let s = "exf5";

        assert_eq!(move_from_algebraic(s, &pos), Ok(m));
    }

    #[test]
    /// Test that promotions are displayed correctly.
    fn test_promotion() {
        // f7 pawn can promote
        let pos = Position::from_fen("8/5P2/2k5/4K3/8/8/8/8 w - - 0 1", Position::no_eval).unwrap();
        let m = Move::promoting(Square::F7, Square::F8, Piece::Queen);
        let s = "f8=Q";
        assert_eq!(algebraic_from_move(m, &pos), s);
    }

    #[test]
    /// Test that you get an error out when you give it a bad string.
    fn test_bad_algebraic() {
        let pos = Position::default();
        let s = "garbage";

        assert!(move_from_algebraic(s, &pos).is_err());
    }

    #[test]
    /// Test that algebraic moves are correctly disambiguated by their rank if
    /// needed.
    fn test_rank_identifier() {
        let pos = Position::from_fen(
            "rnbqkbnr/pppppppp/8/8/3P4/1N6/PPP1PPPP/RNBQKB1R w KQkq - 1 5",
            Position::no_eval,
        )
        .unwrap();
        let m = Move::normal(Square::B3, Square::D2);
        let s = "N3d2";
        assert_eq!(algebraic_from_move(m, &pos), s);
        assert_eq!(move_from_algebraic(s, &pos).unwrap(), m);
    }
}
