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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// A difference between two squares. `Direction`s form a vector field, which allows us to define
/// subtraction between squares.
/// Internally, they use the same representation as a `Square` but with a signed integer.
pub struct Direction(pub(crate) i8);

impl Direction {
    /* Cardinal directions */

    /// A `Direction` corresponding to a move "north" from White's point of view, in the direction
    /// a white pawn would travel.
    pub const NORTH: Direction = Direction(8);

    /// A `Direction` corresponding to a move "east" from White's point of view.
    pub const EAST: Direction = Direction(1);

    // sadly, the nature of rust consts means the following doesn't work:
    // pub const SOUTH: Direction = -NORTH;

    /// A `Direction` corresponding to a move "south" from White's point of view.
    pub const SOUTH: Direction = Direction(-8);

    /// A `Direction` corresponding to a move "west" from White's point of view.
    pub const WEST: Direction = Direction(-1);

    /* Composite directions */

    /// The directions that a rook can move, along only one step.
    pub const ROOK_DIRECTIONS: [Direction; 4] = [
        Direction::NORTH,
        Direction::SOUTH,
        Direction::EAST,
        Direction::WEST,
    ];
}