#![deny(clippy::unwrap_used, clippy::expect_used)]

pub mod cli;
pub mod config;
pub mod core;
pub mod errors;
pub mod locale;
pub mod parser;

pub use errors::{Error, Result};
