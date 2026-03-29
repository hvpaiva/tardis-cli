//! Parser error types with span-based diagnostics.
//!
//! Errors carry the original input, byte-level position information, and
//! optional typo-correction suggestions.

use std::fmt;

use crate::parser::token::ByteSpan;

/// A parse error with optional span, expected/found context, and suggestion.
#[must_use]
#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    span: Option<ByteSpan>,
    input: String,
    suggestion: Option<String>,
}

#[derive(Debug)]
enum ParseErrorKind {
    UnexpectedToken { expected: String, found: String },
    UnrecognizedInput,
    ResolutionFailed(String),
    InputTooLong { len: usize, max: usize },
}

impl ParseError {
    /// Construct an error for unrecognized input.
    pub(crate) fn unrecognized(input: &str) -> Self {
        Self {
            kind: ParseErrorKind::UnrecognizedInput,
            span: None,
            input: input.to_string(),
            suggestion: None,
        }
    }

    /// Construct an error for unexpected token with position.
    pub(crate) fn unexpected(input: &str, span: ByteSpan, expected: &str, found: &str) -> Self {
        Self {
            kind: ParseErrorKind::UnexpectedToken {
                expected: expected.to_string(),
                found: found.to_string(),
            },
            span: Some(span),
            input: input.to_string(),
            suggestion: None,
        }
    }

    /// Construct an error for resolution failures (e.g., overflow).
    pub(crate) fn resolution(detail: String) -> Self {
        Self {
            kind: ParseErrorKind::ResolutionFailed(detail),
            span: None,
            input: String::new(),
            suggestion: None,
        }
    }

    /// Construct an error for input too long.
    pub(crate) fn input_too_long(len: usize, max: usize) -> Self {
        Self {
            kind: ParseErrorKind::InputTooLong { len, max },
            span: None,
            input: String::new(),
            suggestion: None,
        }
    }

    /// Attach a typo-correction suggestion.
    pub(crate) fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Access the typo-correction suggestion, if any.
    pub fn suggestion(&self) -> &Option<String> {
        &self.suggestion
    }

    /// Format the error message for display to the user.
    pub fn format_message(&self) -> String {
        let mut msg = match &self.kind {
            ParseErrorKind::UnexpectedToken { expected, found } => {
                if let Some(span) = &self.span {
                    format!(
                        "expected {} at position {}, found '{}'",
                        expected, span.start, found,
                    )
                } else {
                    format!("expected {}, found '{}'", expected, found)
                }
            }
            ParseErrorKind::UnrecognizedInput => {
                if self.input.is_empty() {
                    "could not parse as a date expression".to_string()
                } else {
                    format!("could not parse '{}' as a date expression", self.input)
                }
            }
            ParseErrorKind::ResolutionFailed(detail) => detail.clone(),
            ParseErrorKind::InputTooLong { len, max } => {
                format!("input too long ({len} bytes, max {max})")
            }
        };

        if let Some(suggestion) = &self.suggestion {
            msg.push_str(&format!("\n\nDid you mean '{suggestion}'?"));
        }

        msg
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_message())
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn unrecognized_error_message() {
        let err = ParseError::unrecognized("xyz");
        assert_eq!(
            err.format_message(),
            "could not parse 'xyz' as a date expression"
        );
    }

    #[test]
    fn unrecognized_empty_input_no_echo() {
        let err = ParseError::unrecognized("");
        assert_eq!(err.format_message(), "could not parse as a date expression");
    }

    #[test]
    fn unrecognized_with_suggestion_echoes_input_and_suggests() {
        let err = ParseError::unrecognized("tomorow").with_suggestion("tomorrow".to_string());
        let msg = err.format_message();
        assert!(msg.contains("could not parse 'tomorow'"));
        assert!(msg.contains("Did you mean 'tomorrow'?"));
    }

    #[test]
    fn suggestion_accessor_returns_value() {
        let err = ParseError::unrecognized("tomorow").with_suggestion("tomorrow".to_string());
        assert_eq!(err.suggestion(), &Some("tomorrow".to_string()));
    }

    #[test]
    fn suggestion_accessor_returns_none() {
        let err = ParseError::unrecognized("xyz");
        assert_eq!(err.suggestion(), &None);
    }

    #[test]
    fn unexpected_token_with_span() {
        let err =
            ParseError::unexpected("next 32", ByteSpan { start: 5, end: 7 }, "day name", "32");
        assert_eq!(
            err.format_message(),
            "expected day name at position 5, found '32'"
        );
    }

    #[test]
    fn input_too_long_message() {
        let err = ParseError::input_too_long(2048, 1024);
        assert_eq!(
            err.format_message(),
            "input too long (2048 bytes, max 1024)"
        );
    }

    #[test]
    fn error_with_suggestion() {
        let err = ParseError::unrecognized("thursdya").with_suggestion("thursday".to_string());
        assert!(err.format_message().contains("Did you mean 'thursday'?"));
    }

    #[test]
    fn suggestion_is_multiline_with_blank_separator() {
        let err = ParseError::unrecognized("tomorow").with_suggestion("tomorrow".to_string());
        let msg = err.format_message();
        let lines: Vec<&str> = msg.lines().collect();
        assert!(lines[0].contains("could not parse 'tomorow'"));
        assert_eq!(
            lines.len(),
            3,
            "Expected 3 lines: error, blank, suggestion. Got: {msg:?}"
        );
        assert!(lines[2].contains("Did you mean"));
    }

    #[test]
    fn suggestion_is_plain_text_no_ansi() {
        let err = ParseError::unrecognized("tomorow").with_suggestion("tomorrow".to_string());
        let msg = err.format_message();
        assert!(
            !msg.contains("\x1b["),
            "format_message() must not contain ANSI codes: {msg:?}"
        );
        assert!(msg.contains("Did you mean 'tomorrow'?"));
    }

    #[test]
    fn error_without_suggestion_has_no_trailing_blank_lines() {
        let err = ParseError::unrecognized("xyz");
        let msg = err.format_message();
        assert!(!msg.ends_with('\n'), "Message should not end with newline");
        assert!(
            !msg.contains("\n\n"),
            "Message should not contain double newlines"
        );
    }

    #[test]
    fn display_impl_matches_format_message() {
        let err = ParseError::unrecognized("@999999999999999999");
        assert_eq!(format!("{err}"), err.format_message());
    }

    #[test]
    fn resolution_failed_message() {
        let err = ParseError::resolution("overflow: date out of bounds".to_string());
        assert_eq!(err.format_message(), "overflow: date out of bounds");
    }
}
