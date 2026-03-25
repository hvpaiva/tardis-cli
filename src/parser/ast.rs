//! Abstract syntax tree for parsed date expressions.
//!
//! The AST separates syntax (what the user typed) from semantics (what datetime
//! it resolves to). The resolver in `resolver.rs` maps these nodes to `jiff::Zoned`.


use crate::parser::token::{EpochPrecision, TemporalUnit};

/// Top-level AST node representing a parsed date expression.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DateExpr {
    // Phase 2: Fully implemented
    /// "now" or empty input
    Now,
    /// "today", "tomorrow", "yesterday", "overmorrow" with optional time
    Relative(RelativeDate, Option<TimeExpr>),
    /// "next/last/this friday" with optional time
    DayRef(Direction, jiff::civil::Weekday, Option<TimeExpr>),
    /// "2025-01-01", "24 March 2025" with optional time
    Absolute(AbsoluteDate, Option<TimeExpr>),
    /// "15:30" (time only, resolved against today)
    TimeOnly(TimeExpr),
    /// "@1735689600", "@1735689600ms"
    Epoch(EpochValue),
    /// "in 3 days", "3 hours ago"
    Offset(Direction, Vec<DurationComponent>),
    /// "3 hours ago from next friday"
    OffsetFrom(Direction, Vec<DurationComponent>, Box<DateExpr>),

    // Phase 3: Arithmetic and range expressions
    /// "tomorrow + 3 hours" -- compound arithmetic
    Arithmetic(Box<DateExpr>, ArithOp, Vec<DurationComponent>),
    /// "last week", "this month", "Q3 2025" -- range expressions
    Range(RangeExpr),
}

/// Named relative date variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelativeDate {
    Today,
    Tomorrow,
    Yesterday,
    Overmorrow,
}

/// Direction for day references and duration offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Next,
    Last,
    This,
    Future, // "in N ..."
    Past,   // "N ... ago"
}

/// A single duration component (e.g., "3 hours" -> count=3, unit=Hour).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DurationComponent {
    pub count: i64,
    pub unit: TemporalUnit,
}

/// Time expression (hours:minutes or hours:minutes:seconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeExpr {
    HourMinute(i8, i8),
    HourMinuteSecond(i8, i8, i8),
}

/// Absolute date components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbsoluteDate {
    pub year: i16,
    pub month: i8,
    pub day: i8,
}

/// Epoch value with precision.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EpochValue {
    pub raw: i64,
    pub precision: EpochPrecision,
}

/// Arithmetic operation for compound date expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArithOp {
    Add,
    Sub,
}

/// Range expression types for date range queries.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RangeExpr {
    LastWeek,
    ThisWeek,
    NextWeek,
    LastMonth,
    ThisMonth,
    NextMonth,
    LastYear,
    ThisYear,
    NextYear,
    /// Quarter(year, quarter_number). year=0 is sentinel for "current year".
    Quarter(i16, i8),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn date_expr_relative_with_time() {
        let expr = DateExpr::Relative(
            RelativeDate::Tomorrow,
            Some(TimeExpr::HourMinute(15, 30)),
        );
        assert!(matches!(
            expr,
            DateExpr::Relative(RelativeDate::Tomorrow, Some(_))
        ));
    }

    #[test]
    fn duration_component_construction() {
        let dc = DurationComponent {
            count: 3,
            unit: TemporalUnit::Hour,
        };
        assert_eq!(dc.count, 3);
        assert_eq!(dc.unit, TemporalUnit::Hour);
    }

    #[test]
    fn epoch_value_construction() {
        let ev = EpochValue {
            raw: 1735689600,
            precision: EpochPrecision::Seconds,
        };
        assert_eq!(ev.raw, 1735689600);
        assert_eq!(ev.precision, EpochPrecision::Seconds);
    }

    #[test]
    fn phase3_extension_types_exist() {
        // Verify Phase 3 types compile (D-06)
        let _ = DateExpr::Arithmetic(
            Box::new(DateExpr::Now),
            ArithOp::Add,
            vec![DurationComponent {
                count: 1,
                unit: TemporalUnit::Day,
            }],
        );
        let _ = DateExpr::Range(RangeExpr::LastWeek);
    }
}
