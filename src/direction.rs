use crate::square::Square;
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Copy, Clone, Debug)]
pub struct Direction(pub i8);

impl Direction {
    #[inline]
    fn new(rank_step: i8, file_step: i8) -> Direction {
        Direction(rank_step + (file_step * 8))
    }
    #[inline]
    fn file_step(self) -> i8 {
        self.0 % 8
    }
    #[inline]
    fn rank_step(self) -> i8 {
        self.0 / 8
    }
}

impl Neg for Direction {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Direction(-self.0)
    }
}

impl Mul<Direction> for i8 {
    type Output = Direction;
    fn mul(self, rhs: Direction) -> Direction {
        Direction(self * rhs.0)
    }
}

impl Add<Square> for Direction {
    type Output = Square;
    fn add(self, rhs: Square) -> Self::Output {
        rhs + self
    }
}

impl Add<Direction> for Direction {
    type Output = Self;
    fn add(self, rhs: Direction) -> Self::Output {
        Direction(self.0 + rhs.0)
    }
}

impl Sub<Direction> for Direction {
    type Output = Self;
    fn sub(self, rhs: Direction) -> Self::Output {
        Direction(self.0 - rhs.0)
    }
}

impl PartialEq for Direction {
    fn eq(&self, rhs: &Direction) -> bool {
        return self.0 == rhs.0;
    }
}
impl Eq for Direction {}

pub const NODIR: Direction = Direction(0);
//adding 8 to a square in conventional form brings up a row
pub const NORTH: Direction = Direction(8);
//adding 1 moves you east in conventional
pub const EAST: Direction = Direction(1);

//sadly, the nature of rust consts means this doesn't work
//pub const SOUTH: Direction = -NORTH;
pub const SOUTH: Direction = Direction(-8);
pub const WEST: Direction = Direction(-1);

//composite directions
pub const NORTHWEST: Direction = Direction(NORTH.0 + WEST.0);
pub const NORTHEAST: Direction = Direction(NORTH.0 + EAST.0);
pub const SOUTHEAST: Direction = Direction(SOUTH.0 + EAST.0);
pub const SOUTHWEST: Direction = Direction(SOUTH.0 + WEST.0);

//knight directions
pub const NNW: Direction = Direction(2 * NORTH.0 + WEST.0);
pub const NNE: Direction = Direction(2 * NORTH.0 + EAST.0);
pub const NEE: Direction = Direction(NORTH.0 + 2 * EAST.0);
pub const SEE: Direction = Direction(SOUTH.0 + 2 * EAST.0);
pub const SSE: Direction = Direction(2 * SOUTH.0 + EAST.0);
pub const SSW: Direction = Direction(2 * SOUTH.0 + WEST.0);
pub const SWW: Direction = Direction(SOUTH.0 + 2 * WEST.0);
pub const NWW: Direction = Direction(NORTH.0 + 2 * WEST.0);

pub const ROOK_DIRECTIONS: [Direction; 4] = [NORTH, SOUTH, EAST, WEST];
pub const BISHOP_DIRECTIONS: [Direction; 4] = [NORTHWEST, NORTHEAST, SOUTHWEST, SOUTHEAST];

#[cfg(test)]
mod tests {

    #[allow(dead_code)]
    use super::*;

    #[test]
    fn test_add_directions() {
        assert_eq!(NODIR + EAST, EAST);
        assert_eq!(EAST + WEST, NODIR);
    }

    #[test]
    fn test_opposite_directions() {
        assert_eq!(-EAST, WEST);
        assert_eq!(-NORTH, SOUTH);
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(NORTHEAST - EAST, NORTH);
        assert_eq!(EAST - EAST, NODIR);
    }
}
