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

use super::{game::Game, Piece, Square};

use std::{
    fmt::{Debug, Display, Formatter},
    mem::transmute,
    num::NonZeroU16,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
/// The information of one move, containing its from- and to-squares, as well as its promote type.

// Internally, moves are represented as packed structures in a single unsigned 16-bit integer.
// From MSB to LSB, the bits inside of a `Move` are as follows:
// - 2 bits: flags (promotion, castling, or en passant)
// - 2 bits: promote type
// - 6 bits: destination
// - 6 bits: origin
pub struct Move(NonZeroU16);

impl Move {
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
    pub const fn new(origin: Square, destination: Square) -> Self {
        Self(unsafe { NonZeroU16::new_unchecked(((destination as u16) << 6) | origin as u16) })
    }

    #[must_use]
    /// Create a `Move` with the given promotion type.
    /// The promote type must not be a pawn or a king.
    pub fn promoting(origin: Square, destination: Square, promote_type: Piece) -> Self {
        Self(Self::new(origin, destination).0 | ((promote_type as u16) << 12) | Self::PROMOTE_FLAG)
    }

    #[must_use]
    /// Create a `Move` which is tagged as a castling move.
    pub fn castling(origin: Square, destination: Square) -> Self {
        Self(Self::new(origin, destination).0 | Self::CASTLE_FLAG)
    }

    #[must_use]
    /// Create a `Move` which is tagged as a castling move.
    pub fn en_passant(origin: Square, destination: Square) -> Self {
        Self(Self::new(origin, destination).0 | Self::EN_PASSANT_FLAG)
    }

    #[must_use]
    /// Get the target square of this move.
    pub const fn destination(self) -> Square {
        // Masking out the bottom bits will make this always valid.
        unsafe { transmute(((self.0.get() >> 6) & 63u16) as u8) }
    }

    #[must_use]
    /// Get the square that a piece moves from to execute this move.
    pub const fn origin(self) -> Square {
        // Masking out the bottom bits will make this always valid
        unsafe { transmute((self.0.get() & 63u16) as u8) }
    }

    #[must_use]
    /// Determine whether this move is marked as a promotion.
    pub const fn is_promotion(self) -> bool {
        self.0.get() & Self::FLAG_MASK == Self::PROMOTE_FLAG
    }

    #[must_use]
    /// Determine whether this move is marked as a castle.
    pub const fn is_castle(self) -> bool {
        self.0.get() & Self::FLAG_MASK == Self::CASTLE_FLAG
    }

    #[must_use]
    /// Determine whether this move is marked as an en passant capture.
    pub const fn is_en_passant(self) -> bool {
        self.0.get() & Self::FLAG_MASK == Self::EN_PASSANT_FLAG
    }

    #[must_use]
    /// Get the promotion type of this move.
    /// The resulting type will never be a pawn or a king.
    pub const fn promote_type(self) -> Option<Piece> {
        if self.is_promotion() {
            Some(unsafe { std::mem::transmute::<u8, Piece>(((self.0.get() >> 12) & 3u16) as u8) })
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
    pub fn from_uci(s: &str, game: &Game) -> Result<Self, &'static str> {
        if !(s.len() == 4 || s.len() == 5) {
            return Err("string was neither a normal move or a promotion");
        }
        let orig = Square::from_algebraic(&s[0..2])?;
        let dest = Square::from_algebraic(&s[2..4])?;
        if let Some(charcode) = s.chars().nth(4) {
            let pt = Piece::from_code(charcode.to_ascii_uppercase())
                .ok_or("invalid promote type given")?;
            return Ok(Self::promoting(orig, dest, pt));
        }

        if game.kings().contains(orig) && orig.file_distance(dest) > 1 {
            return Ok(Self::castling(orig, dest));
        }

        if game.pawns().contains(orig) && game.meta().en_passant_square == Some(dest) {
            return Ok(Self::en_passant(orig, dest));
        }

        Ok(Self::new(orig, dest))
    }

    #[must_use]
    /// Construct a UCI string version of this move.
    pub fn to_uci(self) -> String {
        self.promote_type().map_or_else(
            || format!("{}{}", self.origin(), self.destination()),
            |p| {
                format!(
                    "{}{}{}",
                    self.origin(),
                    self.destination(),
                    p.code().to_lowercase()
                )
            },
        )
    }

    #[must_use]
    /// Get a number representing this move uniquely.
    /// The value returned may change from version to version.
    pub const fn value(self) -> u16 {
        self.0.get()
    }

    #[must_use]
    /// Reconstruct a move based on its `value`.
    /// Should only be used with values returned from `Move::value()`.
    ///
    /// # Safety
    ///
    /// This function is only safe if caled on a number generated by `Move::value`.
    pub const unsafe fn from_val(val: u16) -> Self {
        Self(NonZeroU16::new_unchecked(val))
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.origin(), self.destination())?;
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
            None => write!(f, "{} -> {}", self.origin(), self.destination())?,
            Some(p) => write!(f, "{} -> {} ={p}", self.origin(), self.destination())?,
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
        let m = Move::from_uci("e2e4", &Game::default()).unwrap();
        println!("{m}");
        assert_eq!(m, Move::new(Square::E2, Square::E4));
    }

    #[test]
    fn uci_move_promotion() {
        assert_eq!(
            Move::from_uci(
                "b7b8q",
                &Game::from_fen("r1b1kbnr/pPqppppp/2n5/8/8/8/P1PPPPPP/RNBQKBNR w KQkq - 1 5")
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
                &Game::from_fen("1rr3k1/5pp1/3pp2p/p2n3P/1q1P4/1P1Q1N2/5PP1/R1R3K1 b - - 1 26")
                    .unwrap()
            )
            .unwrap(),
            Move::new(Square::C8, Square::C1)
        );
    }

    #[test]
    fn uci_not_castle() {
        assert_eq!(
            Move::from_uci(
                "e1c1",
                &Game::from_fen("1rr3k1/5pp1/3pp2p/p2n3P/1q1P4/1P1Q1N2/5PP1/R3R1K1 w - - 0 26")
                    .unwrap()
            )
            .unwrap(),
            Move::new(Square::E1, Square::C1)
        );
    }
}
