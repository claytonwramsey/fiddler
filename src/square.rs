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

//! Squares, which are positions on a board.

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[allow(unused)]
/// A square: one of 64 spots on a `Board` that a `Piece` can occupy.
//  Internally, `Square`s are represented as a single integer to maintain a small size.
//  From MSB to LSB, each square is composed of:
//  * 2 unused bits
//  * 3 bits for the rank
//  * 3 bits for the file
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
    #[must_use]
    /// Get the integer representing the rank (0 -> 1, ...) of this square.
    pub const fn rank(self) -> u8 {
        self as u8 >> 3u8
    }

    #[must_use]
    /// Get the integer representing the file (0 -> A, ...) of this square.
    pub const fn file(self) -> u8 {
        self as u8 & 7u8
    }

    #[must_use]
    /// Get the Chebyshev distance to another square.
    pub const fn chebyshev_to(self, rhs: Square) -> u8 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        {
            let rankdiff = ((rhs.rank() as i8) - (self.rank() as i8)).unsigned_abs();
            let filediff = self.file_distance(rhs);
            // we would use `max()` here if it were a const function
            if rankdiff > filediff {
                rankdiff
            } else {
                filediff
            }
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    /// Get the distance between two `Square`s by traveling along files.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::Square;
    ///
    /// let sq1 = Square::A8;
    /// let sq2 = Square::C1;
    ///
    /// assert_eq!(sq1.file_distance(sq2), 2);
    /// ```
    pub const fn file_distance(self, rhs: Square) -> u8 {
        ((rhs.file() as i8) - (self.file() as i8)).unsigned_abs()
    }
}