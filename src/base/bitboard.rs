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

use std::{
    fmt::{Display, Formatter, Result},
    mem::transmute,
    ops::{
        BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    },
};

use crate::base::movegen::{bishop_attacks, rook_attacks};

use super::Square;

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
    pub const EMPTY: Self = Self::new(0);

    /// A bitboard containing all 64 squares on the board, i.e. the universal set.
    ///
    /// Often, it can be used as an efficient way to iterate over every square of a board.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let sq = Square::A1;
    /// assert!(Bitboard::ALL.contains(sq));
    /// ```
    ///
    /// Use as an iterator over all squares:
    /// ```
    /// use fiddler::base::Bitboard;
    ///
    /// for sq in Bitboard::ALL {
    ///     println!("Now visiting square {sq}!");
    /// }
    /// ```
    pub const ALL: Self = Self::new(!0);

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
    pub const fn new(x: u64) -> Self {
        Self(x)
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

    /// Add a square to the set of squares contained in this `Bitboard`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert!(!bb.contains(Square::A1));
    /// bb.insert(Square::A1);
    /// assert!(bb.contains(Square::A1));
    /// ```
    pub fn insert(&mut self, sq: Square) {
        self.0 |= 1 << sq as u8;
    }

    #[must_use]
    /// Create a new `Bitboard` which is the same as this one, but with the square `sq` inserted.
    /// Returns a copy if `sq` was alreay contained by this bitboard.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let bb1 = Bitboard::EMPTY;
    /// let bb2 = bb1.with_square(Square::A1);
    ///
    /// assert!(!bb1.contains(Square::A1));
    /// assert!(bb2.contains(Square::A1));
    /// ```
    pub const fn with_square(self, sq: Square) -> Self {
        Self(self.0 | (1 << sq as u8))
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
    /// Count the number of trailing zeros (i.e. empty squares between A1 and
    /// the first non-empty square) in this bitboard. Alternately, this can be
    /// used to construct a `Square` from the lowest-rank square in this
    /// bitboard.
    pub const fn trailing_zeros(self) -> u32 {
        self.0.trailing_zeros()
    }

    #[must_use]
    /// Determine whether this bitboard is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
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
    /// Determine whether this bitboard has exactly one element.
    /// This function is equivalent to `Bitboard.len() == 1`, but it is slightly faster.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert!(!bb.just_one());
    /// bb.insert(Square::A1);
    /// assert!(bb.just_one());
    /// bb.insert(Square::A2);
    /// assert!(!bb.just_one());
    /// ```
    pub const fn just_one(self) -> bool {
        // use bitwise and to make it branchless
        (self.0 != 0) & ((self.0 & self.0.overflowing_sub(1).0) == 0)
    }

    #[must_use]
    /// Determine whether this bitboard conains more than one `Square`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
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

    #[must_use]
    /// Get a bitboard of all the squares between the two given squares, along the moves of a
    /// bishop or rook.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let between_bb = Bitboard::between(Square::A1, Square::A3);
    /// assert_eq!(between_bb, Bitboard::from(Square::A2))
    /// ```
    pub fn between(sq1: Square, sq2: Square) -> Self {
        /// A lookup table for the squares "between" two other squares, either down a row like a
        /// rook or on a diagonal like a bishop.
        ///
        /// `between[A1][A3]` would return a `Bitboard` with A2 as its only active square.
        const BETWEEN: [[Bitboard; 64]; 64] = {
            // start with an unitialized value and then set it element-wise
            let mut between = [[Bitboard::EMPTY; 64]; 64];

            let mut i = 0;
            while i < 64 {
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let sq1: Square = unsafe { transmute(i as u8) };
                let batt = bishop_attacks(Bitboard::EMPTY, sq1);
                let ratt = rook_attacks(Bitboard::EMPTY, sq1);

                let mut j = 0;

                // our between table is symmetric; calculate half and copy the values across the
                // diagonal
                while j < i {
                    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                    let sq2: Square = unsafe { transmute(j as u8) };

                    if batt.contains(sq2) {
                        between[i][j] = Bitboard::new(
                            bishop_attacks(Bitboard::new(1 << j), sq1).as_u64()
                                & bishop_attacks(Bitboard::new(1 << i), sq2).as_u64(),
                        );
                    }

                    if ratt.contains(sq2) {
                        between[i][j] = Bitboard::new(
                            rook_attacks(Bitboard::new(1 << j), sq1).as_u64()
                                & rook_attacks(Bitboard::new(1 << i), sq2).as_u64(),
                        );
                    }

                    between[j][i] = between[i][j];
                    j += 1;
                }
                i += 1;
            }

            between
        };

        unsafe {
            // SAFETY: Because a square is always in the range 0..64, these squares are always valid
            // indices.
            *BETWEEN
                .get_unchecked(sq1 as usize)
                .get_unchecked(sq2 as usize)
        }
    }

    #[must_use]
    /// Get a `Bitboard` containing all squares along the line between `sq1` and `sq2`.
    ///
    /// Squares which are not aligned (in the ways that a rook or bishop move) will result in a
    /// return of `Bitboard::EMPTY`.
    pub fn line(sq1: Square, sq2: Square) -> Self {
        const LINES: [[Bitboard; 64]; 64] = {
            let mut lines = [[Bitboard::EMPTY; 64]; 64];

            let mut i = 0u8;
            while i < 64 {
                let sq1: Square = unsafe { transmute(i) };
                let i_bb = 1 << i;
                let bishop_1 = Bitboard::diags(sq1).0;
                let rook_1 = Bitboard::hv(sq1).0;
                let mut j = 0u8;
                while j < 64 {
                    let sq2: Square = unsafe { transmute(j) };
                    let j_bb = 1 << j;
                    if bishop_1 & j_bb != 0 {
                        let bishop_2 = Bitboard::diags(sq2).0;
                        lines[i as usize][j as usize] = Bitboard(
                            lines[i as usize][j as usize].0 | i_bb | j_bb | (bishop_1 & bishop_2),
                        );
                    }
                    if rook_1 & j_bb != 0 {
                        let rook_2 = Bitboard::hv(sq2).0;
                        lines[i as usize][j as usize] = Bitboard(
                            lines[i as usize][j as usize].0 | i_bb | j_bb | (rook_1 & rook_2),
                        );
                    }
                    j += 1;
                }
                i += 1;
            }

            lines
        };

        unsafe {
            // SAFETY: Because a square is always in the range 0..64, these
            // squares are always valid indices.
            *LINES
                .get_unchecked(sq1 as usize)
                .get_unchecked(sq2 as usize)
        }
    }

    #[must_use]
    /// Get the primary diagonal running through a square, parallel to the diagonal from A1 through
    /// H8.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let sq = Square::F1;
    /// let mut diag = Bitboard::EMPTY
    ///     .with_square(Square::F1)
    ///     .with_square(Square::G2)
    ///     .with_square(Square::H3);
    ///
    /// assert_eq!(diag, Bitboard::diagonal(sq));
    /// ```
    pub const fn diagonal(sq: Square) -> Self {
        /// The diagonal going from A1 to H8.
        const MAIN_DIAG: Bitboard = Bitboard(0x8040_2010_0804_0201);

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        const DIAGONAL: [Bitboard; 64] = {
            let mut boards = [Bitboard::EMPTY; 64];
            // the classic for-loop hack
            let mut i = 0i32;
            while i < 64 {
                let main_diag = 8 * (i & 7) - (i & 56);
                let main_left_shift = (-main_diag & (main_diag >> 31)) as u8;
                let main_right_shift = (main_diag & (-main_diag >> 31)) as u8;
                let main_diag_mask = (MAIN_DIAG.0 >> main_right_shift) << main_left_shift;
                boards[i as usize] = Bitboard(main_diag_mask);
                i += 1;
            }

            boards
        };

        DIAGONAL[sq as usize]
    }

    #[must_use]
    /// Get the anti-diagonal running through a square, parallel to the diagonal from A8 through H1.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let sq = Square::A3;
    /// let mut diag = Bitboard::EMPTY
    ///     .with_square(Square::A3)
    ///     .with_square(Square::B2)
    ///     .with_square(Square::C1);
    ///
    /// assert_eq!(diag, Bitboard::anti_diagonal(sq));
    /// ```
    pub const fn anti_diagonal(sq: Square) -> Self {
        /// The diagonal going from A8 to H1.
        const ANTI_DIAG: Bitboard = Bitboard::new(0x0102_0408_1020_4080);

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        /// Lookup table used for `Bitboard::anti_diagonal()`.
        const ANTI_DIAGONAL: [Bitboard; 64] = {
            let mut boards = [Bitboard::EMPTY; 64];
            // the classic for-loop hack
            let mut i = 0i32;
            while i < 64 {
                let anti_diag = 56 - 8 * (i & 7) - (i & 56);
                let anti_left_shift = (-anti_diag & (anti_diag >> 31)) as u8;
                let anti_right_shift = (anti_diag & (-anti_diag >> 31)) as u8;
                let anti_diag_mask = (ANTI_DIAG.0 >> anti_right_shift) << anti_left_shift;
                boards[i as usize] = Bitboard(anti_diag_mask);
                i += 1;
            }

            boards
        };

        ANTI_DIAGONAL[sq as usize]
    }

    #[must_use]
    /// Get the set of all squares in the same file as a given square.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let bb = Bitboard::EMPTY
    ///     .with_square(Square::A1)
    ///     .with_square(Square::A2)
    ///     .with_square(Square::A3)
    ///     .with_square(Square::A4)
    ///     .with_square(Square::A5)
    ///     .with_square(Square::A6)
    ///     .with_square(Square::A7)
    ///     .with_square(Square::A8);
    ///
    /// assert_eq!(Bitboard::vertical(Square::A1), bb);
    /// ```
    pub const fn vertical(sq: Square) -> Self {
        const COL_A: Bitboard = Bitboard(0x0101_0101_0101_0101);

        Self(COL_A.0 << sq.file())
    }

    #[must_use]
    /// Get the set of all squares in the same rank as a given square.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let bb = Bitboard::EMPTY
    ///     .with_square(Square::A1)
    ///     .with_square(Square::B1)
    ///     .with_square(Square::C1)
    ///     .with_square(Square::D1)
    ///     .with_square(Square::E1)
    ///     .with_square(Square::F1)
    ///     .with_square(Square::G1)
    ///     .with_square(Square::H1);
    ///
    /// assert_eq!(Bitboard::horizontal(Square::A1), bb);
    /// ```
    pub const fn horizontal(sq: Square) -> Self {
        const RANK_1: Bitboard = Bitboard(0x0000_0000_0000_00FF);

        Self(RANK_1.0 << (sq.rank() << 3))
    }

    #[must_use]
    /// Get a `Bitboard` containing all squares in the same rank or file as `sq`, but not including
    /// `sq`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// assert_eq!(
    ///     Bitboard::hv(Square::A1),
    ///     Bitboard::horizontal(Square::A1) ^ Bitboard::vertical(Square::A1)
    /// );
    /// ```
    pub const fn hv(sq: Square) -> Self {
        const MASKS: [Bitboard; 64] = {
            let mut masks = [Bitboard::EMPTY; 64];
            let mut i = 0u8;
            while i < 64 {
                let sq = unsafe { transmute::<u8, Square>(i) };
                masks[i as usize] =
                    Bitboard::new(Bitboard::vertical(sq).0 ^ Bitboard::horizontal(sq).0);
                i += 1;
            }

            masks
        };

        MASKS[sq as usize]
    }

    #[must_use]
    /// Get a `Bitboard` containing all squares in the same diagonal or anti-diagonal as `sq`, but
    /// not including `sq`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// assert_eq!(
    ///     Bitboard::diags(Square::E4),
    ///     Bitboard::diagonal(Square::E4) ^ Bitboard::anti_diagonal(Square::E4)
    /// );
    /// ```
    pub const fn diags(sq: Square) -> Self {
        const MASKS: [Bitboard; 64] = {
            let mut masks = [Bitboard::EMPTY; 64];
            let mut i = 0u8;
            while i < 64 {
                let sq = unsafe { transmute::<u8, Square>(i) };
                masks[i as usize] =
                    Bitboard::new(Bitboard::diagonal(sq).0 ^ Bitboard::anti_diagonal(sq).0);
                i += 1;
            }

            masks
        };

        MASKS[sq as usize]
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

/// Helper macro for definining binary operations on bitboards.
macro_rules! bb_binop_define {
    ($trait: ident, $fn_name: ident, $op: tt) => {
        impl $trait for Bitboard {
            type Output = Self;

            fn $fn_name(self, rhs: Self) -> Self::Output {
                Bitboard(self.0 $op rhs.0)
            }
        }
    };
}

bb_binop_define!(BitAnd, bitand, &);
bb_binop_define!(BitOr, bitor, |);
bb_binop_define!(BitXor, bitxor, ^);

/// Helper macro for defining assigning binary operations on bitboards.
macro_rules! bb_binassign_define {
    ($trait: ident, $fn_name: ident, $op: tt) => {
        impl $trait for Bitboard {
                        fn $fn_name(&mut self, rhs: Self) {
                self.0 $op rhs.0;
            }
        }
    };
}

bb_binassign_define!(BitAndAssign, bitand_assign, &=);
bb_binassign_define!(BitOrAssign, bitor_assign, |=);
bb_binassign_define!(BitXorAssign, bitxor_assign, ^=);

impl Shl<u8> for Bitboard {
    type Output = Self;

    fn shl(self, rhs: u8) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl Shr<u8> for Bitboard {
    type Output = Self;

    fn shr(self, rhs: u8) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl ShlAssign<u8> for Bitboard {
    fn shl_assign(&mut self, rhs: u8) {
        self.0 <<= rhs;
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl From<Square> for Bitboard {
    fn from(sq: Square) -> Self {
        Self(1 << sq as u8)
    }
}

impl From<Bitboard> for u64 {
    fn from(bb: Bitboard) -> Self {
        bb.0
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

    #[allow(clippy::cast_possible_truncation)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            return None;
        }
        let trailing = self.trailing_zeros() as u8;
        // SAFETY: The empty bitboard case has been handled already, so the number of trailing zeros
        // is between 0 and 63.
        let result = Some(unsafe { transmute::<u8, Square>(trailing) });
        self.0 ^= 1 << trailing;
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count_ones() as usize;
        (n, Some(n))
    }
}
