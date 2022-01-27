use crate::base::constants::{FILE_NAMES, RANK_NAMES};
use crate::base::Bitboard;
use crate::base::Direction;

use std::cmp::max;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Sub};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
///
/// A single integer containing all the data to identify one square on a board.
/// From MSB to LSB, this is composed of:
/// * 2 unused bits
/// * 3 bits for the rank
/// * 3 bits for the file
///
pub struct Square(pub u8);

impl Square {
    #[inline]
    ///
    /// Create a Square from the given rank and file. The ranks run from 0 to 7
    /// (instead of 1 through 8), and the files run from A to H.
    ///
    pub fn new(rank: usize, file: usize) -> Square {
        Square((((rank & 7) << 3) | (file & 7)) as u8)
    }

    #[inline]
    ///
    /// Get the integer representing the rank (0 -> 1, ...) of this square.
    ///
    pub fn rank(self) -> usize {
        return ((self.0 >> 3u8) & 7u8) as usize;
    }

    #[inline]
    ///
    /// Get the integer representing the file (0 -> A, ...) of this square.
    ///
    pub fn file(self) -> usize {
        return (self.0 & 7u8) as usize;
    }

    #[inline]
    ///
    /// Return false if this is an illegal (i.e. inaccessible) square.
    ///
    pub fn is_inbounds(self) -> bool {
        self.0 < 64
    }

    #[inline]
    ///
    /// Get the Chebyshev distance to another square.
    ///
    pub fn chebyshev_to(self, rhs: Square) -> u8 {
        let rankdiff = ((rhs.rank() as i16) - (self.rank() as i16)).abs();
        let filediff = ((rhs.file() as i16) - (self.file() as i16)).abs();
        return max(rankdiff, filediff) as u8;
    }

    ///
    /// Convert an algebraic string (such as 'e7') to a square.
    /// To get an `Ok` result, the string must be two characters.
    /// The file must be in lowercase.
    ///
    pub fn from_algebraic(s: &str) -> Result<Square, &'static str> {
        if s.len() != 2 {
            return Err("square name must be 2 characters");
        }
        let (ep_file, _) = match "abcdefgh".match_indices(s.chars().nth(0).unwrap()).next() {
            Some(d) => d,
            None => return Err("illegal file for square"),
        };
        let ep_rank = match s.chars().nth(1).unwrap().to_digit(10) {
            Some(n) => n as usize,
            None => return Err("expected number for square rank"),
        };
        Ok(Square::new(ep_rank - 1, ep_file))
    }
}

impl Add<Direction> for Square {
    type Output = Square;
    #[inline]
    fn add(self, rhs: Direction) -> Self::Output {
        let new_square: i8 = (self.0 as i8) + rhs.0;
        return Square(new_square as u8);
    }
}

impl AddAssign<Direction> for Square {
    #[inline]
    fn add_assign(&mut self, rhs: Direction) {
        self.0 = ((self.0 as i8) + rhs.0) as u8;
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
        Direction((self.0 as i8) - (rhs.0 as i8))
    }
}

impl From<Bitboard> for Square {
    //Create the square closest to A1 (prioritizing rank) on the bitboard
    #[inline]
    fn from(bb: Bitboard) -> Square {
        //Comment this out if you think you're strong enough
        //new behavior: returns the square closest to A1 that is occupied
        /*if bb.0.count_ones() != 1 {
            return BAD_SQUARE;
        }*/
        return Square(bb.0.trailing_zeros() as u8);
    }
}

