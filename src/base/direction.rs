/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Directions, which form a vector field describing motions between `Square`s.

use super::Square;
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// A difference between two squares. `Direction`s form a vector field, which allows us to define
/// subtraction between squares.
/// Internally, they use the same representation as a `Square` but with a signed integer.
pub struct Direction(pub(crate) i8);

impl Direction {
    /* Cardinal directions */

    /// A `Direction` corresponding to a movement from nowhere to nowhere.
    pub const NONE: Self = Self(0);

    /// A `Direction` corresponding to a move "north" from White's point of view, in the direction
    /// a white pawn would travel.
    pub const NORTH: Self = Self(8);

    /// A `Direction` corresponding to a move "east" from White's point of view.
    pub const EAST: Self = Self(1);

    // sadly, the nature of rust consts means the following doesn't work:
    // pub const SOUTH: Direction = -NORTH;

    /// A `Direction` corresponding to a move "south" from White's point of view.
    pub const SOUTH: Self = Self(-8);

    /// A `Direction` corresponding to a move "west" from White's point of view.
    pub const WEST: Self = Self(-1);

    /* Composite directions */

    /// A `Direction` corresponding to a move "southwest" from White's point of view.
    pub const NORTHWEST: Self = Self(Self::NORTH.0 + Self::WEST.0);
    /// A `Direction` corresponding to a move "northeast" from White's point of view.
    pub const NORTHEAST: Self = Self(Self::NORTH.0 + Self::EAST.0);
    /// A `Direction` corresponding to a move "southeast" from White's point of view.
    pub const SOUTHEAST: Self = Self(Self::SOUTH.0 + Self::EAST.0);
    /// A `Direction` corresponding to a move "southwest" from White's point of  view.
    pub const SOUTHWEST: Self = Self(Self::SOUTH.0 + Self::WEST.0);

    /* Knight directions */

    /// A `Direction` corresponding to a move "north-by-northwest" from White's point of view.
    pub const NNW: Self = Self(2 * Self::NORTH.0 + Self::WEST.0);

    /// A `Direction` corresponding to a move "north-by-northeast" from White's point of view.
    pub const NNE: Self = Self(2 * Self::NORTH.0 + Self::EAST.0);

    /// A `Direction` corresponding to a move "east-by-northeast" from White's point of view.
    pub const ENE: Self = Self(Self::NORTH.0 + 2 * Self::EAST.0);

    /// A `Direction` corresponding to a move "east-by-southeast" from White's point of view.
    pub const ESE: Self = Self(Self::SOUTH.0 + 2 * Self::EAST.0);

    /// A `Direction` corresponding to a move "south-by-southeast" from White's point of view.
    pub const SSE: Self = Self(2 * Self::SOUTH.0 + Self::EAST.0);

    /// A `Direction` corresponding to a move "south-by-southwest" from White's point of view.
    pub const SSW: Self = Self(2 * Self::SOUTH.0 + Self::WEST.0);

    /// A `Direction` corresponding to a move "west-by-southwest" from White's point of view.
    pub const WSW: Self = Self(Self::SOUTH.0 + 2 * Self::WEST.0);

    /// A `Direction` corresponding to a move "west-by-northwest" from White's point of view.
    pub const WNW: Self = Self(Self::NORTH.0 + 2 * Self::WEST.0);

    /// The directions that a rook can move, along only one step.
    pub const ROOK_DIRECTIONS: [Self; 4] = [
        Self::NORTH,
        Self::SOUTH,
        Self::EAST,
        Self::WEST,
    ];

    /// The directions that a bishop can move, along only one step.
    pub const BISHOP_DIRECTIONS: [Self; 4] = [
        Self::NORTHWEST,
        Self::NORTHEAST,
        Self::SOUTHWEST,
        Self::SOUTHEAST,
    ];

    /// The steps that a knight can make.
    pub const KNIGHT_STEPS: [Self; 8] = [
        Self::NNW,
        Self::NNE,
        Self::ENE,
        Self::ESE,
        Self::SSE,
        Self::SSW,
        Self::WSW,
        Self::WNW,
    ];

    /// The steps that a king can make.
    pub const KING_STEPS: [Self; 8] = [
        Self::NORTH,
        Self::NORTHEAST,
        Self::EAST,
        Self::SOUTHEAST,
        Self::SOUTH,
        Self::SOUTHWEST,
        Self::WEST,
        Self::NORTHWEST,
    ];

    #[must_use]
    /// Create a new Direction based on how far it moves in rank and file.
    pub const fn new(rank_step: i8, file_step: i8) -> Self {
        Self(rank_step + (file_step * 8))
    }
}

impl Neg for Direction {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
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

impl Add<Self> for Direction {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Self> for Direction {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_directions() {
        assert_eq!(Direction::NONE + Direction::EAST, Direction::EAST);
        assert_eq!(Direction::EAST + Direction::WEST, Direction::NONE);
    }

    #[test]
    fn opposite_directions() {
        assert_eq!(-Direction::EAST, Direction::WEST);
        assert_eq!(-Direction::NORTH, Direction::SOUTH);
    }

    #[test]
    fn subtraction() {
        assert_eq!(Direction::NORTHEAST - Direction::EAST, Direction::NORTH);
        assert_eq!(Direction::EAST - Direction::EAST, Direction::NONE);
    }

    #[test]
    fn direction_out_of_bounds() {
        let bad_sq = Square::A1 + Direction::SOUTH;
        println!("{bad_sq}");
    }
}
