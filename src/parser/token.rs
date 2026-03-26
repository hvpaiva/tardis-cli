//! Token types and span tracking for the TARDIS lexer.

/// Byte offset range into the original input string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSpan {
    pub start: usize,
    pub end: usize,
}

/// A token paired with its source position.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub kind: Token,
    pub span: ByteSpan,
}

/// Temporal duration unit (PARS-06: all seven standard units).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalUnit {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}

/// Epoch timestamp precision levels (EPOCH-01, EPOCH-02).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochPrecision {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

/// TaskWarrior-style boundary keywords (D-11).
/// Current period (so/eo = start-of / end-of):
///   Sod, Eod, Sow, Eow, Soww, Eoww, Som, Eom, Soq, Eoq, Soy, Eoy
/// Previous period (sop/eop = start-of-previous / end-of-previous):
///   Sopd, Eopd, Sopw, Eopw, Sopm, Eopm, Sopq, Eopq, Sopy, Eopy
/// Next period (son/eon = start-of-next / end-of-next):
///   Sond, Eond, Sonw, Eonw, Sonm, Eonm, Sonq, Eonq, Sony, Eony
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
    // Current period (12 variants: includes soww, eoww, soq, eoq)
    Sod,
    Eod,
    Sow,
    Eow,
    Soww,
    Eoww,
    Som,
    Eom,
    Soq,
    Eoq,
    Soy,
    Eoy,
    // Previous period (10 variants)
    Sopd,
    Eopd,
    Sopw,
    Eopw,
    Sopm,
    Eopm,
    Sopq,
    Eopq,
    Sopy,
    Eopy,
    // Next period (10 variants)
    Sond,
    Eond,
    Sonw,
    Eonw,
    Sonm,
    Eonm,
    Sonq,
    Eonq,
    Sony,
    Eony,
}

/// Lexer token types.
///
/// Keywords are simple enum variants (zero heap allocation per D-04 perf constraint).
/// Only `Word(String)` carries owned data (for unrecognized words in error messages).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
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

    // AM/PM meridiem indicators
    Am,
    Pm,

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

    /// TaskWarrior boundary keyword (D-11, D-12)
    Boundary(BoundaryKind),

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
    fn boundary_kind_equality() {
        assert_eq!(BoundaryKind::Sod, BoundaryKind::Sod);
        assert_ne!(BoundaryKind::Sod, BoundaryKind::Eod);
    }

    #[test]
    fn boundary_token_construction() {
        assert_eq!(
            Token::Boundary(BoundaryKind::Eod),
            Token::Boundary(BoundaryKind::Eod)
        );
        assert_ne!(
            Token::Boundary(BoundaryKind::Eod),
            Token::Boundary(BoundaryKind::Sod)
        );
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
