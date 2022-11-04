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

//! Representation of player colors.

use super::{Bitboard, Direction};
use std::{mem::transmute, ops::Not};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// An enum representing the possible colors that a piece or player can be.
pub enum Color {
    /// The white player, a.k.a. the first player to move in a game.
    White = 0,
    /// The black player, a.k.a. the second player to move in a game.
    Black = 1,
}

impl Color {
    #[inline(always)]
    #[must_use]
    /// Get the direction that a pawn of the given color normally moves.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Color, Direction};
    ///
    /// assert_eq!(Color::White.pawn_direction(), Direction::NORTH);
    /// ```
    pub const fn pawn_direction(self) -> Direction {
        const DIRS: [Direction; 2] = [Direction::NORTH, Direction::SOUTH];
        DIRS[self as usize]
    }

    #[inline(always)]
    #[must_use]
    /// Get the promotion rank of a given color.
    ///
    /// ```
    /// use fiddler::base::{Color, Bitboard};
    ///
    /// assert_eq!(Color::Black.pawn_promote_rank(), Bitboard::new(0xFF));
    /// ```
    pub const fn pawn_promote_rank(self) -> Bitboard {
        const PROMOTE_RANKS: [Bitboard; 2] = [
            Bitboard::new(0xFF00_0000_0000_0000),
            Bitboard::new(0x0000_0000_0000_00FF),
        ];
        PROMOTE_RANKS[self as usize]
    }

    #[inline(always)]
    #[must_use]
    /// Get a `Bitboard` with 1's on the start rank of the pawn of the given
    /// color.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Color, Bitboard};
    ///
    /// assert_eq!(Color::White.pawn_start_rank(), Bitboard::new(0xFF00));
    /// ```
    pub const fn pawn_start_rank(self) -> Bitboard {
        const START_RANKS: [Bitboard; 2] = [
            Bitboard::new(0x0000_0000_0000_FF00),
            Bitboard::new(0x00FF_0000_0000_0000),
        ];
        START_RANKS[self as usize]
    }
}

impl Not for Color {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Color {
        // SAFETY: `self` will always be equal to 0 or 1, so the xor operation
        // will still return a valid color.
        unsafe { transmute(self as u8 ^ 1) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that the opposite color of `Color::White` is `Color::Black`, and
    /// vice versa.
    fn opposite_color() {
        assert_eq!(Color::White, !Color::Black);
        assert_eq!(Color::Black, !Color::White);
    }

    #[test]
    /// Test that the direction for White pawns is north and the direction for
    /// Black pawns is south.
    fn directions() {
        assert_eq!(Color::White.pawn_direction(), Direction::NORTH);
        assert_eq!(Color::Black.pawn_direction(), Direction::SOUTH);
    }

    #[test]
    /// Test that the pawn promotion rank bitboards are correct.
    fn pawn_promote_rank() {
        assert_eq!(
            Bitboard::new(0xFF00_0000_0000_0000),
            Color::White.pawn_promote_rank()
        );
        assert_eq!(
            Bitboard::new(0x0000_0000_0000_00FF),
            Color::Black.pawn_promote_rank()
        );
    }

    #[test]
    /// Test that the start ranks for pawns are correct.
    fn pawn_start_rank() {
        assert_eq!(
            Color::White.pawn_start_rank(),
            Bitboard::new(0x0000_0000_0000_FF00)
        );
        assert_eq!(
            Color::Black.pawn_start_rank(),
            Bitboard::new(0x00FF_0000_0000_0000)
        );
    }
}
