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
use std::{ops::Not, mem::transmute};

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
    /// Get the direction that a pawn of the given color normally moves.
    pub const fn pawn_direction(&self) -> Direction {
        match self {
            Color::White => Direction::NORTH,
            Color::Black => Direction::SOUTH,
        }
    }

    #[inline(always)]
    /// Get the promotion rank of a given color.
    pub const fn pawn_promote_rank(&self) -> Bitboard {
        match self {
            Color::White => Bitboard::new(0xFF00000000000000),
            Color::Black => Bitboard::new(0x00000000000000FF),
        }
    }

    #[inline(always)]
    /// Get a `Bitboard` with 1's on the start rank of the pawn of the given
    /// color.
    pub const fn pawn_start_rank(&self) -> Bitboard {
        match self {
            Color::White => Bitboard::new(0x000000000000FF00),
            Color::Black => Bitboard::new(0x00FF000000000000),
        }
    }
}

impl Not for Color {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Color {
        // self as u8 will always be 0 or 1
        // so self as u8 ^ 1 will always be 1 or 0
        // so we can safely transmute back
        unsafe {transmute(self as u8 ^ 1)}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that the opposite color of `Color::White` is `Color::Black`, and
    /// vice versa.
    fn test_opposite_color() {
        assert_eq!(Color::White, !Color::Black);
        assert_eq!(Color::Black, !Color::White);
    }

    #[test]
    /// Test that the direction for White pawns is north and the direction for
    /// Black pawns is south.
    fn test_directions() {
        assert_eq!(Color::White.pawn_direction(), Direction::NORTH);
        assert_eq!(Color::Black.pawn_direction(), Direction::SOUTH);
    }

    #[test]
    /// Test that the pawn promotion rank bitboards are correct.
    fn test_pawn_promote_rank() {
        assert_eq!(
            Bitboard::new(0xFF00000000000000),
            Color::White.pawn_promote_rank()
        );
        assert_eq!(
            Bitboard::new(0x00000000000000FF),
            Color::Black.pawn_promote_rank()
        );
    }

    #[test]
    /// Test that the start ranks for pawns are correct.
    fn test_pawn_start_rank() {
        assert_eq!(
            Color::White.pawn_start_rank(),
            Bitboard::new(0x000000000000FF00)
        );
        assert_eq!(
            Color::Black.pawn_start_rank(),
            Bitboard::new(0x00FF000000000000)
        );
    }
}
