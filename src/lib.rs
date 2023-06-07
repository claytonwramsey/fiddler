//! # Fiddler: Clayton's little chess engine.
//!
//! Fiddler is a hobby chess engine created by Clayton Ramsey.
//! This code is not meant to be used as a library; however, I have tried very hard to make sure it
//! is well documented.
//!
//! For more details, refer to the README.

#![warn(clippy::cargo)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(incomplete_features)]
#![feature(adt_const_params)]

pub mod base;
pub mod engine;
