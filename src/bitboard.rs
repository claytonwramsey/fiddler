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

//! Bitboards, data structures used to efficiently represent sets of squares.

use super::{Square};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
/// A bitboard, which uses an integer to express a set of `Square`s.
/// This expression allows the efficient computation of set intersection, union, disjunction,
/// element selection, and more, all in constant time.
///
/// Nearly all board-related representations use `Bitboard`s as a key part of their construction.
pub struct Bitboard(u64);

impl Bitboard {
    /// A bitboard representing the empty set.
    /// Accordingly, `Bitboard::EMPTY` contains no squares, and functions exactly like the empty set
    /// in all observable behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let sq = Square::A1; // this could be any square
    /// assert!(!Bitboard::EMPTY.contains(sq));
    /// ```
    pub const EMPTY: Bitboard = Bitboard::new(0);

    #[must_use]
    /// Construct a new Bitboard from a numeric literal.
    ///
    /// Internally, `Bitboard`s are 64-bit integers, where the LSB represents whether the square A1
    /// is an element, the second-least bit represents the square A2, and so on.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let bb = Bitboard::EMPTY.with_square(Square::A1);
    ///
    /// assert_eq!(bb, Bitboard::new(1));
    /// ```
    pub const fn new(x: u64) -> Bitboard {
        Bitboard(x)
    }

    #[must_use]
    /// Determine whether this bitboard contains a given square.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// assert!(Bitboard::new(1).contains(Square::A1));
    /// assert!(!(Bitboard::new(2).contains(Square::A1)));
    /// ```
    pub const fn contains(self, square: Square) -> bool {
        self.0 & (1 << square as u8) != 0
    }

    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    /// Compute the number of squares contained in this `Bitboard`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert_eq!(bb.len(), 0);
    /// bb.insert(Square::A1);
    /// assert_eq!(bb.len(), 1);
    /// ```
    pub const fn len(self) -> u8 {
        self.0.count_ones() as u8
    }

    #[must_use]
    /// Convert this bitboard to a u64.
    /// This operation requires no computation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::Bitboard;
    ///
    /// assert_eq!(Bitboard::EMPTY.as_u64(), 0);
    /// assert_eq!(Bitboard::ALL.as_u64(), !0);
    /// ```
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}