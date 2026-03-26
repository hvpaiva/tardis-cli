//! Custom natural-language date parser for **TARDIS**.
//!
//! Pipeline: input string -> lexer (tokens) -> grammar (AST) -> resolver (Zoned).
//! Public submodules: [`ast`], [`token`], [`error`] (for library consumers).
//! Internal submodules: `grammar`, `lexer`, `resolver`, `suggest`.

pub mod ast;
pub mod error;
pub(crate) mod grammar;
pub(crate) mod lexer;
pub(crate) mod resolver;
pub(crate) mod suggest;
pub mod token;

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

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return resolver::resolve(&ast::DateExpr::Now, now);
    }

    // Try RFC 3339/ISO 8601 first (handles "2025-03-24T12:00:00Z" etc.)
    if let Ok(ts) = trimmed.parse::<jiff::Timestamp>() {
        return Ok(ts.to_zoned(now.time_zone().clone()));
    }

    let tokens = lexer::tokenize(trimmed);
    let mut parser = grammar::Parser::new(&tokens, trimmed);
    let expr = parser.parse_expression()?;
    resolver::resolve(&expr, now)
}

/// Parse any expression and resolve it as a range with implicit granularity (D-05).
///
/// This is the API used by the `td range` subcommand. Unlike [`parse_range`],
/// it accepts any expression type (not just Range variants) and applies
/// granularity expansion based on the smallest unspecified time unit:
///
/// - `"tomorrow"` -> day granularity (00:00:00..23:59:59)
/// - `"tomorrow at 18h"` -> hour granularity (18:00:00..18:59:59)
/// - `"tomorrow at 18:30"` -> minute granularity (18:30:00..18:30:59)
/// - `"now"` -> instant (now..now)
/// - `"this week"` -> week range (Monday..Sunday)
pub fn parse_range_with_granularity(
    input: &str,
    now: &jiff::Zoned,
) -> std::result::Result<(jiff::Zoned, jiff::Zoned), ParseError> {
    if input.len() > MAX_INPUT_LEN {
        return Err(ParseError::input_too_long(input.len(), MAX_INPUT_LEN));
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        let z = now.clone();
        return Ok((z.clone(), z));
    }

    // Try RFC 3339/ISO 8601 first (instant -> duplicated)
    if let Ok(ts) = trimmed.parse::<jiff::Timestamp>() {
        let z = ts.to_zoned(now.time_zone().clone());
        return Ok((z.clone(), z));
    }

    let tokens = lexer::tokenize(trimmed);
    let mut parser = grammar::Parser::new(&tokens, trimmed);
    let expr = parser.parse_expression()?;
    resolver::resolve_range_with_granularity(&expr, now)
}

/// Parse a range expression into a `(start, end)` pair of [`jiff::Zoned`] datetimes.
///
/// Range expressions like `"last week"`, `"this month"`, `"Q3 2025"` produce
/// two datetimes: the start (inclusive) and end (inclusive, 23:59:59.999999999).
///
/// Returns an error if the input is not a range expression. Use [`parse`] for
/// single-datetime expressions.
pub fn parse_range(
    input: &str,
    now: &jiff::Zoned,
) -> std::result::Result<(jiff::Zoned, jiff::Zoned), ParseError> {
    // UX-03: Input length validation
    if input.len() > MAX_INPUT_LEN {
        return Err(ParseError::input_too_long(input.len(), MAX_INPUT_LEN));
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::resolution(
            "expression is not a range".to_string(),
        ));
    }

    let tokens = lexer::tokenize(trimmed);
    let mut parser = grammar::Parser::new(&tokens, trimmed);
    let expr = parser.parse_expression()?;

    match expr {
        ast::DateExpr::Range(ref range) => resolver::resolve_range(range, now),
        _ => Err(ParseError::resolution(
            "expression is not a range".to_string(),
        )),
    }
}
