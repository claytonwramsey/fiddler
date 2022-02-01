///
/// A module containing the functionality of bitboards, which express a set of
/// squares on the board as a single 64-bit integer, which allows for O(1) set
/// operations.
///
pub mod bitboard;
pub use crate::base::bitboard::Bitboard;

///
/// A module containing all the functionality of a board. A board contains all
/// the necessary information to play a move in "the moment," but is memoryless:
///  it knows nothing about how the position on the board was reached. For a
/// game with memory, use `Game`.
///
pub mod board;
pub use crate::base::board::Board;

///
/// A module for managing castling rights.
///
mod castling;
pub use crate::base::castling::CastleRights;

///
/// The set of common constants used in chess.
///
pub mod constants;

///
/// A module for managing directions, which represent the differences between
/// two squares.
///
pub mod direction;
pub use crate::base::direction::Direction;

pub mod game;
pub use crate::base::game::Game;

mod magic;
pub use crate::base::magic::{Magic, MagicTable};

pub mod moves;
pub use crate::base::moves::Move;

pub mod movegen;
pub use crate::base::movegen::MoveGenerator;

pub mod piece;
pub use crate::base::piece::Piece;

pub mod square;
pub use crate::base::square::Square;

pub mod util;

///
/// A module for storing the hashes of positions in a computationally efficient
/// manner.
///
mod zobrist;

///
/// A module for converting `Move`s to their algebraic notation form, as well as
/// converting algebraic notation strings to `Move`s.
///
pub mod algebraic;

///
/// A module for the representation of players and their colors.
///
pub mod color;
pub use crate::base::color::Color;
