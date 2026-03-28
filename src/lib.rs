//! **TARDIS** -- Time And Relative Date Input Simplifier.
//!
//! Library crate exposing the CLI argument types, configuration loader,
//! core transformation pipeline, natural-language parser, and error types.

#![deny(clippy::unwrap_used, clippy::expect_used)]

pub mod cli;
pub mod config;
pub mod core;
pub mod errors;
pub mod parser;

pub use errors::{Error, Result};
