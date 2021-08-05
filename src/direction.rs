use crate::square::Square;
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Copy, Clone, Debug)]
pub struct Direction(pub i8);

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

#[allow(dead_code)]
pub const NODIR: Direction = Direction(0);
//adding 8 to a square in conventional form brings up a row
#[allow(dead_code)]
pub const NORTH: Direction = Direction(8);
//adding 1 moves you east in conventional
#[allow(dead_code)]
pub const EAST: Direction = Direction(1);

//sadly, the nature of rust consts means this doesn't work
//pub const SOUTH: Direction = -NORTH;
#[allow(dead_code)]
pub const SOUTH: Direction = Direction(-8);
#[allow(dead_code)]
pub const WEST: Direction = Direction(-1);

//composite directions
#[allow(dead_code)]
pub const NORTHWEST: Direction = Direction(NORTH.0 + WEST.0);
#[allow(dead_code)]
pub const NORTHEAST: Direction = Direction(NORTH.0 + EAST.0);
#[allow(dead_code)]
pub const SOUTHEAST: Direction = Direction(SOUTH.0 + EAST.0);
#[allow(dead_code)]
pub const SOUTHWEST: Direction = Direction(SOUTH.0 + WEST.0);

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
