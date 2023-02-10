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
#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

//! Shared data types and useful basic definitions found across the entire Fiddler engine.

// Many module elements are re-exported to make names more ergonomic to access.

mod bitboard;
pub use bitboard::Bitboard;

mod board;
pub use board::Board;

mod castling;
use castling::CastleRights;

mod color;
pub use color::Color;

mod direction;
pub use direction::Direction;

pub mod game;

pub mod movegen;

mod moves;
pub use moves::Move;

mod piece;
pub use piece::Piece;

mod square;
pub use square::Square;

mod zobrist;
