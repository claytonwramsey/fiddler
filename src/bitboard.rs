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
    mem::transmute,
};

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
    pub const ALL: Bitboard = Bitboard::new(!0);

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
    pub const fn with_square(self, sq: Square) -> Bitboard {
        Bitboard(self.0 | (1 << sq as u8))
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
    /// Determine whether this bitboard has exactly one bit.
    /// This function is equivalent to `Bitboard.len() == 1`, but it is slightly faster.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert!(!bb.has_single_bit());
    /// bb.insert(Square::A1);
    /// assert!(bb.has_single_bit());
    /// bb.insert(Square::A2);
    /// assert!(!bb.has_single_bit());
    /// ```
    pub const fn has_single_bit(self) -> bool {
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
    pub const fn diagonal(sq: Square) -> Bitboard {
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
    pub const fn anti_diagonal(sq: Square) -> Bitboard {
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
    pub const fn vertical(sq: Square) -> Bitboard {
        const COL_A: Bitboard = Bitboard(0x0101_0101_0101_0101);

        Bitboard(COL_A.0 << sq.file())
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
    pub const fn horizontal(sq: Square) -> Bitboard {
        const RANK_1: Bitboard = Bitboard(0x0000_0000_0000_00FF);

        Bitboard(RANK_1.0 << (sq.rank() << 3))
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
    pub const fn hv(sq: Square) -> Bitboard {
        const MASKS: [Bitboard; 64] = {
            let mut masks = [Bitboard::EMPTY; 64];
            let mut i = 0u8;
            while i < 64 {
                let sq = unsafe { transmute(i) };
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
    pub const fn diags(sq: Square) -> Bitboard {
        const MASKS: [Bitboard; 64] = {
            let mut masks = [Bitboard::EMPTY; 64];
            let mut i = 0u8;
            while i < 64 {
                let sq = unsafe { transmute(i) };
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