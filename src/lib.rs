pub mod base;
pub mod cli;
pub mod engine;
pub use crate::engine::Engine;

///
/// A module containing Forsyth-Edwards Notation (FEN) strings which are used
/// for tests.
///
mod fens;

///
/// A module for supporting the Universal Chess Interface (UCI).
///
pub mod uci;
