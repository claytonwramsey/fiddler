use std::ops::BitAnd;
use crate::square::Square;

/* a bitboard to express positions
 * uses standard form, so H8G8F8 (...) C1B1A1.
 */
#[derive(Copy, Clone)]
pub struct Bitboard(pub u64);

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand (self, rhs: Self) -> Self::Output {
        return Self(self.0 & rhs.0);
    }
}


impl From<Square> for Bitboard {
    fn from(sq: Square) -> Bitboard {
        Bitboard(1 << sq.0)
    }
}
