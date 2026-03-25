//! AST resolver: maps DateExpr nodes to concrete jiff::Zoned datetimes.
//!
//! This module is a pure function of AST + reference time. No parsing logic.
//! All datetime arithmetic uses jiff's native calendar-aware operations.
//!
//! ## Clamping policy (PARS-07)
//!
//! jiff's `checked_add`/`checked_sub` clamps to end-of-month for calendar
//! unit arithmetic (e.g., Jan 31 + 1 month = Feb 28). This is intentional
//! and matches Python dateutil, JS Temporal, and Go `time.AddDate` behavior.
//! Non-reversibility is inherent: `Jan 31 + 1 month = Feb 28`, but
//! `Feb 28 - 1 month = Jan 28` (not Jan 31).

use jiff::{Span, Zoned, civil};

use crate::parser::{
    ast::*,
    error::ParseError,
    token::{EpochPrecision, TemporalUnit},
};

/// Resolve an AST node to a concrete `jiff::Zoned` datetime.
pub(crate) fn resolve(expr: &DateExpr, now: &Zoned) -> Result<Zoned, ParseError> {
    match expr {
        DateExpr::Now => Ok(now.clone()),
        DateExpr::Relative(rel, time) => resolve_relative(rel, time, now),
        DateExpr::DayRef(dir, weekday, time) => resolve_day_ref(dir, weekday, time, now),
        DateExpr::Absolute(abs, time) => resolve_absolute(abs, time, now),
        DateExpr::TimeOnly(time) => resolve_time_only(time, now),
        DateExpr::Epoch(epoch) => resolve_epoch(epoch, now.time_zone()),
        DateExpr::Offset(dir, comps) => resolve_offset(dir, comps, now),
        DateExpr::OffsetFrom(dir, comps, base) => resolve_offset_from(dir, comps, base, now),
        DateExpr::Arithmetic(base, op, comps) => resolve_arithmetic(base, op, comps, now),
        DateExpr::Range(..) => Err(ParseError::resolution(
            "range expressions produce (start, end) pairs; use parse_range() instead".to_string(),
        )),
    }
}

/// Resolve relative dates: today/tomorrow/yesterday/overmorrow at midnight,
/// or at the specified time if provided.
fn resolve_relative(
    rel: &RelativeDate,
    time: &Option<TimeExpr>,
    now: &Zoned,
) -> Result<Zoned, ParseError> {
    let today = now.date();

    let target_date = match rel {
        RelativeDate::Today => today,
        RelativeDate::Tomorrow => today
            .checked_add(Span::new().days(1))
            .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?,
        RelativeDate::Yesterday => today
            .checked_sub(Span::new().days(1))
            .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?,
        RelativeDate::Overmorrow => today
            .checked_add(Span::new().days(2))
            .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?,
        RelativeDate::Ereyesterday => today
            .checked_sub(Span::new().days(2))
            .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?,
    };

    let civil_dt = apply_time_or_midnight(target_date, time);
    let tz = now.time_zone().clone();
    tz.to_ambiguous_zoned(civil_dt)
        .compatible()
        .map_err(|e| ParseError::resolution(format!("ambiguous datetime: {e}")))
}

/// Resolve day references: next/last/this <weekday> at midnight or given time.
///
/// Weekday delta computation:
/// - Next: advance forward, skip today -> `(target - current + 7) % 7; if 0 then 7`
/// - Last: go backward, skip today -> `(current - target + 7) % 7; if 0 then 7`
/// - This: within current week, today = 0 -> `(target - current + 7) % 7`
fn resolve_day_ref(
    dir: &Direction,
    weekday: &jiff::civil::Weekday,
    time: &Option<TimeExpr>,
    now: &Zoned,
) -> Result<Zoned, ParseError> {
    let today = now.date();
    let current_wd = today.weekday();

    // Use Monday-zero offsets for arithmetic
    let current_offset = current_wd.to_monday_zero_offset() as i32;
    let target_offset = weekday.to_monday_zero_offset() as i32;

    let delta_days: i32 = match dir {
        Direction::Next => {
            let d = (target_offset - current_offset + 7) % 7;
            if d == 0 { 7 } else { d }
        }
        Direction::Last => {
            let d = (current_offset - target_offset + 7) % 7;
            if d == 0 { -7 } else { -d }
        }
        Direction::This => (target_offset - current_offset + 7) % 7,
        _ => {
            return Err(ParseError::resolution(format!(
                "unexpected direction {dir:?} for day reference"
            )));
        }
    };

    let target_date = today
        .checked_add(Span::new().days(i64::from(delta_days)))
        .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;

    let civil_dt = apply_time_or_midnight(target_date, time);
    let tz = now.time_zone().clone();
    tz.to_ambiguous_zoned(civil_dt)
        .compatible()
        .map_err(|e| ParseError::resolution(format!("ambiguous datetime: {e}")))
}

