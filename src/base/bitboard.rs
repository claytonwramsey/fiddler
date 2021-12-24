use crate::base::Square;
use std::fmt::{Display, Formatter, Result};
use std::iter::Iterator;
use std::ops::{AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Mul, Not, Shl, Shr};

///
/// a bitboard to express positions/// uses standard form, so H8G8F8 (...)
/// C1B1A1.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bitboard(pub u64);

impl Bitboard {
    ///
    /// An empty bitboard.
    ///
    pub const EMPTY: Bitboard = Bitboard(0);

    #[inline]
    ///
    /// Determine whether a square of a bitboard is occupied.
    ///
    pub fn contains(self, square: Square) -> bool {
        self.0 & (1 << square.0) != 0
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
        Bitboard(1 << sq.0)
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
        match self {
            Bitboard(0) => None,
            _ => {
                let sq = Square::from(*self);
                self.0 &= !Bitboard::from(sq).0;
                Some(sq)
            }
        }
    }
}
