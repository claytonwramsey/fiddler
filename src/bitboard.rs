use crate::square::Square;
use std::fmt::{Display, Formatter, Result};
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Mul, Not, Shl, Shr};

/* a bitboard to express positions
 * uses standard form, so H8G8F8 (...) C1B1A1.
 */
#[derive(Copy, Clone, Debug)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub fn is_square_occupied(self, square: Square) -> bool {
        self.0 & (1 << square.0) != 0
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        return Self(self.0 & rhs.0);
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        return Self(self.0 | rhs.0);
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        return Self(self.0 ^ rhs.0);
    }
}

impl Shl<i8> for Bitboard {
    type Output = Self;

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

    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl Mul for Bitboard {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Bitboard(self.0.wrapping_mul(rhs.0))
    }
}

impl PartialEq for Bitboard {
    fn eq(&self, rhs: &Bitboard) -> bool {
        return self.0 == rhs.0;
    }
}
impl Eq for Bitboard {}

impl From<Square> for Bitboard {
    fn from(sq: Square) -> Bitboard {
        Bitboard(1 << sq.0)
    }
}

impl Display for Bitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Bitboard({:#18x})", self.0)
    }
}
