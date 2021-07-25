use std::ops::{Neg, Add};
use crate::square::Square;

#[derive(Copy, Clone, Debug)]
pub struct Direction(i8);

impl Direction {
    pub fn value(self) -> i8 {
        self.0
    }
}

impl Neg for Direction {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Direction(-self.0)
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

}