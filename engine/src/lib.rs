pub mod base;
pub mod cli;
pub mod engine;
pub mod uci;

/// A module containing Forsyth-Edwards Notation (FEN) strings which are used
/// for tests.
mod fens;

#[cfg(feature = "tune")]
/// A module for tuning the engine.
pub mod tuning;
