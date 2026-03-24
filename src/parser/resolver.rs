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

use jiff::{civil, Span, Zoned};

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
        DateExpr::Arithmetic(..) | DateExpr::Range(..) => Err(ParseError::unsupported(
            "arithmetic and range expressions are not yet supported (Phase 3)",
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
    };

    let civil_dt = apply_time_or_midnight(target_date, time);
    let tz = now.time_zone().clone();
    tz.to_ambiguous_zoned(civil_dt).compatible().map_err(|e| {
        ParseError::resolution(format!("ambiguous datetime: {e}"))
    })
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
    tz.to_ambiguous_zoned(civil_dt).compatible().map_err(|e| {
        ParseError::resolution(format!("ambiguous datetime: {e}"))
    })
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
    tz.to_ambiguous_zoned(civil_dt).compatible().map_err(|e| {
        ParseError::resolution(format!("ambiguous datetime: {e}"))
    })
}

/// Resolve time-only expressions against today's date from `now`.
fn resolve_time_only(time: &TimeExpr, now: &Zoned) -> Result<Zoned, ParseError> {
    let today = now.date();
    let civil_dt = apply_time(today, time);
    let tz = now.time_zone().clone();
    tz.to_ambiguous_zoned(civil_dt).compatible().map_err(|e| {
        ParseError::resolution(format!("ambiguous datetime: {e}"))
    })
}

/// Resolve epoch timestamps, dispatching on precision.
fn resolve_epoch(
    epoch: &EpochValue,
    tz: &jiff::tz::TimeZone,
) -> Result<Zoned, ParseError> {
    let timestamp = match epoch.precision {
        EpochPrecision::Seconds => jiff::Timestamp::from_second(epoch.raw),
        EpochPrecision::Milliseconds => jiff::Timestamp::from_millisecond(epoch.raw),
        EpochPrecision::Microseconds => jiff::Timestamp::from_microsecond(epoch.raw),
        EpochPrecision::Nanoseconds => jiff::Timestamp::from_nanosecond(epoch.raw as i128),
    };
    let ts = timestamp.map_err(|e| {
        ParseError::resolution(format!("epoch timestamp out of range: {e}"))
    })?;
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
        let result =
            resolve(&DateExpr::Relative(RelativeDate::Overmorrow, None), &now).unwrap();
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
        let result = resolve(
            &DateExpr::TimeOnly(TimeExpr::HourMinute(15, 30)),
            &now,
        )
        .unwrap();
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
    fn resolve_arithmetic_unsupported() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Arithmetic(
                Box::new(DateExpr::Now),
                ArithOp::Add,
                vec![DurationComponent {
                    count: 1,
                    unit: TemporalUnit::Day,
                }],
            ),
            &now,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().format_message().contains("Phase 3"));
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
        let result = crate::parser::parse("now", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T12:00:00");
    }

    #[test]
    fn parse_tomorrow_e2e() {
        let now = make_now();
        let result = crate::parser::parse("tomorrow", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn parse_in_3_days_e2e() {
        let now = make_now();
        let result = crate::parser::parse("in 3 days", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T12:00:00");
    }

    #[test]
    fn parse_epoch_e2e() {
        let now = make_now();
        let result = crate::parser::parse("@1735689600", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-01-01T00:00:00");
    }

    #[test]
    fn parse_empty_e2e() {
        let now = make_now();
        let result = crate::parser::parse("", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T12:00:00");
    }

    #[test]
    fn parse_error_e2e() {
        let now = make_now();
        let result = crate::parser::parse("???", &now);
        assert!(result.is_err());
    }

    #[test]
    fn parse_whitespace_e2e() {
        let now = make_now();
        let result = crate::parser::parse("   ", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T12:00:00");
    }

    #[test]
    fn parse_next_friday_17_00_e2e() {
        let now = make_now();
        let result = crate::parser::parse("next friday 17:00", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-20T17:00:00");
    }

    #[test]
    fn parse_a_week_ago_e2e() {
        let now = make_now();
        let result = crate::parser::parse("a week ago", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-08T12:00:00");
    }

    #[test]
    fn parse_an_hour_ago_e2e() {
        let now = make_now();
        let result = crate::parser::parse("an hour ago", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T11:00:00");
    }

    #[test]
    fn parse_iso_date_with_time_e2e() {
        let now = make_now();
        let result = crate::parser::parse("2022-11-07 13:25:30", &now).unwrap();
        assert_eq!(format_zoned(&result), "2022-11-07T13:25:30");
    }

    #[test]
    fn parse_today_18_30_e2e() {
        let now = make_now();
        let result = crate::parser::parse("today 18:30", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T18:30:00");
    }

    #[test]
    fn parse_this_sunday_e2e() {
        let now = make_now();
        let result = crate::parser::parse("this sunday", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T00:00:00");
    }

    #[test]
    fn parse_next_sunday_e2e() {
        let now = make_now();
        let result = crate::parser::parse("next sunday", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-22T00:00:00");
    }

    #[test]
    fn parse_last_sunday_e2e() {
        let now = make_now();
        let result = crate::parser::parse("last sunday", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-08T00:00:00");
    }
}
