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

/// Temporal duration unit.
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

/// Epoch timestamp precision levels.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochPrecision {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

/// TaskWarrior-style boundary keywords.
///
/// Current period (so/eo = start-of / end-of),
/// previous period (sop/eop), and next period (son/eon).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
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
/// Keywords are simple enum variants with zero heap allocation.
/// Only `Word(String)` carries owned data (for unrecognized words in error messages).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Now,
    Today,
    Tomorrow,
    Yesterday,
    Overmorrow,
    Ereyesterday,
    Next,
    Last,
    This,
    In,
    Ago,
    From,
    A,
    An,
    At,
    And,
    Am,
    Pm,
    Unit(TemporalUnit),
    Weekday(jiff::civil::Weekday),
    Month(i8),
    Number(i64),
    Colon,
    Dash,
    Slash,
    AtSign,
    Plus,
    After,
    Before,
    Quarter(i8),
    Boundary(BoundaryKind),
    EpochSuffix(EpochPrecision),
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
