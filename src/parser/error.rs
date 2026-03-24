//! Parser error types with span-based diagnostics.
//!
//! Errors carry the original input, byte-level position information, and
//! optional typo-correction suggestions (D-08).


use std::fmt;

use crate::parser::token::ByteSpan;

/// A parse error with optional span, expected/found context, and suggestion.
#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    span: Option<ByteSpan>,
    /// Original input string for error context (reserved for enhanced diagnostics).
    #[allow(dead_code)]
    input: String,
    suggestion: Option<String>,
}

#[derive(Debug)]
enum ParseErrorKind {
    UnexpectedToken { expected: String, found: String },
    UnrecognizedInput,
    /// Reserved for explicit epoch range errors (currently handled by ResolutionFailed).
    #[allow(dead_code)]
    EpochOutOfRange,
    ResolutionFailed(String),
    InputTooLong { len: usize, max: usize },
    Unsupported(String),
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
    pub(crate) fn unexpected(
        input: &str,
        span: ByteSpan,
        expected: &str,
        found: &str,
    ) -> Self {
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

    /// Construct an error for epoch timestamps out of range.
    #[allow(dead_code)]
    pub(crate) fn epoch_out_of_range(input: &str) -> Self {
        Self {
            kind: ParseErrorKind::EpochOutOfRange,
            span: None,
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

    /// Construct an error for input too long (UX-03).
    pub(crate) fn input_too_long(len: usize, max: usize) -> Self {
        Self {
            kind: ParseErrorKind::InputTooLong { len, max },
            span: None,
            input: String::new(),
            suggestion: None,
        }
    }

    /// Construct an unsupported-feature error (Phase 3 stubs).
    pub(crate) fn unsupported(what: &str) -> Self {
        Self {
            kind: ParseErrorKind::Unsupported(what.to_string()),
            span: None,
            input: String::new(),
            suggestion: None,
        }
    }

    /// Attach a typo-correction suggestion (D-08).
    pub(crate) fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
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
                "could not parse as a date expression".to_string()
            }
            ParseErrorKind::EpochOutOfRange => {
                "epoch timestamp out of range".to_string()
            }
            ParseErrorKind::ResolutionFailed(detail) => detail.clone(),
            ParseErrorKind::InputTooLong { len, max } => {
                format!("input too long ({len} bytes, max {max})")
            }
            ParseErrorKind::Unsupported(what) => what.clone(),
        };

        if let Some(suggestion) = &self.suggestion {
            msg.push_str(&format!(". Did you mean '{suggestion}'?"));
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
        assert_eq!(err.format_message(), "could not parse as a date expression");
    }

    #[test]
    fn unexpected_token_with_span() {
        let err = ParseError::unexpected(
            "next 32",
            ByteSpan { start: 5, end: 7 },
            "day name",
            "32",
        );
        assert_eq!(
            err.format_message(),
            "expected day name at position 5, found '32'"
        );
    }

    #[test]
    fn input_too_long_message() {
        let err = ParseError::input_too_long(2048, 1024);
        assert_eq!(err.format_message(), "input too long (2048 bytes, max 1024)");
    }

    #[test]
    fn error_with_suggestion() {
        let err =
            ParseError::unrecognized("thursdya").with_suggestion("thursday".to_string());
        assert!(err.format_message().contains("Did you mean 'thursday'?"));
    }

    #[test]
    fn display_impl_matches_format_message() {
        let err = ParseError::epoch_out_of_range("@999999999999999999");
        assert_eq!(format!("{err}"), err.format_message());
    }

    #[test]
    fn resolution_failed_message() {
        let err = ParseError::resolution("overflow: date out of bounds".to_string());
        assert_eq!(err.format_message(), "overflow: date out of bounds");
    }
}
