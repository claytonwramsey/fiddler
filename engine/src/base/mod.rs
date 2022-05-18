/// A module for converting `Move`s to their algebraic notation form, as well as
/// converting algebraic notation strings to `Move`s.
pub mod algebraic;

/// A module containing the functionality of bitboards, which express a set of
/// squares on the board as a single 64-bit integer, which allows for O(1) set
/// operations.
mod bitboard;
pub use crate::base::bitboard::Bitboard;

/// A module containing all the functionality of a board. A board contains all
/// the necessary information to play a move in "the moment," but is memoryless:
/// it knows nothing about how the position on the board was reached. For a
/// game with memory, use `Game`.
mod board;
pub use board::Board;

/// A module for managing castling rights.
mod castling;
pub use castling::CastleRights;

/// A module for the representation of players and their colors.
mod color;
pub use color::Color;

/// A module for managing directions, which represent the differences between
/// two squares.
pub mod direction;
pub use direction::Direction;

/// A module containing information for storing evaluations of positions.
mod eval;
pub use eval::Eval;
pub use eval::Score;

/// A module containing a game, with the history of positions as well as their
/// current state.
mod game;
pub use game::Game;

/// A module for magic bitboards, used in move generation in bishops, rooks,
/// and queens.
mod magic;
pub use magic::MagicTable;

/// A module used for generating legal moves on a position.
pub mod movegen;

/// A module used for defining moves.
mod moves;
pub use moves::Move;

/// A performance testing module to record the speed of move generation.
pub mod perft;

/// A module used for defining piece types.
mod piece;
pub use piece::Piece;

/// A module for storing a position, which is a board as well as useful
/// metadata about a board.
mod position;
pub use position::Position;

/// A module used for defining squares.
mod square;
pub use square::Square;

/// A module for storing the hashes of positions in a computationally efficient
/// manner.
mod zobrist;
