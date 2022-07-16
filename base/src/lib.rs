/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

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

//! Shared data types and useful basic definitions found across the entire
//! Fiddler engine.

// Many module elements are re-exported to make names more ergonomic to access.

pub mod algebraic;

mod bitboard;
pub use crate::bitboard::Bitboard;

mod board;
pub use board::Board;

mod castling;
use castling::CastleRights;

mod color;
pub use color::Color;

mod direction;
use direction::Direction;

mod eval;
pub use eval::{Eval, Score};

mod game;
pub use game::Game;

mod magic;
pub mod movegen;

mod moves;
pub use moves::Move;

pub mod perft;

mod piece;
pub use piece::Piece;

mod square;
pub use square::Square;

mod zobrist;
