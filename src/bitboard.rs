use std::ops::{BitAnd, BitOr};
use std::fmt::{Display, Formatter, Result};
use crate::square::Square;

/* a bitboard to express positions
 * uses standard form, so H8G8F8 (...) C1B1A1.
 */
#[derive(Copy, Clone, Debug)]
pub struct Bitboard(pub u64);

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand (self, rhs: Self) -> Self::Output {
        return Self(self.0 & rhs.0);
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        return Self(self.0 | rhs.0);
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
        write!(f, "0x{:16x}", self.0)
    }
}