/// Resolve absolute dates with optional time.
/// If `year == 0` (sentinel for "year not specified"), uses `now.year()`.
fn resolve_absolute(
    abs: &AbsoluteDate,
    time: &Option<TimeExpr>,
    now: &Zoned,
) -> Result<Zoned, ParseError> {
    let year = if abs.year == 0 {
        now.date().year()
    } else {
        abs.year
    };

    let date = civil::date(year, abs.month, abs.day);
    let civil_dt = apply_time_or_midnight(date, time);
    let tz = now.time_zone().clone();
    tz.to_ambiguous_zoned(civil_dt)
        .compatible()
        .map_err(|e| ParseError::resolution(format!("ambiguous datetime: {e}")))
}

/// Resolve time-only expressions against today's date from `now`.
fn resolve_time_only(time: &TimeExpr, now: &Zoned) -> Result<Zoned, ParseError> {
    let today = now.date();
    let civil_dt = apply_time(today, time);
    let tz = now.time_zone().clone();
    tz.to_ambiguous_zoned(civil_dt)
        .compatible()
        .map_err(|e| ParseError::resolution(format!("ambiguous datetime: {e}")))
}

/// Resolve epoch timestamps, dispatching on precision.
fn resolve_epoch(epoch: &EpochValue, tz: &jiff::tz::TimeZone) -> Result<Zoned, ParseError> {
    let timestamp = match epoch.precision {
        EpochPrecision::Seconds => jiff::Timestamp::from_second(epoch.raw),
        EpochPrecision::Milliseconds => jiff::Timestamp::from_millisecond(epoch.raw),
        EpochPrecision::Microseconds => jiff::Timestamp::from_microsecond(epoch.raw),
        EpochPrecision::Nanoseconds => jiff::Timestamp::from_nanosecond(epoch.raw as i128),
    };
    let ts = timestamp
        .map_err(|e| ParseError::resolution(format!("epoch timestamp out of range: {e}")))?;
    Ok(ts.to_zoned(tz.clone()))
}

/// Resolve duration offsets relative to `now`.
/// Preserves the current time (unlike relative dates which snap to midnight).
fn resolve_offset(
    dir: &Direction,
    comps: &[DurationComponent],
    now: &Zoned,
) -> Result<Zoned, ParseError> {
    let span = build_span(comps);
    match dir {
        Direction::Future => now
            .checked_add(span)
            .map_err(|e| ParseError::resolution(format!("overflow: {e}"))),
        Direction::Past => now
            .checked_sub(span)
            .map_err(|e| ParseError::resolution(format!("overflow: {e}"))),
        _ => Err(ParseError::resolution(format!(
            "unexpected direction {dir:?} for offset"
        ))),
    }
}

/// Resolve arithmetic expressions: base +/- duration components.
/// Handles chained arithmetic recursively since
/// `Arithmetic(Arithmetic(base, op1, c1), op2, c2)` resolves the inner first.
fn resolve_arithmetic(
    base: &DateExpr,
    op: &ArithOp,
    comps: &[DurationComponent],
    now: &Zoned,
) -> Result<Zoned, ParseError> {
    let base_zoned = resolve(base, now)?;
    let span = build_span(comps);
    match op {
        ArithOp::Add => base_zoned
            .checked_add(span)
            .map_err(|e| ParseError::resolution(format!("overflow: {e}"))),
        ArithOp::Sub => base_zoned
            .checked_sub(span)
            .map_err(|e| ParseError::resolution(format!("overflow: {e}"))),
    }
}

