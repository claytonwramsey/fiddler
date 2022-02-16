use crate::base::Square;
use std::fmt::{Display, Formatter, Result};
use std::iter::Iterator;
use std::mem::transmute;
use std::ops::{
    AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Mul, Not, Shl, ShlAssign, Shr,
};

///
/// A bitboard to express sets of `Square`s.
/// uses standard form, so from MSB to LSB, the squares are H8G8F8 (...) C1B1A1.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bitboard(pub u64);

impl Bitboard {
    ///
    /// An empty bitboard.
    ///
    pub const EMPTY: Bitboard = Bitboard(0);

    ///
    /// A bitboard containing all squares.
    ///
    pub const ALL: Bitboard = Bitboard(!0);

    #[inline]
    ///
    /// Determine whether a square of a bitboard is occupied.
    ///
    pub const fn contains(self, square: Square) -> bool {
        self.0 & (1 << square as u8) != 0
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl Shl<i8> for Bitboard {
    type Output = Self;

    #[inline]
    fn shl(self, rhs: i8) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl Shr<i8> for Bitboard {
    type Output = Self;

    #[inline]
    fn shr(self, rhs: i8) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl Shl<i32> for Bitboard {
    type Output = Self;

    #[inline]
    fn shl(self, rhs: i32) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl ShlAssign<i32> for Bitboard {
    #[inline]
    fn shl_assign(&mut self, rhs: i32) {
        self.0 <<= rhs;
    }
}

impl Shr<i32> for Bitboard {
    type Output = Self;

    #[inline]
    fn shr(self, rhs: i32) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl Not for Bitboard {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl AddAssign for Bitboard {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Mul for Bitboard {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Bitboard(self.0.wrapping_mul(rhs.0))
    }
}

impl From<Square> for Bitboard {
    #[inline]
    fn from(sq: Square) -> Bitboard {
        Bitboard(1 << sq as u8)
    }
}

impl Display for Bitboard {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Bitboard({:#18x})", self.0)
    }
}

impl Iterator for Bitboard {
    type Item = Square;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if *self == Bitboard::EMPTY {
            return None;
        }
        // This will not cause UB because we already accounted for the empty
        // board case.
        let result = Some(unsafe { transmute(self.0.trailing_zeros() as u8) });
        self.0 &= self.0 - 1;
        result
    }
}
