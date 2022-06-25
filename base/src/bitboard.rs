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

use super::Square;

use std::{
    fmt::{Display, Formatter, Result},
    iter::Iterator,
    mem::transmute,
    ops::{
        AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Mul, Not, Shl, ShlAssign, Shr,
    },
};

/// A bitboard to express sets of `Square`s.
/// The LSB of the internal bitboard represents whether A1 is included; the
/// second-lowest represents B1, and so on, until the MSB is H8.
/// For example, `Bitboard(3)` represents the set `{A1, B1}`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bitboard(u64);

impl Bitboard {
    /// An empty bitboard.
    pub const EMPTY: Bitboard = Bitboard::new(0);

    /// A bitboard containing all squares.
    pub const ALL: Bitboard = Bitboard::new(!0);

    #[inline(always)]
    /// Construct a new Bitboard from a numeric literal.
    pub const fn new(x: u64) -> Bitboard {
        Bitboard(x)
    }

    #[inline(always)]
    /// Determine whether a square of a bitboard is occupied.
    /// # Examples
    /// ```
    /// # use fiddler_base::Bitboard;
    /// # use fiddler_base::Square;
    /// assert!(Bitboard::new(1).contains(Square::A1));
    /// assert!(!(Bitboard::new(2).contains(Square::A1)));
    /// ```
    pub const fn contains(&self, square: Square) -> bool {
        self.0 & (1 << square as u8) != 0
    }

    #[inline(always)]
    /// Count the number of ones in this bitboard.
    pub const fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }

    #[inline(always)]
    /// Count the number of trailing zeros (i.e. empty squares between A1 and
    /// the first non-emtpy square) in this bitboard. Alternately, this can be
    /// used to construct a `Square` from the lowest-rank square in this
    /// bitboard.
    pub const fn trailing_zeros(&self) -> u32 {
        self.0.trailing_zeros()
    }

    /// Count the number of leading zeros (i.e. empty squares between H8 and
    /// the highest non-empty square). Will be zero if H8 is occupied.
    pub const fn leading_zeros(&self) -> u32 {
        self.0.leading_zeros()
    }

    /// Determine whether this bitboard is empty.
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Determine whether this bitboard has exactly one bit. Equivalent to
    /// `Bitboard.count_ones() == 1`.
    pub const fn has_single_bit(&self) -> bool {
        // 5 arithmetic operations,
        // faster than the 13 required for `count_ones() == 1`
        self.0 != 0 && (self.0 & self.0.overflowing_sub(1).0) == 0
    }

    /// Determine whether more than one bit is set in this bitboard.
    pub const fn more_than_one(&self) -> bool {
        (self.0 & self.0.overflowing_sub(1).0) != 0
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    #[inline(always)]
    /// Compute the intersection of the sets represented by this bitboard and
    /// the right-hand side.
    /// # Examples
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

impl Shl<i8> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shl(self, rhs: i8) -> Self::Output {
        match rhs < 0 {
            false => Bitboard(self.0 << rhs),
            true => Bitboard(self.0 >> -rhs),
        }
    }
}

impl Shr<i8> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shr(self, rhs: i8) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl Shr<u8> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shr(self, rhs: u8) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl Shl<i32> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shl(self, rhs: i32) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl Shl<usize> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shl(self, rhs: usize) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl ShlAssign<i32> for Bitboard {
    #[inline(always)]
    fn shl_assign(&mut self, rhs: i32) {
        self.0 <<= rhs;
    }
}

impl Shr<i32> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shr(self, rhs: i32) -> Self::Output {
        Bitboard(self.0 >> rhs)
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
        bb.0 as usize
    }
}

impl Display for Bitboard {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Bitboard({:#18x})", self.0)
    }
}

impl Iterator for Bitboard {
    type Item = Square;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            return None;
        }
        // SAFETY: The empty bitboard case has been handled already, so the
        // number of trailing zeros is between 0 and 63.
        let result = Some(unsafe { transmute(self.trailing_zeros() as u8) });
        self.0 &= self.0 - 1;
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count_ones() as usize;
        (n, Some(n))
    }
}
