mod bitboard;
pub use crate::bitboard::Bitboard;

mod board;
pub use crate::board::Board;

mod castling;
pub use crate::castling::CastleRights;

mod constants;

mod direction;
pub use crate::direction::Direction;

pub mod engine;
pub use crate::engine::Engine;

pub mod fens;

mod game;
pub use crate::game::Game;

mod magic;
pub use crate::magic::{Magic, MagicTable};

mod moves;
pub use crate::moves::Move;

mod movegen;
pub use crate::movegen::MoveGenerator;

mod piece;
pub use crate::piece::PieceType;

mod square;
pub use crate::square::Square;

mod util;

/**
 * A module for storing the hashes of positions in a computationally efficient
 * manner.
 */
mod zobrist;

/**
 * A module for converting `Move`s to their algebraic notation form, as well as
 * converting algebraic notation strings to `Move`s.
 */
mod algebraic;

pub mod cli;
