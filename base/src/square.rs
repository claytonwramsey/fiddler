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

//! Squares, which are positions on a board.

use super::{Bitboard, Direction};

use std::{
    cmp::max,
    convert::TryFrom,
    fmt::{Display, Formatter},
    mem::transmute,
    ops::{Add, AddAssign, Sub},
};

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
/// A square: one of 64 spots on a `Board` that a `Piece` can occupy.
///
/// Internally, `Square`s are represented as a single integer to maintain a
/// small size.From MSB to LSB, each square is composed of:
/// * 2 unused bits
/// * 3 bits for the rank
/// * 3 bits for the file
pub enum Square {
    A1 = 0,
    B1 = 1,
    C1 = 2,
    D1 = 3,
    E1 = 4,
    F1 = 5,
    G1 = 6,
    H1 = 7,
    A2 = 8,
    B2 = 9,
    C2 = 10,
    D2 = 11,
    E2 = 12,
    F2 = 13,
    G2 = 14,
    H2 = 15,
    A3 = 16,
    B3 = 17,
    C3 = 18,
    D3 = 19,
    E3 = 20,
    F3 = 21,
    G3 = 22,
    H3 = 23,
    A4 = 24,
    B4 = 25,
    C4 = 26,
    D4 = 27,
    E4 = 28,
    F4 = 29,
    G4 = 30,
    H4 = 31,
    A5 = 32,
    B5 = 33,
    C5 = 34,
    D5 = 35,
    E5 = 36,
    F5 = 37,
    G5 = 38,
    H5 = 39,
    A6 = 40,
    B6 = 41,
    C6 = 42,
    D6 = 43,
    E6 = 44,
    F6 = 45,
    G6 = 46,
    H6 = 47,
    A7 = 48,
    B7 = 49,
    C7 = 50,
    D7 = 51,
    E7 = 52,
    F7 = 53,
    G7 = 54,
    H7 = 55,
    A8 = 56,
    B8 = 57,
    C8 = 58,
    D8 = 59,
    E8 = 60,
    F8 = 61,
    G8 = 62,
    H8 = 63,
}

impl Square {
    #[inline(always)]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    /// Create a Square from the given rank and file. The ranks run from 0 to 7
    /// (instead of 1 through 8), and the files run from A to H.
    pub fn new(rank: usize, file: usize) -> Option<Square> {
        Square::try_from(((rank << 3) | file) as u8).ok()
    }

    #[inline(always)]
    #[must_use]
    /// Get the integer representing the rank (0 -> 1, ...) of this square.
    pub const fn rank(self) -> usize {
        (self as u8 >> 3u8) as usize
    }

    #[inline(always)]
    #[must_use]
    /// Get the integer representing the file (0 -> A, ...) of this square.
    pub const fn file(self) -> usize {
        (self as u8 & 7u8) as usize
    }

