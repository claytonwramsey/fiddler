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

//! Bitboards, data structures used to efficiently represent sets of squares.

use once_cell::sync::Lazy;

use crate::MAGIC;

use super::Square;

use std::{
    fmt::{Display, Formatter, Result},
    iter::Iterator,
    mem::transmute,
    ops::{
        AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Mul, Not, Shl, ShlAssign, Shr,
    },
};

/// A lookup table for the squares on a line between any two squares,
/// either down a row like a rook or diagonal like a bishop.
static LINES: Lazy<[[Bitboard; 64]; 64]> = Lazy::new(|| {
    let mut lines = [[Bitboard::EMPTY; 64]; 64];

    for sq1 in Bitboard::ALL {
        let bishop_1 = MAGIC.bishop_attacks(Bitboard::EMPTY, sq1);
        let rook_1 = MAGIC.rook_attacks(Bitboard::EMPTY, sq1);
        for sq2 in Bitboard::ALL {
            if bishop_1.contains(sq2) {
                let bishop_2 = MAGIC.bishop_attacks(Bitboard::EMPTY, sq2);
                lines[sq1 as usize][sq2 as usize] |= Bitboard::from(sq1) | Bitboard::from(sq2);
                lines[sq1 as usize][sq2 as usize] |= bishop_1 & bishop_2;
            }
            if rook_1.contains(sq2) {
                let rook_2 = MAGIC.rook_attacks(Bitboard::EMPTY, sq2);
                lines[sq1 as usize][sq2 as usize] |= Bitboard::from(sq1) | Bitboard::from(sq2);

                lines[sq1 as usize][sq2 as usize] |= rook_1 & rook_2;
            }
        }
    }

    lines
});

/// A lookup table for the squares "between" two other squares, either down
/// a row like a rook or on a diagonal like a bishop. `between[A1][A3]`
/// would return a `Bitboard` with A2 as its only active square.
static BETWEEN: Lazy<[[Bitboard; 64]; 64]> = Lazy::new(|| {
    // start with an unitialized value and then set it element-wise
    let mut between = [[Bitboard::EMPTY; 64]; 64];

    for sq1 in Bitboard::ALL {
        for sq2 in Bitboard::ALL {
            if MAGIC.bishop_attacks(Bitboard::EMPTY, sq1).contains(sq2) {
                let bishop1 = MAGIC.bishop_attacks(Bitboard::from(sq2), sq1);
                let bishop2 = MAGIC.bishop_attacks(Bitboard::from(sq1), sq2);

                between[sq1 as usize][sq2 as usize] |= bishop1 & bishop2;
            }
            if MAGIC.rook_attacks(Bitboard::EMPTY, sq1).contains(sq2) {
                let rook1 = MAGIC.rook_attacks(Bitboard::from(sq2), sq1);
                let rook2 = MAGIC.rook_attacks(Bitboard::from(sq1), sq2);

                between[sq1 as usize][sq2 as usize] |= rook1 & rook2;
            }
        }
    }

    between
});

/// A bitboard, which uses an integer to express a set of `Square`s.
/// This expression allows the efficient computation of set intersection, union,
/// disjunction, element selection, and more, all in constant time.
///
/// Nearly all board-related representations use `Bitboard`s as a key part of
/// their construction.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bitboard(u64);

impl Bitboard {
    /// A bitboard representing the empty set.
    /// Accordingly, `Bitboard::EMPTY` contains no squares, and functions
    /// exactly like the empty set in all observable behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// let sq = Square::A1; // this could be any square
    /// assert!(!Bitboard::EMPTY.contains(sq));
    /// ```
    pub const EMPTY: Bitboard = Bitboard::new(0);

    /// A bitboard containing all 64 squares on the board, i.e. the universal
    /// set.
    ///
    /// Often, it can be used as an efficient way to iterate over every square
    /// of a board.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// let sq = Square::A1;
    /// assert!(Bitboard::ALL.contains(sq));
    /// ```
    ///
    /// Use as an iterator over all squares:
    /// ```
    /// use fiddler_base::{Bitboard};
    ///
    /// for sq in Bitboard::ALL {
    ///     println!("Now visiting square {sq}!");
    /// }
    /// ```
    pub const ALL: Bitboard = Bitboard::new(!0);

    #[inline(always)]
    #[must_use]
    /// Construct a new Bitboard from a numeric literal.
    /// Internally, `Bitboard`s are 64-bit integers, where the LSB represents
    /// whether the square A1 is an element, the second-least bit represents the
    /// square A2, and so on.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// bb.insert(Square::A1);
    ///
    /// assert_eq!(bb, Bitboard::new(1));
    /// ```
    pub const fn new(x: u64) -> Bitboard {
        Bitboard(x)
    }

    #[inline(always)]
    #[must_use]
    /// Determine whether this bitboard contains a given square.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// assert!(Bitboard::new(1).contains(Square::A1));
    /// assert!(!(Bitboard::new(2).contains(Square::A1)));
    /// ```
    pub const fn contains(self, square: Square) -> bool {
        self.0 & (1 << square as u8) != 0
    }

