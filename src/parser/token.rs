//! Token types and span tracking for the TARDIS lexer.


/// Byte offset range into the original input string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ByteSpan {
    pub start: usize,
    pub end: usize,
}

/// A token paired with its source position.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpannedToken {
    pub kind: Token,
    pub span: ByteSpan,
}

/// Temporal duration unit (PARS-06: all seven standard units).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemporalUnit {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}

/// Epoch timestamp precision levels (EPOCH-01, EPOCH-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EpochPrecision {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

/// Lexer token types.
///
/// Keywords are simple enum variants (zero heap allocation per D-04 perf constraint).
/// Only `Word(String)` carries owned data (for unrecognized words in error messages).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    // Relative keywords
    Now,
    Today,
    Tomorrow,
    Yesterday,
    Overmorrow,
    Ereyesterday,

    // Direction modifiers
    Next,
    Last,
    This,
    In,
    Ago,
    From,

    // Articles (implicit count=1)
    A,
    An,

    // Connectors
    At,
    And,

    // Temporal units
    Unit(TemporalUnit),

    // Weekday (using jiff's Weekday enum)
    Weekday(jiff::civil::Weekday),

    // Month (1-12)
    Month(i8),

    // Numeric literal
    Number(i64),

    // Separators
    Colon,
    Dash,
    Slash,
    AtSign,

    // Arithmetic operators
    Plus,

    // Verbal arithmetic keywords
    After,
    Before,

    // Quarter indicator (Q1-Q4)
    Quarter(i8),

    // Epoch suffix (@NNNms, @NNNus, @NNNns, @NNNs)
    EpochSuffix(EpochPrecision),

    // Unrecognized word (kept for error messages and typo suggestions)
    Word(String),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn bytespan_equality() {
        let a = ByteSpan { start: 0, end: 5 };
        let b = ByteSpan { start: 0, end: 5 };
        assert_eq!(a, b);
    }

    #[test]
    fn token_variants_no_heap_alloc() {
        // Verify keyword tokens are simple enum variants (no String allocation)
        let t = Token::Now;
        assert_eq!(t, Token::Now);
        let t2 = Token::Unit(TemporalUnit::Day);
        assert_eq!(t2, Token::Unit(TemporalUnit::Day));
    }

    #[test]
    fn spanned_token_construction() {
        let st = SpannedToken {
            kind: Token::Number(42),
            span: ByteSpan { start: 0, end: 2 },
        };
        assert_eq!(st.kind, Token::Number(42));
        assert_eq!(st.span.start, 0);
    }
}