    #[inline(always)]
    #[must_use]
    /// Get the Chebyshev distance to another square.
    pub fn chebyshev_to(&self, rhs: Square) -> u8 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            clippy::cast_sign_loss
        )]
        {
            let rankdiff = ((rhs.rank() as i8) - (self.rank() as i8)).abs();
            let filediff = ((rhs.file() as i8) - (self.file() as i8)).abs();

            max(rankdiff, filediff) as u8
        }
    }

    #[inline(always)]
    #[must_use]
    /// Get what this square would appear to be from the point of view of the
    /// opposing player.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::Square;
    /// let sq1 = Square::A1;
    /// let sq2 = sq1.opposite();
    /// assert_eq!(sq2, Square::A8);
    /// ```
    pub fn opposite(&self) -> Square {
        unsafe { transmute(*self as u8 ^ 56) }
    }

    /// Convert an algebraic string (such as 'e7') to a square.
    /// To get an `Ok` result, the string must be two characters.
    /// The file must be in lowercase.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if `s` is not a legal algebraic
    /// square.
    ///
    /// # Panics
    ///
    /// This function will panic in the case of an internal error.
    pub fn from_algebraic(s: &str) -> Result<Square, &'static str> {
        if s.len() != 2 {
            return Err("square name must be 2 characters");
        }
        let mut chars = s.chars();
        let (file, _) = match "abcdefgh".match_indices(chars.next().unwrap()).next() {
            Some(d) => d,
            None => return Err("illegal file for square"),
        };
        let rank = match chars.next().unwrap().to_digit(10) {
            Some(n) => n as usize,
            None => return Err("expected number for square rank"),
        };
        // will not fail because we have already validated the rank and file
        Ok(Square::new(rank - 1, file).unwrap())
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    /// Unsafely convert a `Bitboard` to a `Square` by creating the square
    /// representing its lowest occupied bit. Will result in undefined behavior
    /// (most likely a `Square` whose enum value is not in 0..64) if the given
    /// board is empty.
    ///
    /// # Safety
    ///
    /// This function will behave safely if `bb` is nonzero. It will result in
    /// undefined behavior if `bb` is equal to `Bitboard::EMPTY`.
    pub unsafe fn unsafe_from(bb: Bitboard) -> Square {
        transmute(bb.trailing_zeros() as u8)
    }

    #[must_use]
    /// Get the name of the file of this square. For instance, the square
    /// representing A1 will have the name "a".
    pub fn file_name(&self) -> &str {
        match self.file() {
            0 => "a",
            1 => "b",
            2 => "c",
            3 => "d",
            4 => "e",
            5 => "f",
            6 => "g",
            7 => "h",
            // files are only from 0..8
            _ => unreachable!(),
        }
    }
}

impl Add<Direction> for Square {
    type Output = Square;
    #[inline(always)]
    #[allow(clippy::cast_sign_loss)]
    fn add(self, rhs: Direction) -> Self::Output {
        // Apply the modulo to prevent UB.
        unsafe { transmute(((self as i8) + rhs.0) as u8 & 63) }
    }
}

impl AddAssign<Direction> for Square {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Direction) {
        *self = *self + rhs;
    }
}

impl Display for Square {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.file_name(), self.rank() + 1)
    }
}

impl Sub<Square> for Square {
    type Output = Direction;
    #[inline(always)]
    fn sub(self, rhs: Square) -> Self::Output {
        Direction((self as i8) - (rhs as i8))
    }
}

impl Sub<Direction> for Square {
    type Output = Square;
    #[inline(always)]
    #[allow(clippy::cast_sign_loss)]
    fn sub(self, rhs: Direction) -> Self::Output {
        Square::try_from(((self as i8) - (rhs.0)) as u8 & 63u8).unwrap()
    }
}

impl TryFrom<Bitboard> for Square {
    type Error = &'static str;

    /// Create the square closest to A1 (prioritizing rank) on the given
    /// bitboard.
    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    fn try_from(bb: Bitboard) -> Result<Square, Self::Error> {
        Square::try_from(bb.trailing_zeros() as u8)
    }
}

impl TryFrom<u8> for Square {
    type Error = &'static str;
    #[inline(always)]
    fn try_from(x: u8) -> Result<Square, Self::Error> {
        match x {
            // This transmutation is safe because i will always be less than
            // the total number of squares.
            x if x <= Square::H8 as u8 => Ok(unsafe { transmute(x) }),
            _ => Err("input for square conversion is out of bounds"),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::Direction;

    #[test]
    fn add_square_and_direction() {
        assert_eq!(Square::A1 + Direction::EAST, Square::B1);
        assert_eq!(Square::A1 + Direction::NORTHEAST, Square::B2);
    }

    #[test]
    fn add_direction_and_square() {
        assert_eq!(Direction::EAST + Square::A1, Square::B1);
    }

    #[test]
    fn square_from_algebraic() {
        assert_eq!(Square::from_algebraic("e4"), Ok(Square::E4));
        assert_eq!(Square::from_algebraic("f7"), Ok(Square::F7));
    }
}