    #[inline(always)]
    /// Add a square to the set of squares contained in this `Bitboard`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// bb.insert(Square::A1);
    /// assert!(bb.contains(Square::A1));
    /// ```
    pub fn insert(&mut self, sq: Square) {
        self.0 |= 1 << sq as u8;
    }

    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    /// Compute the number of squares contained in this `Bitboard`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert!(bb.len() == 0);
    /// bb.insert(Square::A1);
    /// assert!(bb.len() == 1);
    /// ```
    pub const fn len(self) -> u8 {
        self.0.count_ones() as u8
    }

    #[inline(always)]
    #[must_use]
    /// Count the number of trailing zeros (i.e. empty squares between A1 and
    /// the first non-empty square) in this bitboard. Alternately, this can be
    /// used to construct a `Square` from the lowest-rank square in this
    /// bitboard.
    pub const fn trailing_zeros(self) -> u32 {
        self.0.trailing_zeros()
    }

    #[must_use]
    /// Count the number of leading zeros (i.e. empty squares between H8 and
    /// the highest non-empty square). Will be zero if H8 is occupied.
    pub const fn leading_zeros(self) -> u32 {
        self.0.leading_zeros()
    }

    #[must_use]
    #[inline(always)]
    /// Determine whether this bitboard is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert!(bb.is_empty());
    /// bb.insert(Square::A1);
    /// assert!(!bb.is_empty());
    /// ```
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    #[inline(always)]
    /// Determine whether this bitboard has exactly one bit. Equivalent to
    /// `Bitboard.len() == 1`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert!(!bb.has_single_bit());
    /// bb.insert(Square::A1);
    /// assert!(bb.has_single_bit());
    /// bb.insert(Square::A2);
    /// assert!(!bb.has_single_bit());
    /// ```
    pub const fn has_single_bit(self) -> bool {
        // 5 arithmetic operations,
        // faster than the 13 required for `count_ones() == 1`
        self.0 != 0 && (self.0 & self.0.overflowing_sub(1).0) == 0
    }

    #[must_use]
    /// Determine whether this bitboard conains more than one `Square`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert!(!bb.more_than_one());
    /// bb.insert(Square::A1);
    /// assert!(!bb.more_than_one());
    /// bb.insert(Square::A2);
    /// assert!(bb.more_than_one());
    /// ```
    pub const fn more_than_one(self) -> bool {
        (self.0 & self.0.overflowing_sub(1).0) != 0
    }

    #[inline(always)]
    #[must_use]
    /// Get a bitboard of all the squares between the two given squares, along
    /// the moves of a bishop or rook.
    pub fn between(sq1: Square, sq2: Square) -> Bitboard {
        BETWEEN[sq1 as usize][sq2 as usize]
    }

    #[inline(always)]
    #[must_use]
    /// Get a `Bitboard` containing all squares along the line between `sq1` and
    /// `sq`. Squares which are not aligned (in the ways that a rook or bishop
    /// move) will result in a return of `Bitboard::EMPTY`.
    pub fn line(sq1: Square, sq2: Square) -> Bitboard {
        LINES[sq1 as usize][sq2 as usize]
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    #[inline(always)]
    /// Compute the intersection of the sets represented by this bitboard and
    /// the right-hand side.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fiddler_base::Square;
    /// # use fiddler_base::Bitboard;
    /// let bb1 = Bitboard::new(7); // {A1, B1, C1}
    /// let bb2 = Bitboard::new(14); // {B1, C1, D1}
    /// let intersection = bb1 & bb2; // {B1, C1}
    /// assert!(!intersection.contains(Square::A1));
    /// assert!(intersection.contains(Square::B1));
    /// assert!(intersection.contains(Square::C1));
    /// assert!(!intersection.contains(Square::D1));
    /// ```
    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl Shl<u8> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shl(self, rhs: u8) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl Shr<u8> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shr(self, rhs: u8) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl ShlAssign<u8> for Bitboard {
    #[inline(always)]
    fn shl_assign(&mut self, rhs: u8) {
        self.0 <<= rhs;
    }
}

impl Not for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl AddAssign for Bitboard {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Mul for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Bitboard(self.0.wrapping_mul(rhs.0))
    }
}

impl From<Square> for Bitboard {
    #[inline(always)]
    fn from(sq: Square) -> Bitboard {
        Bitboard(1 << sq as u8)
    }
}

impl From<Bitboard> for usize {
    fn from(bb: Bitboard) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        {
            bb.0 as usize
        }
    }
}

impl Display for Bitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for row_idx in 0..8 {
            for col_idx in 0..8 {
                let bit = 1 << ((8 * (7 - row_idx)) + col_idx);
                if bit & self.0 == 0 {
                    write!(f, ". ")?;
                } else {
                    write!(f, "1 ")?;
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

#[allow(clippy::copy_iterator)]
impl Iterator for Bitboard {
    type Item = Square;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            return None;
        }
        // SAFETY: The empty bitboard case has been handled already, so the
        // number of trailing zeros is between 0 and 63.
        let result = Some(unsafe {
            transmute(
                #[allow(clippy::cast_possible_truncation)]
                {
                    self.trailing_zeros() as u8
                },
            )
        });
        self.0 &= self.0 - 1;
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count_ones() as usize;
        (n, Some(n))
    }
}