/// Resolve range expressions to (start, end) datetime pairs.
///
/// - Monday as week start (ISO 8601, D-08)
/// - End boundaries are inclusive: 23:59:59.999999999 (D-09/Pitfall 3)
/// - All boundaries use `compatible()` for DST safety
pub(crate) fn resolve_range(range: &RangeExpr, now: &Zoned) -> Result<(Zoned, Zoned), ParseError> {
    let tz = now.time_zone().clone();
    let today = now.date();

    match range {
        RangeExpr::LastWeek => {
            let current_wd = today.weekday().to_monday_zero_offset() as i32;
            // Go to this Monday, then back one week
            let this_monday = today
                .checked_sub(Span::new().days(i64::from(current_wd)))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let last_monday = this_monday
                .checked_sub(Span::new().weeks(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let last_sunday = last_monday
                .checked_add(Span::new().days(6))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            Ok((
                zoned_midnight(last_monday, &tz)?,
                zoned_end_of_day(last_sunday, &tz)?,
            ))
        }
        RangeExpr::ThisWeek => {
            let current_wd = today.weekday().to_monday_zero_offset() as i32;
            let this_monday = today
                .checked_sub(Span::new().days(i64::from(current_wd)))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let this_sunday = this_monday
                .checked_add(Span::new().days(6))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            Ok((
                zoned_midnight(this_monday, &tz)?,
                zoned_end_of_day(this_sunday, &tz)?,
            ))
        }
        RangeExpr::NextWeek => {
            let current_wd = today.weekday().to_monday_zero_offset() as i32;
            let this_monday = today
                .checked_sub(Span::new().days(i64::from(current_wd)))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let next_monday = this_monday
                .checked_add(Span::new().weeks(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let next_sunday = next_monday
                .checked_add(Span::new().days(6))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            Ok((
                zoned_midnight(next_monday, &tz)?,
                zoned_end_of_day(next_sunday, &tz)?,
            ))
        }
        RangeExpr::LastMonth => {
            let (year, month) = prev_month(today.year(), today.month());
            month_range(year, month, &tz)
        }
        RangeExpr::ThisMonth => month_range(today.year(), today.month(), &tz),
        RangeExpr::NextMonth => {
            let (year, month) = next_month(today.year(), today.month());
            month_range(year, month, &tz)
        }
        RangeExpr::LastYear => year_range(today.year() - 1, &tz),
        RangeExpr::ThisYear => year_range(today.year(), &tz),
        RangeExpr::NextYear => year_range(today.year() + 1, &tz),
        RangeExpr::Quarter(year, q) => {
            let actual_year = if *year == 0 { today.year() } else { *year };
            quarter_range(actual_year, *q, &tz)
        }
    }
}

/// Create a Zoned at midnight (00:00:00.000000000) for the given date.
fn zoned_midnight(date: civil::Date, tz: &jiff::tz::TimeZone) -> Result<Zoned, ParseError> {
    let dt = date.at(0, 0, 0, 0);
    tz.to_ambiguous_zoned(dt)
        .compatible()
        .map_err(|e| ParseError::resolution(format!("ambiguous datetime: {e}")))
}

/// Create a Zoned at end of day (23:59:59.999999999) for the given date.
fn zoned_end_of_day(date: civil::Date, tz: &jiff::tz::TimeZone) -> Result<Zoned, ParseError> {
    let dt = date.at(23, 59, 59, 999_999_999);
    tz.to_ambiguous_zoned(dt)
        .compatible()
        .map_err(|e| ParseError::resolution(format!("ambiguous datetime: {e}")))
}

/// Compute month range: first day at midnight to last day at end of day.
fn month_range(
    year: i16,
    month: i8,
    tz: &jiff::tz::TimeZone,
) -> Result<(Zoned, Zoned), ParseError> {
    let first = civil::date(year, month, 1);
    let days = first.days_in_month();
    let last = civil::date(year, month, days);
    Ok((zoned_midnight(first, tz)?, zoned_end_of_day(last, tz)?))
}

/// Compute year range: Jan 1 at midnight to Dec 31 at end of day.
fn year_range(year: i16, tz: &jiff::tz::TimeZone) -> Result<(Zoned, Zoned), ParseError> {
    let first = civil::date(year, 1, 1);
    let last = civil::date(year, 12, 31);
    Ok((zoned_midnight(first, tz)?, zoned_end_of_day(last, tz)?))
}

/// Compute quarter range.
/// Q1=Jan-Mar, Q2=Apr-Jun, Q3=Jul-Sep, Q4=Oct-Dec.
fn quarter_range(year: i16, q: i8, tz: &jiff::tz::TimeZone) -> Result<(Zoned, Zoned), ParseError> {
    let (start_month, end_month) = match q {
        1 => (1, 3),
        2 => (4, 6),
        3 => (7, 9),
        4 => (10, 12),
        _ => {
            return Err(ParseError::resolution(format!(
                "invalid quarter number: {q}"
            )));
        }
    };
    let first = civil::date(year, start_month, 1);
    let end_date = civil::date(year, end_month, 1);
    let last = civil::date(year, end_month, end_date.days_in_month());
    Ok((zoned_midnight(first, tz)?, zoned_end_of_day(last, tz)?))
}

/// Get previous month (year, month) handling year boundary.
fn prev_month(year: i16, month: i8) -> (i16, i8) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

/// Get next month (year, month) handling year boundary.
fn next_month(year: i16, month: i8) -> (i16, i8) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

/// Resolve "N ago from <base>" by first resolving the base, then applying offset.
fn resolve_offset_from(
    dir: &Direction,
    comps: &[DurationComponent],
    base: &DateExpr,
    now: &Zoned,
) -> Result<Zoned, ParseError> {
    let base_zoned = resolve(base, now)?;
    resolve_offset(dir, comps, &base_zoned)
}

/// Build a `jiff::Span` from a list of duration components.
fn build_span(comps: &[DurationComponent]) -> Span {
    let mut span = Span::new();
    for comp in comps {
        span = match comp.unit {
            TemporalUnit::Year => span.years(comp.count),
            TemporalUnit::Month => span.months(comp.count),
            TemporalUnit::Week => span.weeks(comp.count),
            TemporalUnit::Day => span.days(comp.count),
            TemporalUnit::Hour => span.hours(comp.count),
            TemporalUnit::Minute => span.minutes(comp.count),
            TemporalUnit::Second => span.seconds(comp.count),
        };
    }
    span
}

/// Apply a TimeExpr to a date, or use midnight.
fn apply_time_or_midnight(date: civil::Date, time: &Option<TimeExpr>) -> civil::DateTime {
    match time {
        Some(t) => apply_time(date, t),
        None => date.at(0, 0, 0, 0),
    }
}

/// Apply a TimeExpr to a date.
fn apply_time(date: civil::Date, time: &TimeExpr) -> civil::DateTime {
    match time {
        TimeExpr::HourMinute(h, m) => date.at(*h, *m, 0, 0),
        TimeExpr::HourMinuteSecond(h, m, s) => date.at(*h, *m, *s, 0),
    }
}

// detect_epoch_precision lives in grammar.rs where it is called during parsing.

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::locale::{LocaleKeywords, en::EN_LOCALE};
    use jiff::{civil::Weekday, tz::TimeZone};

    fn utc() -> TimeZone {
        TimeZone::get("UTC").unwrap()
    }

    fn make_now() -> Zoned {
        let dt = civil::date(2025, 6, 15).at(12, 0, 0, 0);
        utc().to_ambiguous_zoned(dt).compatible().unwrap()
    }

    fn format_zoned(z: &Zoned) -> String {
        z.strftime("%Y-%m-%dT%H:%M:%S").to_string()
    }

    fn en_kw() -> LocaleKeywords {
        LocaleKeywords::from_locale(&EN_LOCALE)
    }

    #[test]
    fn resolve_now() {
        let now = make_now();
        let result = resolve(&DateExpr::Now, &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T12:00:00");
    }

    #[test]
    fn resolve_today_midnight() {
        let now = make_now();
        let result = resolve(&DateExpr::Relative(RelativeDate::Today, None), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T00:00:00");
    }

    #[test]
    fn resolve_tomorrow() {
        let now = make_now();
        let result = resolve(&DateExpr::Relative(RelativeDate::Tomorrow, None), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn resolve_yesterday() {
        let now = make_now();
        let result = resolve(&DateExpr::Relative(RelativeDate::Yesterday, None), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-14T00:00:00");
    }

    #[test]
    fn resolve_overmorrow() {
        let now = make_now();
        let result = resolve(&DateExpr::Relative(RelativeDate::Overmorrow, None), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-17T00:00:00");
    }

    #[test]
    fn resolve_today_with_time() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Relative(RelativeDate::Today, Some(TimeExpr::HourMinute(18, 30))),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T18:30:00");
    }

    #[test]
    fn resolve_next_friday_on_sunday() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Friday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-20T00:00:00");
    }

    #[test]
    fn resolve_next_sunday_on_sunday_advances_7() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Sunday, None),
            &now,
        )
        .unwrap();
        // Pitfall 3: "next sunday" on Sunday advances to next week
        assert_eq!(format_zoned(&result), "2025-06-22T00:00:00");
    }

    #[test]
    fn resolve_this_sunday_on_sunday_is_today() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::This, Weekday::Sunday, None),
            &now,
        )
        .unwrap();
        // Pitfall 3: "this sunday" on Sunday returns today
        assert_eq!(format_zoned(&result), "2025-06-15T00:00:00");
    }

    #[test]
    fn resolve_last_sunday_on_sunday_goes_back_7() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::Last, Weekday::Sunday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-08T00:00:00");
    }

    #[test]
    fn resolve_offset_future_3_days() {
        let now = make_now(); // Sunday June 15 at 12:00
        let result = resolve(
            &DateExpr::Offset(
                Direction::Future,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Day,
                }],
            ),
            &now,
        )
        .unwrap();
        // Preserves time for offsets
        assert_eq!(format_zoned(&result), "2025-06-18T12:00:00");
    }

    #[test]
    fn resolve_offset_past_1_hour() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Offset(
                Direction::Past,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Hour,
                }],
            ),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T11:00:00");
    }

    #[test]
    fn resolve_epoch_seconds() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Epoch(EpochValue {
                raw: 1_735_689_600,
                precision: EpochPrecision::Seconds,
            }),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-01-01T00:00:00");
    }

    #[test]
    fn resolve_epoch_negative() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Epoch(EpochValue {
                raw: -86400,
                precision: EpochPrecision::Seconds,
            }),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "1969-12-31T00:00:00");
    }

    // detect_epoch_precision tests are in grammar.rs where the function lives.

    #[test]
    fn resolve_absolute_iso_date() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Absolute(
                AbsoluteDate {
                    year: 2025,
                    month: 1,
                    day: 1,
                },
                None,
            ),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-01-01T00:00:00");
    }

    #[test]
    fn resolve_absolute_with_time() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Absolute(
                AbsoluteDate {
                    year: 2022,
                    month: 11,
                    day: 7,
                },
                Some(TimeExpr::HourMinuteSecond(13, 25, 30)),
            ),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2022-11-07T13:25:30");
    }

    #[test]
    fn resolve_absolute_year_sentinel() {
        let now = make_now(); // 2025-06-15
        let result = resolve(
            &DateExpr::Absolute(
                AbsoluteDate {
                    year: 0, // sentinel
                    month: 3,
                    day: 24,
                },
                None,
            ),
            &now,
        )
        .unwrap();
        // Uses current year (2025)
        assert_eq!(format_zoned(&result), "2025-03-24T00:00:00");
    }

    #[test]
    fn resolve_time_only() {
        let now = make_now(); // 2025-06-15
        let result = resolve(&DateExpr::TimeOnly(TimeExpr::HourMinute(15, 30)), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T15:30:00");
    }

    #[test]
    fn resolve_offset_a_week() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Offset(
                Direction::Past,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Week,
                }],
            ),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-08T12:00:00");
    }

    #[test]
    fn resolve_offset_a_month() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Offset(
                Direction::Past,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Month,
                }],
            ),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-05-15T12:00:00");
    }

    #[test]
    fn resolve_offset_a_year() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Offset(
                Direction::Past,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Year,
                }],
            ),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2024-06-15T12:00:00");
    }

    #[test]
    fn resolve_offset_from_base() {
        let now = make_now(); // Sunday June 15 at 12:00
        let result = resolve(
            &DateExpr::OffsetFrom(
                Direction::Past,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Hour,
                }],
                Box::new(DateExpr::Relative(RelativeDate::Tomorrow, None)),
            ),
            &now,
        )
        .unwrap();
        // tomorrow = June 16 at 00:00, minus 3 hours = June 15 at 21:00
        assert_eq!(format_zoned(&result), "2025-06-15T21:00:00");
    }

    #[test]
    fn resolve_arithmetic_add() {
        let now = make_now(); // 2025-06-15T12:00:00 UTC
        let result = resolve(
            &DateExpr::Arithmetic(
                Box::new(DateExpr::Relative(RelativeDate::Tomorrow, None)),
                ArithOp::Add,
                vec![DurationComponent {
                    count: 3,
                    unit: TemporalUnit::Hour,
                }],
            ),
            &now,
        )
        .unwrap();
        // tomorrow = June 16 at 00:00, + 3 hours = June 16 at 03:00
        assert_eq!(format_zoned(&result), "2025-06-16T03:00:00");
    }

    #[test]
    fn resolve_arithmetic_chained() {
        let now = make_now(); // 2025-06-15T12:00:00 UTC
        let result = resolve(
            &DateExpr::Arithmetic(
                Box::new(DateExpr::Arithmetic(
                    Box::new(DateExpr::Arithmetic(
                        Box::new(DateExpr::Now),
                        ArithOp::Add,
                        vec![DurationComponent {
                            count: 1,
                            unit: TemporalUnit::Day,
                        }],
                    )),
                    ArithOp::Add,
                    vec![DurationComponent {
                        count: 3,
                        unit: TemporalUnit::Hour,
                    }],
                )),
                ArithOp::Sub,
                vec![DurationComponent {
                    count: 30,
                    unit: TemporalUnit::Minute,
                }],
            ),
            &now,
        )
        .unwrap();
        // now (12:00) + 1 day (13 June 16 12:00) + 3 hours (15:00) - 30 min (14:30)
        assert_eq!(format_zoned(&result), "2025-06-16T14:30:00");
    }

    #[test]
    fn resolve_arithmetic_month_clamping() {
        // now + 1 month when now is Jan 31 -> Feb 28 (clamping per PARS-07)
        let jan31 = {
            let dt = civil::date(2025, 1, 31).at(12, 0, 0, 0);
            utc().to_ambiguous_zoned(dt).compatible().unwrap()
        };
        let result = resolve(
            &DateExpr::Arithmetic(
                Box::new(DateExpr::Now),
                ArithOp::Add,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Month,
                }],
            ),
            &jan31,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-02-28T12:00:00");
    }

    #[test]
    fn resolve_arithmetic_next_friday_minus_1_week() {
        let now = make_now(); // Sunday 2025-06-15
        let result = resolve(
            &DateExpr::Arithmetic(
                Box::new(DateExpr::DayRef(Direction::Next, Weekday::Friday, None)),
                ArithOp::Sub,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Week,
                }],
            ),
            &now,
        )
        .unwrap();
        // Next friday = June 20, - 1 week = June 13
        assert_eq!(format_zoned(&result), "2025-06-13T00:00:00");
    }

    #[test]
    fn resolve_range_returns_error_from_resolve() {
        // Calling resolve() on a Range expression should error
        let now = make_now();
        let result = resolve(&DateExpr::Range(RangeExpr::LastWeek), &now);
        assert!(result.is_err());
        assert!(result.unwrap_err().format_message().contains("parse_range"));
    }

    // --- Day reference golden test verification ---

    #[test]
    fn resolve_this_monday_on_sunday() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::This, Weekday::Monday, None),
            &now,
        )
        .unwrap();
        // "this monday" on Sunday = Monday June 16 (golden test expectation)
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn resolve_this_friday_on_sunday() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::This, Weekday::Friday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-20T00:00:00");
    }

    #[test]
    fn resolve_this_saturday_on_sunday() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::This, Weekday::Saturday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-21T00:00:00");
    }

    #[test]
    fn resolve_last_saturday_on_sunday() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::Last, Weekday::Saturday, None),
            &now,
        )
        .unwrap();
        // Last Saturday from Sunday = June 14
        assert_eq!(format_zoned(&result), "2025-06-14T00:00:00");
    }

    #[test]
    fn resolve_last_monday_on_sunday() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::Last, Weekday::Monday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-09T00:00:00");
    }

    #[test]
    fn resolve_next_monday_on_sunday() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Monday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn resolve_next_saturday_on_sunday() {
        let now = make_now(); // Sunday June 15
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Saturday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-21T00:00:00");
    }

    // --- Integration: parse() end-to-end ---

    #[test]
    fn parse_now_e2e() {
        let now = make_now();
        let result = crate::parser::parse("now", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T12:00:00");
    }

    #[test]
    fn parse_tomorrow_e2e() {
        let now = make_now();
        let result = crate::parser::parse("tomorrow", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn parse_in_3_days_e2e() {
        let now = make_now();
        let result = crate::parser::parse("in 3 days", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T12:00:00");
    }

    #[test]
    fn parse_epoch_e2e() {
        let now = make_now();
        let result = crate::parser::parse("@1735689600", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-01-01T00:00:00");
    }

    #[test]
    fn parse_empty_e2e() {
        let now = make_now();
        let result = crate::parser::parse("", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T12:00:00");
    }

    #[test]
    fn parse_error_e2e() {
        let now = make_now();
        let result = crate::parser::parse("???", &now, &en_kw());
        assert!(result.is_err());
    }

    #[test]
    fn parse_whitespace_e2e() {
        let now = make_now();
        let result = crate::parser::parse("   ", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T12:00:00");
    }

    #[test]
    fn parse_next_friday_17_00_e2e() {
        let now = make_now();
        let result = crate::parser::parse("next friday 17:00", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-20T17:00:00");
    }

    #[test]
    fn parse_a_week_ago_e2e() {
        let now = make_now();
        let result = crate::parser::parse("a week ago", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-08T12:00:00");
    }

    #[test]
    fn parse_an_hour_ago_e2e() {
        let now = make_now();
        let result = crate::parser::parse("an hour ago", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T11:00:00");
    }

    #[test]
    fn parse_iso_date_with_time_e2e() {
        let now = make_now();
        let result = crate::parser::parse("2022-11-07 13:25:30", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2022-11-07T13:25:30");
    }

    #[test]
    fn parse_today_18_30_e2e() {
        let now = make_now();
        let result = crate::parser::parse("today 18:30", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T18:30:00");
    }

    #[test]
    fn parse_this_sunday_e2e() {
        let now = make_now();
        let result = crate::parser::parse("this sunday", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T00:00:00");
    }

    #[test]
    fn parse_next_sunday_e2e() {
        let now = make_now();
        let result = crate::parser::parse("next sunday", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-22T00:00:00");
    }

    #[test]
    fn parse_last_sunday_e2e() {
        let now = make_now();
        let result = crate::parser::parse("last sunday", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-08T00:00:00");
    }

    // ── Phase 3: Range resolution tests ──────────────────────

    fn format_range(start: &Zoned, end: &Zoned) -> (String, String) {
        (format_zoned(start), format_zoned(end))
    }

    fn make_wednesday() -> Zoned {
        // Wednesday 2025-06-18 at 12:00 UTC
        let dt = civil::date(2025, 6, 18).at(12, 0, 0, 0);
        utc().to_ambiguous_zoned(dt).compatible().unwrap()
    }

    #[test]
    fn resolve_range_last_week() {
        let now = make_wednesday(); // Wed 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::LastWeek, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-09T00:00:00"); // Monday
        assert_eq!(e, "2025-06-15T23:59:59"); // Sunday
    }

    #[test]
    fn resolve_range_this_week() {
        let now = make_wednesday(); // Wed 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::ThisWeek, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-16T00:00:00"); // Monday
        assert_eq!(e, "2025-06-22T23:59:59"); // Sunday
    }

    #[test]
    fn resolve_range_next_week() {
        let now = make_wednesday(); // Wed 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::NextWeek, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-23T00:00:00"); // Monday
        assert_eq!(e, "2025-06-29T23:59:59"); // Sunday
    }

    #[test]
    fn resolve_range_this_month() {
        let now = make_wednesday(); // 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::ThisMonth, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-01T00:00:00");
        assert_eq!(e, "2025-06-30T23:59:59");
    }

    #[test]
    fn resolve_range_last_month() {
        let now = make_wednesday(); // 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::LastMonth, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-05-01T00:00:00");
        assert_eq!(e, "2025-05-31T23:59:59");
    }

    #[test]
    fn resolve_range_next_month() {
        let now = make_wednesday(); // 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::NextMonth, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-07-01T00:00:00");
        assert_eq!(e, "2025-07-31T23:59:59");
    }

    #[test]
    fn resolve_range_next_year() {
        let now = make_wednesday(); // 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::NextYear, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2026-01-01T00:00:00");
        assert_eq!(e, "2026-12-31T23:59:59");
    }

    #[test]
    fn resolve_range_this_year() {
        let now = make_wednesday(); // 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::ThisYear, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-01-01T00:00:00");
        assert_eq!(e, "2025-12-31T23:59:59");
    }

    #[test]
    fn resolve_range_last_year() {
        let now = make_wednesday(); // 2025-06-18
        let (start, end) = resolve_range(&RangeExpr::LastYear, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2024-01-01T00:00:00");
        assert_eq!(e, "2024-12-31T23:59:59");
    }

    #[test]
    fn resolve_range_q3_2025() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::Quarter(2025, 3), &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-07-01T00:00:00");
        assert_eq!(e, "2025-09-30T23:59:59");
    }

    #[test]
    fn resolve_range_q1_no_year() {
        let now = make_wednesday(); // 2025
        let (start, end) = resolve_range(&RangeExpr::Quarter(0, 1), &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-01-01T00:00:00");
        assert_eq!(e, "2025-03-31T23:59:59");
    }

    // ── Phase 3: End-to-end arithmetic tests ──────────────────────

    #[test]
    fn parse_tomorrow_plus_3_hours_e2e() {
        let now = make_now(); // 2025-06-15T12:00:00 UTC
        let result = crate::parser::parse("tomorrow + 3 hours", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T03:00:00");
    }

    #[test]
    fn parse_now_plus_1_day_plus_3_hours_minus_30_minutes_e2e() {
        let now = make_now();
        let result =
            crate::parser::parse("now + 1 day + 3 hours - 30 minutes", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T14:30:00");
    }

    #[test]
    fn parse_3_hours_after_tomorrow_e2e() {
        let now = make_now();
        let result = crate::parser::parse("3 hours after tomorrow", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T03:00:00");
    }

    #[test]
    fn parse_2_days_before_next_friday_e2e() {
        let now = make_now(); // Sunday 2025-06-15
        let result = crate::parser::parse("2 days before next friday", &now, &en_kw()).unwrap();
        // next friday = June 20, - 2 days = June 18
        assert_eq!(format_zoned(&result), "2025-06-18T00:00:00");
    }

    #[test]
    fn parse_next_friday_minus_1_week_e2e() {
        let now = make_now();
        let result = crate::parser::parse("next friday - 1 week", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-13T00:00:00");
    }

    #[test]
    fn parse_verbal_and_infix_same_result() {
        let now = make_now();
        let verbal = crate::parser::parse("3 hours after tomorrow", &now, &en_kw()).unwrap();
        let infix = crate::parser::parse("tomorrow + 3 hours", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&verbal), format_zoned(&infix));
    }

    // ── Phase 3: End-to-end range tests ──────────────────────

    #[test]
    fn parse_last_week_returns_single_date_e2e() {
        // "last week" now resolves as Offset(Past, 1 week) = single date, not range
        let now = make_wednesday(); // Wed 2025-06-18 12:00:00
        let result = crate::parser::parse("last week", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-11T12:00:00"); // 18 - 7 = 11, time preserved
    }

    #[test]
    fn parse_range_last_week_not_a_range_e2e() {
        // "last week" is no longer a range expression
        let now = make_wednesday();
        let result = crate::parser::parse_range("last week", &now, &en_kw());
        assert!(result.is_err());
    }

    #[test]
    fn parse_range_this_month_e2e() {
        let now = make_wednesday();
        let (start, end) = crate::parser::parse_range("this month", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&start), "2025-06-01T00:00:00");
        assert_eq!(format_zoned(&end), "2025-06-30T23:59:59");
    }

    #[test]
    fn parse_range_q3_2025_e2e() {
        let now = make_wednesday();
        let (start, end) = crate::parser::parse_range("Q3 2025", &now, &en_kw()).unwrap();
        assert_eq!(format_zoned(&start), "2025-07-01T00:00:00");
        assert_eq!(format_zoned(&end), "2025-09-30T23:59:59");
    }

    #[test]
    fn parse_range_not_a_range_errors() {
        let now = make_now();
        let result = crate::parser::parse_range("tomorrow", &now, &en_kw());
        assert!(result.is_err());
        assert!(result.unwrap_err().format_message().contains("not a range"));
    }
}
