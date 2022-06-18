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

mod position;
pub use position::Position;

mod square;
pub use square::Square;

mod zobrist;
