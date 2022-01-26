use crate::base::Square;
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
///
/// A difference between two squares. Directions form a vector field, which 
/// allows us to define subtraction between squares. Internally, they use the
/// same representation as a Square but with a signed integer.
///
pub struct Direction(pub i8);

impl Direction {
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
    pub const NORTHWEST: Direction = Direction(Direction::NORTH.0 + Direction::WEST.0);
    pub const NORTHEAST: Direction = Direction(Direction::NORTH.0 + Direction::EAST.0);
    pub const SOUTHEAST: Direction = Direction(Direction::SOUTH.0 + Direction::EAST.0);
    pub const SOUTHWEST: Direction = Direction(Direction::SOUTH.0 + Direction::WEST.0);

    //knight directions
    pub const NNW: Direction = Direction(2 * Direction::NORTH.0 + Direction::WEST.0);
    pub const NNE: Direction = Direction(2 * Direction::NORTH.0 + Direction::EAST.0);
    pub const NEE: Direction = Direction(Direction::NORTH.0 + 2 * Direction::EAST.0);
    pub const SEE: Direction = Direction(Direction::SOUTH.0 + 2 * Direction::EAST.0);
    pub const SSE: Direction = Direction(2 * Direction::SOUTH.0 + Direction::EAST.0);
    pub const SSW: Direction = Direction(2 * Direction::SOUTH.0 + Direction::WEST.0);
    pub const SWW: Direction = Direction(Direction::SOUTH.0 + 2 * Direction::WEST.0);
    pub const NWW: Direction = Direction(Direction::NORTH.0 + 2 * Direction::WEST.0);

    pub const ROOK_DIRECTIONS: [Direction; 4] = [
        Direction::NORTH,
        Direction::SOUTH,
        Direction::EAST,
        Direction::WEST,
    ];
    pub const BISHOP_DIRECTIONS: [Direction; 4] = [
        Direction::NORTHWEST,
        Direction::NORTHEAST,
        Direction::SOUTHWEST,
        Direction::SOUTHEAST,
    ];

    #[inline]
    #[allow(dead_code)]
    ///
    /// Create a new Direction based on how far it moves in rank and file.
    ///
    fn new(rank_step: i8, file_step: i8) -> Direction {
        Direction(rank_step + (file_step * 8))
    }
    #[inline]
    #[allow(dead_code)]
    ///
    /// Get the difference moved by a Direction in a file.
    ///
    fn file_step(self) -> i8 {
        self.0 % 8
    }
    #[inline]
    #[allow(dead_code)]
    ///
    /// Get the difference moved by a Direction in a rank.
    ///
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_directions() {
        assert_eq!(Direction::NODIR + Direction::EAST, Direction::EAST);
        assert_eq!(Direction::EAST + Direction::WEST, Direction::NODIR);
    }

    #[test]
    fn test_opposite_directions() {
        assert_eq!(-Direction::EAST, Direction::WEST);
        assert_eq!(-Direction::NORTH, Direction::SOUTH);
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(Direction::NORTHEAST - Direction::EAST, Direction::NORTH);
        assert_eq!(Direction::EAST - Direction::EAST, Direction::NODIR);
    }
}
