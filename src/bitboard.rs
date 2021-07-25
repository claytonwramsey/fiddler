use std::ops::BitAnd;

/* a bitboard to express positions
 * uses standard form, so H8G8F8 (...) C1B1A1.
 */
#[derive(Copy, Clone)]
pub struct Bitboard(u64);

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand (self, rhs: Self) -> Self::Output {
        return Self(self.0 & rhs.0);
    }
}

