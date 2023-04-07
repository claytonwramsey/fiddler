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

//! Definitions of moves, which can describe any legal playable move.

use super::{
    game::{Board, Game},
    movegen::{get_moves, has_moves, is_legal, is_square_attacked_by, GenMode},
};

use super::{Piece, Square};

use std::{
    fmt::{Debug, Display, Formatter},
    mem::transmute,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
/// The information of one move, containing its from- and to-squares, as well as its promote type.

// Internally, moves are represented as packed structures in a single unsigned 16-bit integer.
// From MSB to LSB, the bits inside of a `Move` are as follows:
// * 2 bits: flags (promotion, castling, or en passant)
// * 2 bits: promote type
// * 6 bits: from-square
// * 6 bits: to-square
pub struct Move(u16);

impl Move {
    /// A sentinel value for a move which is illegal, or otherwise inexpressible.
    ///
    /// It is *strongly* recommended that `Option<Move>` is used instead of this whenever space is
    /// not an enormous concern.
    pub const BAD_MOVE: Move = Move(0xFFFF);

    /// The mask used to extract the flag bits from a move.
    const FLAG_MASK: u16 = 0xC000;

    /// The flag bits representing a move which is promotion.
    const PROMOTE_FLAG: u16 = 0x4000;

    /// The flag bits representing a move which is a castle.
    const CASTLE_FLAG: u16 = 0x8000;

    /// The flag bits representing a move which is en passant.
    const EN_PASSANT_FLAG: u16 = 0xC000;

    #[must_use]
    /// Create a `Move` with no promotion type, which is not marked as having any extra special
    /// flags.
    pub const fn normal(from_square: Square, to_square: Square) -> Move {
        Move(((to_square as u16) << 6) | from_square as u16)
    }

    #[must_use]
    /// Create a `Move` with the given promotion type.
    /// The promote type must not be a pawn or a king.
    pub const fn promoting(from_square: Square, to_square: Square, promote_type: Piece) -> Move {
        Move(
            Move::normal(from_square, to_square).0
                | ((promote_type as u16) << 12)
                | Move::PROMOTE_FLAG,
        )
    }

    #[must_use]
    /// Create a `Move` which is tagged as a castling move.
    pub const fn castling(from_square: Square, to_square: Square) -> Move {
        Move(Move::normal(from_square, to_square).0 | Move::CASTLE_FLAG)
    }

    #[must_use]
    /// Create a `Move` which is tagged as a castling move.
    pub const fn en_passant(from_square: Square, to_square: Square) -> Move {
        Move(Move::normal(from_square, to_square).0 | Move::EN_PASSANT_FLAG)
    }

    #[must_use]
    /// Get the target square of this move.
    pub const fn to_square(self) -> Square {
        // Masking out the bottom bits will make this always valid.
        unsafe { transmute(((self.0 >> 6) & 63u16) as u8) }
    }

    #[must_use]
    /// Get the square that a piece moves from to execute this move.
    pub const fn from_square(self) -> Square {
        // Masking out the bottom bits will make this always valid
        unsafe { transmute((self.0 & 63u16) as u8) }
    }

    #[must_use]
    /// Determine whether this move is marked as a promotion.
    pub const fn is_promotion(self) -> bool {
        self.0 & Move::FLAG_MASK == Move::PROMOTE_FLAG
    }

    #[must_use]
    /// Determine whether this move is marked as a castle.
    pub const fn is_castle(self) -> bool {
        self.0 & Move::FLAG_MASK == Move::CASTLE_FLAG
    }

    #[must_use]
    /// Determine whether this move is marked as an en passant capture.
    pub const fn is_en_passant(self) -> bool {
        self.0 & Move::FLAG_MASK == Move::EN_PASSANT_FLAG
    }

    #[must_use]
    /// Get the promotion type of this move.
    /// The resulting type will never be a pawn or a king.
    pub const fn promote_type(self) -> Option<Piece> {
        if self.is_promotion() {
            Some(unsafe { std::mem::transmute(((self.0 >> 12) & 3u16) as u8) })
        } else {
            None
        }
    }

    /// Convert a move from its UCI representation.
    /// Requires the game the move was played on to determine extra flags about the move.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if `s` describes an illegal UCI move.
    pub fn from_uci(s: &str, game: &Game) -> Result<Move, &'static str> {
        if !(s.len() == 4 || s.len() == 5) {
            return Err("string was neither a normal move or a promotion");
        }
        let from_sq = Square::from_algebraic(&s[0..2])?;
        let to_sq = Square::from_algebraic(&s[2..4])?;
        if let Some(charcode) = s.chars().nth(4) {
            let pt = Piece::from_code(charcode.to_ascii_uppercase())
                .ok_or("invalid promote type given")?;
            return Ok(Move::promoting(from_sq, to_sq, pt));
        }

        if game.board()[Piece::King].contains(from_sq) && from_sq.file_distance(to_sq) > 1 {
            return Ok(Move::castling(from_sq, to_sq));
        }

        if game.board()[Piece::Pawn].contains(from_sq)
            && game.meta().en_passant_square == Some(to_sq)
        {
            return Ok(Move::en_passant(from_sq, to_sq));
        }

        Ok(Move::normal(from_sq, to_sq))
    }

    #[must_use]
    /// Construct a UCI string version of this move.
    pub fn to_uci(self) -> String {
        match self.promote_type() {
            None => format!("{}{}", self.from_square(), self.to_square()),
            Some(p) => format!(
                "{}{}{}",
                self.from_square(),
                self.to_square(),
                p.code().to_lowercase()
            ),
        }
    }

    #[allow(clippy::missing_panics_doc)]
    /// Given a [`Move`] and the [`Game`] it was played on, construct the algebraic-notation
    /// version of the move.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if the move is illegal on the given
    /// board.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use fiddler::base::{Board, Move, Square};
    ///
    /// let b = Board::default();
    /// let m = Move::normal(Square::E2, Square::E4);
    /// assert_eq!(m.to_algebraic(&b)?, "e4");
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_algebraic(self, g: &Game) -> Result<String, &'static str> {
        // longest possible algebraic string would be something along the lines of Qe4xd4#, exd8=Q#,
        // and O-O-O+
        let mut s = String::with_capacity(7);
        if !is_legal(self, b) {
            // can't make an algebraic form of an illegal move
            return Err("illegal move converted to algebraic");
        }

        if self.is_castle() {
            if self.to_square().file() > self.from_square().file() {
                // moving right, must be O-O
                s += "O-O";
            } else {
                s += "O-O-O";
            }
        } else {
            let mover_type = b.(self.from_square()).unwrap();
            let is_move_capture = b.is_move_capture(self);
            let from_sq = self.from_square();

            // Resolution of un-clarity on mover location
            let mut is_unclear = false;
            let mut is_unclear_rank = false;
            let mut is_unclear_file = false;

            // Type of the piece moving
            if mover_type != Piece::Pawn {
                s.push(mover_type.code());
            } else if is_move_capture {
                is_unclear = true;
                is_unclear_file = true;
            }

            get_moves::<{ GenMode::All }>(b, |other_move| {
                if self != other_move
                    && other_move.to_square() == self.to_square()
                    && other_move.from_square() != self.from_square()
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
            });

            if is_unclear {
                if !is_unclear_rank {
                    // we can specify the mover by its file
                    s.push(from_sq.file_name());
                } else if !is_unclear_file {
                    // we can specify the mover by its rank
                    s = format!("{s}{}", from_sq.rank() + 1);
                } else {
                    // we need the complete square to specify the location of the mover
                    s += &from_sq.to_string();
                }
            }

            if is_move_capture {
                s += "x";
            }

            s += &self.to_square().to_string();

            // Add promote types
            if let Some(p) = self.promote_type() {
                s += "=";
                s.push(p.code());
            }
        }

        // Determine if the move was a check or a mate.
        let mut bcopy = *b;
        let enemy_king_sq = b.king_sqs[!b.player as usize];
        bcopy.make_move(self);
        if is_square_attacked_by(&bcopy, enemy_king_sq, b.player) {
            if has_moves(&bcopy) {
                s += "+";
            } else {
                s += "#";
            }
        }

        Ok(s)
    }

    #[allow(clippy::missing_panics_doc)]
    /// Given the string of an algebraic-notation move, get the `Move` which can be played.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if `s` is not a valid algebraically-represented move in
    /// `b`.
    pub fn from_algebraic(s: &str, b: &Board) -> Result<Move, &'static str> {
        let mut result = Err("not a legal algebraic move");
        get_moves::<{ GenMode::All }>(b, |m| {
            if m.to_algebraic(b).unwrap() == s {
                result = Ok(m);
            }
        });
        result
    }

    #[must_use]
    /// Get a number representing this move uniquely.
    /// The value returned may change from version to version.
    pub const fn value(self) -> u16 {
        self.0
    }

    #[must_use]
    /// Reconstruct a move based on its `value`.
    /// Should only be used with values returned from `Move::value()`.
    pub const fn from_val(val: u16) -> Move {
        Move(val)
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from_square(), self.to_square())?;
        if let Some(pt) = self.promote_type() {
            write!(f, "{}", pt.code())?;
        }
        if self.is_en_passant() {
            write!(f, " [e.p.]")?;
        }
        if self.is_castle() {
            write!(f, " [castle]")?;
        }
        Ok(())
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.promote_type() {
            None => write!(f, "{} -> {}", self.from_square(), self.to_square())?,
            Some(p) => write!(f, "{} -> {} ={p}", self.from_square(), self.to_square())?,
        };
        if self.is_en_passant() {
            write!(f, " [e.p.]")?;
        }
        if self.is_castle() {
            write!(f, " [castle]")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uci_move_normal() {
        let b = Board::default();
        let m = Move::from_uci("e2e4", &b).unwrap();
        println!("{m}");
        assert_eq!(m, Move::normal(Square::E2, Square::E4));
    }

    #[test]
    fn uci_move_promotion() {
        assert_eq!(
            Move::from_uci(
                "b7b8q",
                &Board::from_fen("r1b1kbnr/pPqppppp/2n5/8/8/8/P1PPPPPP/RNBQKBNR w KQkq - 1 5")
                    .unwrap(),
            )
            .unwrap(),
            Move::promoting(Square::B7, Square::B8, Piece::Queen),
        );
    }

    #[test]
    fn uci_move_capture() {
        assert_eq!(
            Move::from_uci(
                "c8c1",
                &Board::from_fen("1rr3k1/5pp1/3pp2p/p2n3P/1q1P4/1P1Q1N2/5PP1/R1R3K1 b - - 1 26")
                    .unwrap()
            )
            .unwrap(),
            Move::normal(Square::C8, Square::C1)
        );
    }

    #[test]
    fn uci_not_castle() {
        assert_eq!(
            Move::from_uci(
                "e1c1",
                &Board::from_fen("1rr3k1/5pp1/3pp2p/p2n3P/1q1P4/1P1Q1N2/5PP1/R3R1K1 w - - 0 26")
                    .unwrap()
            )
            .unwrap(),
            Move::normal(Square::E1, Square::C1)
        );
    }

    #[test]
    /// Test that playing e4 can be successfully converted to its algebraic form.
    fn e4_to_algebraic() {
        let b = Board::default();
        let m = Move::normal(Square::E2, Square::E4);

        assert_eq!("e4", m.to_algebraic(&b).unwrap());
    }

    #[test]
    /// Test that a mating move is correctly displayed.
    fn algebraic_mate() {
        // Rb8# is the winning move
        let b = Board::from_fen("3k4/R7/1R6/5K2/8/8/8/8 w - - 0 1").unwrap();
        let m = Move::normal(Square::B6, Square::B8);

        assert_eq!("Rb8#", m.to_algebraic(&b).unwrap());
    }

    #[test]
    /// Test that capturing a pawn is parsed correctly.
    fn algebraic_from_pawn_capture() {
        // exf5 is legal here
        let b = Board::from_fen("rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2")
            .unwrap();
        let m = Move::normal(Square::E4, Square::F5);
        assert_eq!(m.to_algebraic(&b).unwrap(), "exf5");
    }

    #[test]
    /// Test that the opening move e4 can be converted from a string to a move.
    fn algebraic_move_from_e4() {
        let b = Board::default();
        let m = Move::normal(Square::E2, Square::E4);
        let s = "e4";

        assert_eq!(Move::from_algebraic(s, &b), Ok(m));
    }

    #[test]
    /// Test that capturing a pawn is parsed correctly.
    fn algebraic_move_from_pawn_capture() {
        let b = Board::from_fen("rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2")
            .unwrap();
        let m = Move::normal(Square::E4, Square::F5);
        let s = "exf5";

        assert_eq!(Move::from_algebraic(s, &b), Ok(m));
    }

    #[test]
    /// Test that promotions are displayed correctly.
    fn algebraic_promotion() {
        // f7 pawn can promote
        let b = Board::from_fen("8/5P2/2k5/4K3/8/8/8/8 w - - 0 1").unwrap();
        let m = Move::promoting(Square::F7, Square::F8, Piece::Queen);
        let s = "f8=Q";
        assert_eq!(m.to_algebraic(&b).unwrap(), s);
    }

    #[test]
    /// Test that you get an error out when you give it a bad string.
    fn bad_algebraic() {
        let b = Board::default();
        let s = "garbage";

        assert!(Move::from_algebraic(s, &b).is_err());
    }

    #[test]
    /// Test that algebraic moves are correctly disambiguated by their rank if needed.
    fn algebraic_rank_identifier() {
        let b = Board::from_fen("rnbqkbnr/pppppppp/8/8/3P4/1N6/PPP1PPPP/RNBQKB1R w KQkq - 1 5")
            .unwrap();
        let m = Move::normal(Square::B3, Square::D2);
        let s = "N3d2";
        assert_eq!(m.to_algebraic(&b).unwrap(), s);
        assert_eq!(Move::from_algebraic(s, &b).unwrap(), m);
    }
}