#[allow(dead_code)]
pub const A1: Square = Square(00);
#[allow(dead_code)]
pub const B1: Square = Square(01);
#[allow(dead_code)]
pub const C1: Square = Square(02);
#[allow(dead_code)]
pub const D1: Square = Square(03);
#[allow(dead_code)]
pub const E1: Square = Square(04);
#[allow(dead_code)]
pub const F1: Square = Square(05);
#[allow(dead_code)]
pub const G1: Square = Square(06);
#[allow(dead_code)]
pub const H1: Square = Square(07);
#[allow(dead_code)]
pub const A2: Square = Square(08);
#[allow(dead_code)]
pub const B2: Square = Square(09);
#[allow(dead_code)]
pub const C2: Square = Square(10);
#[allow(dead_code)]
pub const D2: Square = Square(11);
#[allow(dead_code)]
pub const E2: Square = Square(12);
#[allow(dead_code)]
pub const F2: Square = Square(13);
#[allow(dead_code)]
pub const G2: Square = Square(14);
#[allow(dead_code)]
pub const H2: Square = Square(15);
#[allow(dead_code)]
pub const A3: Square = Square(16);
#[allow(dead_code)]
pub const B3: Square = Square(17);
#[allow(dead_code)]
pub const C3: Square = Square(18);
#[allow(dead_code)]
pub const D3: Square = Square(19);
#[allow(dead_code)]
pub const E3: Square = Square(20);
#[allow(dead_code)]
pub const F3: Square = Square(21);
#[allow(dead_code)]
pub const G3: Square = Square(22);
#[allow(dead_code)]
pub const H3: Square = Square(23);
#[allow(dead_code)]
pub const A4: Square = Square(24);
#[allow(dead_code)]
pub const B4: Square = Square(25);
#[allow(dead_code)]
pub const C4: Square = Square(26);
#[allow(dead_code)]
pub const D4: Square = Square(27);
#[allow(dead_code)]
pub const E4: Square = Square(28);
#[allow(dead_code)]
pub const F4: Square = Square(29);
#[allow(dead_code)]
pub const G4: Square = Square(30);
#[allow(dead_code)]
pub const H4: Square = Square(31);
#[allow(dead_code)]
pub const A5: Square = Square(32);
#[allow(dead_code)]
pub const B5: Square = Square(33);
#[allow(dead_code)]
pub const C5: Square = Square(34);
#[allow(dead_code)]
pub const D5: Square = Square(35);
#[allow(dead_code)]
pub const E5: Square = Square(36);
#[allow(dead_code)]
pub const F5: Square = Square(37);
#[allow(dead_code)]
pub const G5: Square = Square(38);
#[allow(dead_code)]
pub const H5: Square = Square(39);
#[allow(dead_code)]
pub const A6: Square = Square(40);
#[allow(dead_code)]
pub const B6: Square = Square(41);
#[allow(dead_code)]
pub const C6: Square = Square(42);
#[allow(dead_code)]
pub const D6: Square = Square(43);
#[allow(dead_code)]
pub const E6: Square = Square(44);
#[allow(dead_code)]
pub const F6: Square = Square(45);
#[allow(dead_code)]
pub const G6: Square = Square(46);
#[allow(dead_code)]
pub const H6: Square = Square(47);
#[allow(dead_code)]
pub const A7: Square = Square(48);
#[allow(dead_code)]
pub const B7: Square = Square(49);
#[allow(dead_code)]
pub const C7: Square = Square(50);
#[allow(dead_code)]
pub const D7: Square = Square(51);
#[allow(dead_code)]
pub const E7: Square = Square(52);
#[allow(dead_code)]
pub const F7: Square = Square(53);
#[allow(dead_code)]
pub const G7: Square = Square(54);
#[allow(dead_code)]
pub const H7: Square = Square(55);
#[allow(dead_code)]
pub const A8: Square = Square(56);
#[allow(dead_code)]
pub const B8: Square = Square(57);
#[allow(dead_code)]
pub const C8: Square = Square(58);
#[allow(dead_code)]
pub const D8: Square = Square(59);
#[allow(dead_code)]
pub const E8: Square = Square(60);
#[allow(dead_code)]
pub const F8: Square = Square(61);
#[allow(dead_code)]
pub const G8: Square = Square(62);
#[allow(dead_code)]
pub const H8: Square = Square(63);
#[allow(dead_code)]
pub const BAD_SQUARE: Square = Square(64);

#[cfg(test)]
mod tests {

    use super::*;
    use crate::base::Direction;

    #[test]
    fn test_add_square_and_direction() {
        assert_eq!(A1 + Direction::EAST, B1);
        assert_eq!(A1 + Direction::NORTHEAST, B2);
    }

    #[test]
    fn test_add_direction_and_square() {
        assert_eq!(Direction::EAST + A1, B1);
    }
}
