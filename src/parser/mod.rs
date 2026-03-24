//! Custom natural-language date parser for **TARDIS**.
//!
//! Pipeline: input string -> lexer (tokens) -> grammar (AST) -> resolver (Zoned).
//! Only [`parse`] and [`ParseError`] are public; all internals are `pub(crate)`.

pub(crate) mod ast;
pub(crate) mod error;
// lexer and grammar will be added in Plan 02 and Plan 03
pub(crate) mod suggest;
pub(crate) mod token;

pub use error::ParseError;

/// Maximum input length in bytes (UX-03). Inputs longer than this are rejected
/// before tokenization to prevent abuse.
const MAX_INPUT_LEN: usize = 1024;

/// Parse a natural-language date expression into a [`jiff::Zoned`] datetime.
///
/// * `input` -- the raw expression (e.g. `"next friday"`, `"@1735689600"`, `"in 3 days"`)
/// * `now` -- reference "now" for relative resolution
///
/// Returns the resolved datetime or a [`ParseError`] with span-based diagnostics.
pub fn parse(input: &str, now: &jiff::Zoned) -> std::result::Result<jiff::Zoned, ParseError> {
    // UX-03: Input length validation
    if input.len() > MAX_INPUT_LEN {
        return Err(ParseError::input_too_long(input.len(), MAX_INPUT_LEN));
    }

    // Stub: will be implemented in Plan 03
    // For now, return an error so the module compiles
    let _ = now;
    Err(ParseError::unrecognized(input))
}
