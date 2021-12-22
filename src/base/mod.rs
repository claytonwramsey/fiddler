pub mod bitboard;
pub use crate::base::bitboard::Bitboard;

pub mod board;
pub use crate::base::board::Board;

mod castling;
pub use crate::base::castling::CastleRights;

pub mod constants;

pub mod direction;
pub use crate::base::direction::Direction;

pub mod fens;

pub mod game;
pub use crate::base::game::Game;

mod magic;
pub use crate::base::magic::{Magic, MagicTable};

pub mod moves;
pub use crate::base::moves::Move;

pub mod movegen;
pub use crate::base::movegen::MoveGenerator;

pub mod piece;
pub use crate::base::piece::PieceType;

pub mod square;
pub use crate::base::square::Square;

pub mod util;

/**
 * A module for storing the hashes of positions in a computationally efficient
 * manner.
 */
mod zobrist;

/**
 * A module for converting `Move`s to their algebraic notation form, as well as
 * converting algebraic notation strings to `Move`s.
 */
pub mod algebraic;