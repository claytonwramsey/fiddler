use crate::base::constants::{FILE_NAMES, RANK_NAMES};
use crate::base::Bitboard;
use crate::base::Direction;

use std::cmp::max;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::mem::transmute;
use std::ops::{Add, AddAssign, Sub};

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
/// A single integer containing all the data to identify one square on a board.
/// From MSB to LSB, this is composed of:
/// * 2 unused bits
/// * 3 bits for the rank
/// * 3 bits for the file
pub enum Square {
    A1 = 0,
    B1 = 1,
    C1 = 2,
    D1 = 3,
    E1 = 4,
    F1 = 5,
    G1 = 6,
    H1 = 7,
    A2 = 8,
    B2 = 9,
    C2 = 10,
    D2 = 11,
    E2 = 12,
    F2 = 13,
    G2 = 14,
    H2 = 15,
    A3 = 16,
    B3 = 17,
    C3 = 18,
    D3 = 19,
    E3 = 20,
    F3 = 21,
    G3 = 22,
    H3 = 23,
    A4 = 24,
    B4 = 25,
    C4 = 26,
    D4 = 27,
    E4 = 28,
    F4 = 29,
    G4 = 30,
    H4 = 31,
    A5 = 32,
    B5 = 33,
    C5 = 34,
    D5 = 35,
    E5 = 36,
    F5 = 37,
    G5 = 38,
    H5 = 39,
    A6 = 40,
    B6 = 41,
    C6 = 42,
    D6 = 43,
    E6 = 44,
    F6 = 45,
    G6 = 46,
    H6 = 47,
    A7 = 48,
    B7 = 49,
    C7 = 50,
    D7 = 51,
    E7 = 52,
    F7 = 53,
    G7 = 54,
    H7 = 55,
    A8 = 56,
    B8 = 57,
    C8 = 58,
    D8 = 59,
    E8 = 60,
    F8 = 61,
    G8 = 62,
    H8 = 63,
}

impl Square {
    #[inline]
    /// Create a Square from the given rank and file. The ranks run from 0 to 7
    /// (instead of 1 through 8), and the files run from A to H.
    pub fn new(rank: usize, file: usize) -> Option<Square> {
        Square::try_from(((rank << 3) | file) as u8).ok()
    }

    #[inline]
    /// Get the integer representing the rank (0 -> 1, ...) of this square.
    pub const fn rank(&self) -> usize {
        (*self as u8 >> 3u8) as usize
    }

    #[inline]
    /// Get the integer representing the file (0 -> A, ...) of this square.
    pub const fn file(self) -> usize {
        (self as u8 & 7u8) as usize
    }

    #[inline]
    /// Get the Chebyshev distance to another square.
    pub fn chebyshev_to(&self, rhs: Square) -> u8 {
        let rankdiff = ((rhs.rank() as i16) - (self.rank() as i16)).abs();
        let filediff = ((rhs.file() as i16) - (self.file() as i16)).abs();

        max(rankdiff, filediff) as u8
    }

    #[inline]
    /// Get what this square would appear to be from the point of view of the
    /// opposing player.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::Square;
    /// let sq1 = Square::A1;
    /// let sq2 = sq1.opposite();
    /// assert_eq!(sq2, Square::A8);
    /// ```
    pub fn opposite(&self) -> Square {
        Square::new(7 - self.rank(), self.file()).unwrap()
    }

    /// Convert an algebraic string (such as 'e7') to a square.
    /// To get an `Ok` result, the string must be two characters.
    /// The file must be in lowercase.
    pub fn from_algebraic(s: &str) -> Result<Square, &'static str> {
        if s.len() != 2 {
            return Err("square name must be 2 characters");
        }
        let mut chars = s.chars();
        let (file, _) = match "abcdefgh".match_indices(chars.next().unwrap()).next() {
            Some(d) => d,
            None => return Err("illegal file for square"),
        };
        let rank = match chars.next().unwrap().to_digit(10) {
            Some(n) => n as usize,
            None => return Err("expected number for square rank"),
        };
        // will not fail because we have already validated the rank and file
        Ok(Square::new(rank - 1, file).unwrap())
    }

    /// Unsafely convert a `Bitboard` to a `Square` by creating the square
    /// representing its lowest occupied bit. Will result in undefined behavior
    /// (most likely a `Square` whose enum value is not in 0..64) if the given
    /// board is empty.
    ///
    /// # Safety
    ///
    /// This function will behave safely if `bb` is nonzero. It will result in
    /// undefined behavior if `bb` is equal to `Bitboard::EMPTY`.
    pub unsafe fn unsafe_from(bb: Bitboard) -> Square {
        transmute(bb.trailing_zeros() as u8)
    }
}

impl Add<Direction> for Square {
    type Output = Square;
    #[inline]
    fn add(self, rhs: Direction) -> Self::Output {
        // Apply the modulo to prevent UB.
        unsafe { transmute(((self as i8) + rhs.0) as u8 & 63) }
    }
}

impl AddAssign<Direction> for Square {
    #[inline]
    fn add_assign(&mut self, rhs: Direction) {
        *self = *self + rhs;
    }
}

impl Display for Square {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", FILE_NAMES[self.file()], RANK_NAMES[self.rank()])
    }
}

impl Sub<Square> for Square {
    type Output = Direction;
    #[inline]
    fn sub(self, rhs: Square) -> Self::Output {
        Direction((self as i8) - (rhs as i8))
    }
}

impl Sub<Direction> for Square {
    type Output = Square;
    #[inline]
    fn sub(self, rhs: Direction) -> Self::Output {
        Square::try_from(((self as i8) - (rhs.0)) as u8 & 63u8).unwrap()
    }
}

impl TryFrom<Bitboard> for Square {
    type Error = &'static str;

    /// Create the square closest to A1 (prioritizing rank) on the given
    /// bitboard.
    #[inline]
    fn try_from(bb: Bitboard) -> Result<Square, Self::Error> {
        Square::try_from(bb.trailing_zeros() as u8)
    }
}

impl TryFrom<u8> for Square {
    type Error = &'static str;
    #[inline]
    fn try_from(x: u8) -> Result<Square, Self::Error> {
        match x {
            // This transmutation is safe because i will always be less than
            // the total number of squares.
            x if x <= Square::H8 as u8 => Ok(unsafe { transmute(x) }),
            _ => Err("input for square conversion is out of bounds"),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::base::Direction;

    #[test]
    fn test_add_square_and_direction() {
        assert_eq!(Square::A1 + Direction::EAST, Square::B1);
        assert_eq!(Square::A1 + Direction::NORTHEAST, Square::B2);
    }

    #[test]
    fn test_add_direction_and_square() {
        assert_eq!(Direction::EAST + Square::A1, Square::B1);
    }

    #[test]
    fn test_square_from_algebraic() {
        assert_eq!(Square::from_algebraic("e4"), Ok(Square::E4));
        assert_eq!(Square::from_algebraic("f7"), Ok(Square::F7));
    }
}
