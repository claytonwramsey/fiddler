use super::Square;
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// A difference between two squares. Directions form a vector field, which
/// allows us to define subtraction between squares. Internally, they use the
/// same representation as a Square but with a signed integer.
pub struct Direction(pub i8);

impl Direction {
    /* Cardinal directions */

    /// A `Direction` corresponding to a movement from nowhere to nowhere.
    pub const NODIR: Direction = Direction(0);

    /// A `Direction` corresponding to a move "north" from White's point of
    /// view, in the direction a white pawn would travel.
    pub const NORTH: Direction = Direction(8);

    /// A `Direction` corresponding to a move "east" from White's point of view.
    pub const EAST: Direction = Direction(1);

    //sadly, the nature of rust consts means this doesn't work
    //pub const SOUTH: Direction = -NORTH;

    /// A `Direction` corresponding to a move "south" from White's point of
    /// view.
    pub const SOUTH: Direction = Direction(-8);

    /// A `Direction` corresponding to a move "west" from White's point of view.
    pub const WEST: Direction = Direction(-1);

    /* Composite directions */

    /// A `Direction` corresponding to a move "southwest" from White's point of
    /// view.
    pub const NORTHWEST: Direction = Direction(Direction::NORTH.0 + Direction::WEST.0);
    /// A `Direction` corresponding to a move "northeast" from White's point of
    /// view.
    pub const NORTHEAST: Direction = Direction(Direction::NORTH.0 + Direction::EAST.0);
    /// A `Direction` corresponding to a move "southeast" from White's point of
    /// view.
    pub const SOUTHEAST: Direction = Direction(Direction::SOUTH.0 + Direction::EAST.0);
    /// A `Direction` corresponding to a move "southwest" from White's point of
    /// view.
    pub const SOUTHWEST: Direction = Direction(Direction::SOUTH.0 + Direction::WEST.0);

    /* Knight directions */

    /// A `Direction` corresponding to a move "north-by-northwest" from White's
    /// point of view.
    pub const NNW: Direction = Direction(2 * Direction::NORTH.0 + Direction::WEST.0);

    /// A `Direction` corresponding to a move "north-by-northeast" from White's
    /// point of view.
    pub const NNE: Direction = Direction(2 * Direction::NORTH.0 + Direction::EAST.0);

    /// A `Direction` corresponding to a move "east-by-northeast" from White's
    /// point of view.
    pub const ENE: Direction = Direction(Direction::NORTH.0 + 2 * Direction::EAST.0);

    /// A `Direction` corresponding to a move "east-by-southeast" from White's
    /// point of view.
    pub const ESE: Direction = Direction(Direction::SOUTH.0 + 2 * Direction::EAST.0);

    /// A `Direction` corresponding to a move "south-by-southeast" from White's
    /// point of view.
    pub const SSE: Direction = Direction(2 * Direction::SOUTH.0 + Direction::EAST.0);

    /// A `Direction` corresponding to a move "south-by-southwest" from White's
    /// point of view.
    pub const SSW: Direction = Direction(2 * Direction::SOUTH.0 + Direction::WEST.0);

    /// A `Direction` corresponding to a move "west-by-southwest" from White's
    /// point of view.
    pub const WSW: Direction = Direction(Direction::SOUTH.0 + 2 * Direction::WEST.0);

    /// A `Direction` corresponding to a move "west-by-northwest" from White's
    /// point of view.
    pub const WNW: Direction = Direction(Direction::NORTH.0 + 2 * Direction::WEST.0);

    /// The directions that a rook can move, along only one step.
    pub const ROOK_DIRECTIONS: [Direction; 4] = [
        Direction::NORTH,
        Direction::SOUTH,
        Direction::EAST,
        Direction::WEST,
    ];

    /// The directions that a bishop can move, along only one step.
    pub const BISHOP_DIRECTIONS: [Direction; 4] = [
        Direction::NORTHWEST,
        Direction::NORTHEAST,
        Direction::SOUTHWEST,
        Direction::SOUTHEAST,
    ];

    /// The steps that a knight can make.
    pub const KNIGHT_STEPS: [Direction; 8] = [
        Direction::NNW,
        Direction::NNE,
        Direction::ENE,
        Direction::ESE,
        Direction::SSE,
        Direction::SSW,
        Direction::WSW,
        Direction::WNW,
    ];

    /// The steps that a king can make.
    pub const KING_STEPS: [Direction; 8] = [
        Direction::NORTH,
        Direction::NORTHEAST,
        Direction::EAST,
        Direction::SOUTHEAST,
        Direction::SOUTH,
        Direction::SOUTHWEST,
        Direction::WEST,
        Direction::NORTHWEST,
    ];

    #[inline]
    /// Create a new Direction based on how far it moves in rank and file.
    pub const fn new(rank_step: i8, file_step: i8) -> Direction {
        Direction(rank_step + (file_step * 8))
    }

    #[inline]
    #[allow(dead_code)]
    /// Get the difference moved by a Direction in a file.
    const fn file_step(self) -> i8 {
        self.0 % 8
    }

    #[inline]
    #[allow(dead_code)]
    /// Get the difference moved by a Direction in a rank.
    const fn rank_step(self) -> i8 {
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

    #[test]
    fn test_direction_out_of_bounds() {
        let bad_sq = Square::A1 + Direction::SOUTH;
        println!("{}", bad_sq);
    }
}
