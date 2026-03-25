//! Custom natural-language date parser for **TARDIS**.
//!
//! Pipeline: input string -> lexer (tokens) -> grammar (AST) -> resolver (Zoned).
//! Only [`parse`] and [`ParseError`] are public; all internals are `pub(crate)`.

pub(crate) mod ast;
pub(crate) mod error;
pub(crate) mod grammar;
pub(crate) mod lexer;
pub(crate) mod resolver;
pub(crate) mod suggest;
pub(crate) mod token;

pub use error::ParseError;

use crate::locale::LocaleKeywords;

/// Maximum input length in bytes (UX-03). Inputs longer than this are rejected
/// before tokenization to prevent abuse.
const MAX_INPUT_LEN: usize = 1024;

/// Parse a natural-language date expression into a [`jiff::Zoned`] datetime.
///
/// * `input` -- the raw expression (e.g. `"next friday"`, `"@1735689600"`, `"in 3 days"`)
/// * `now` -- reference "now" for relative resolution
/// * `locale_keywords` -- locale-driven keyword table for tokenization
///
/// Returns the resolved datetime or a [`ParseError`] with span-based diagnostics.
pub fn parse(
    input: &str,
    now: &jiff::Zoned,
    locale_keywords: &LocaleKeywords,
) -> std::result::Result<jiff::Zoned, ParseError> {
    // UX-03: Input length validation
    if input.len() > MAX_INPUT_LEN {
        return Err(ParseError::input_too_long(input.len(), MAX_INPUT_LEN));
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return resolver::resolve(&ast::DateExpr::Now, now);
    }

    let tokens = lexer::tokenize(trimmed, locale_keywords);
    let kw_list = locale_keywords.all_keywords();
    let mut parser = grammar::Parser::new(&tokens, trimmed, &kw_list);
    let expr = parser.parse_expression()?;
    resolver::resolve(&expr, now)
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
    locale_keywords: &LocaleKeywords,
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

    let tokens = lexer::tokenize(trimmed, locale_keywords);
    let kw_list = locale_keywords.all_keywords();
    let mut parser = grammar::Parser::new(&tokens, trimmed, &kw_list);
    let expr = parser.parse_expression()?;

    match expr {
        ast::DateExpr::Range(ref range) => resolver::resolve_range(range, now),
        _ => Err(ParseError::resolution(
            "expression is not a range".to_string(),
        )),
    }
}
