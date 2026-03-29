//! AST resolver: maps DateExpr nodes to concrete jiff::Zoned datetimes.
//!
//! This module is a pure function of AST + reference time. No parsing logic.
//! All datetime arithmetic uses jiff's native calendar-aware operations.
//!
//! ## Clamping policy
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
    token::{BoundaryKind, EpochPrecision, TemporalUnit},
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
        DateExpr::Range(range) => resolve_range_start(range, now),
        DateExpr::Boundary(kind) => resolve_boundary(kind, now),
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

    let civil_dt = apply_time_or_midnight(target_date, time, now);
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

    let civil_dt = apply_time_or_midnight(target_date, time, now);
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
    let civil_dt = apply_time_or_midnight(date, time, now);
    let tz = now.time_zone().clone();
    tz.to_ambiguous_zoned(civil_dt)
        .compatible()
        .map_err(|e| ParseError::resolution(format!("ambiguous datetime: {e}")))
}

/// Resolve time-only expressions against today's date from `now`.
fn resolve_time_only(time: &TimeExpr, now: &Zoned) -> Result<Zoned, ParseError> {
    let today = now.date();
    let civil_dt = apply_time(today, time, Some(now));
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
/// - Monday as week start (ISO 8601)
/// - End boundaries are inclusive: 23:59:59.999999999
/// - All boundaries use `compatible()` for DST safety
pub(crate) fn resolve_range(range: &RangeExpr, now: &Zoned) -> Result<(Zoned, Zoned), ParseError> {
    let tz = now.time_zone().clone();
    let today = now.date();

    match range {
        RangeExpr::LastWeek => {
            let current_wd = today.weekday().to_monday_zero_offset() as i32;
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

/// Resolve a range expression to its start-of-period instant.
///
/// When the default `td` command encounters "this week", "next month", etc.,
/// it returns the start of that period as a single instant:
/// - "this week" -> Monday 00:00:00
/// - "next month" -> 1st day 00:00:00
/// - "this year" -> Jan 1 00:00:00
/// - "Q3 2025" -> Jul 1 00:00:00
fn resolve_range_start(range: &RangeExpr, now: &Zoned) -> Result<Zoned, ParseError> {
    let (start, _end) = resolve_range(range, now)?;
    Ok(start)
}

/// Resolve any expression as a range pair with implicit granularity.
///
/// Granularity is determined by the smallest unspecified time unit:
/// - No time specified -> day granularity (00:00:00..23:59:59)
/// - Hour only (via Nh notation) -> hour granularity (HH:00:00..HH:59:59)
/// - Hour:Minute specified -> minute granularity (HH:MM:00..HH:MM:59)
/// - Full time specified -> instant (duplicated)
/// - "now" -> instant (duplicated)
/// - Range expressions -> use existing resolve_range (week/month/year/quarter)
/// - Boundary keywords -> instant (duplicated)
pub(crate) fn resolve_range_with_granularity(
    expr: &DateExpr,
    now: &Zoned,
) -> Result<(Zoned, Zoned), ParseError> {
    match expr {
        DateExpr::Now => {
            let z = now.clone();
            Ok((z.clone(), z))
        }
        DateExpr::Range(range) => resolve_range(range, now),
        DateExpr::Relative(_, time) | DateExpr::DayRef(_, _, time) => {
            let z = resolve(expr, now)?;
            expand_by_time_granularity(z, time)
        }
        DateExpr::Absolute(_, time) => {
            let z = resolve(expr, now)?;
            expand_by_time_granularity(z, time)
        }
        DateExpr::Boundary(_) => {
            let z = resolve(expr, now)?;
            Ok((z.clone(), z))
        }
        _ => {
            let z = resolve(expr, now)?;
            Ok((z.clone(), z))
        }
    }
}

/// Expand a resolved datetime by the time specification's granularity.
///
/// - None -> day (00:00:00..23:59:59)
/// - HourOnly(h) -> hour (h:00:00..h:59:59)
/// - HourMinute(h,m) -> minute (h:m:00..h:m:59)
/// - HourMinuteSecond -> instant (same..same)
fn expand_by_time_granularity(
    base: Zoned,
    time: &Option<TimeExpr>,
) -> Result<(Zoned, Zoned), ParseError> {
    let tz = base.time_zone().clone();
    let date = base.date();

    match time {
        None => Ok((zoned_midnight(date, &tz)?, zoned_end_of_day(date, &tz)?)),
        Some(TimeExpr::HourOnly(h)) => {
            let dt_start = date.at(*h, 0, 0, 0);
            let dt_end = date.at(*h, 59, 59, 999_999_999);
            let start = tz
                .to_ambiguous_zoned(dt_start)
                .compatible()
                .map_err(|e| ParseError::resolution(format!("ambiguous: {e}")))?;
            let end = tz
                .to_ambiguous_zoned(dt_end)
                .compatible()
                .map_err(|e| ParseError::resolution(format!("ambiguous: {e}")))?;
            Ok((start, end))
        }
        Some(TimeExpr::HourMinute(h, m)) => {
            let dt_start = date.at(*h, *m, 0, 0);
            let dt_end = date.at(*h, *m, 59, 999_999_999);
            let start = tz
                .to_ambiguous_zoned(dt_start)
                .compatible()
                .map_err(|e| ParseError::resolution(format!("ambiguous: {e}")))?;
            let end = tz
                .to_ambiguous_zoned(dt_end)
                .compatible()
                .map_err(|e| ParseError::resolution(format!("ambiguous: {e}")))?;
            Ok((start, end))
        }
        Some(TimeExpr::HourMinuteSecond(..)) => Ok((base.clone(), base)),
        Some(TimeExpr::SameTime) => Ok((base.clone(), base)),
    }
}

/// Resolve a boundary keyword to a concrete datetime.
///
/// Boundaries are instants (specific moments in time):
/// - Start-of-period: 00:00:00.000000000
/// - End-of-period: 23:59:59.999999999
///
/// Week starts on Monday (ISO 8601).
/// Work week is Monday-Friday (soww/eoww).
/// Quarter boundaries: Q1=Jan-Mar, Q2=Apr-Jun, Q3=Jul-Sep, Q4=Oct-Dec.
fn resolve_boundary(kind: &BoundaryKind, now: &Zoned) -> Result<Zoned, ParseError> {
    let tz = now.time_zone().clone();
    let today = now.date();
    let current_wd = today.weekday().to_monday_zero_offset() as i64;

    match kind {
        BoundaryKind::Sod => zoned_midnight(today, &tz),
        BoundaryKind::Eod => zoned_end_of_day(today, &tz),
        BoundaryKind::Sow => {
            let monday = today
                .checked_sub(Span::new().days(current_wd))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_midnight(monday, &tz)
        }
        BoundaryKind::Eow => {
            let monday = today
                .checked_sub(Span::new().days(current_wd))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let sunday = monday
                .checked_add(Span::new().days(6))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_end_of_day(sunday, &tz)
        }
        BoundaryKind::Soww => {
            let monday = today
                .checked_sub(Span::new().days(current_wd))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_midnight(monday, &tz)
        }
        BoundaryKind::Eoww => {
            let monday = today
                .checked_sub(Span::new().days(current_wd))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let friday = monday
                .checked_add(Span::new().days(4))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_end_of_day(friday, &tz)
        }
        BoundaryKind::Som => {
            let first = civil::date(today.year(), today.month(), 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eom => {
            let first = civil::date(today.year(), today.month(), 1);
            let last_day = first.days_in_month();
            let last = civil::date(today.year(), today.month(), last_day);
            zoned_end_of_day(last, &tz)
        }
        BoundaryKind::Soq => {
            let q = (today.month() - 1) / 3 + 1;
            let start_month = (q - 1) * 3 + 1;
            let first = civil::date(today.year(), start_month, 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eoq => {
            let q = (today.month() - 1) / 3 + 1;
            let end_month = q * 3;
            let end_date = civil::date(today.year(), end_month, 1);
            let last_day = end_date.days_in_month();
            let last = civil::date(today.year(), end_month, last_day);
            zoned_end_of_day(last, &tz)
        }
        BoundaryKind::Soy => {
            let first = civil::date(today.year(), 1, 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eoy => {
            let last = civil::date(today.year(), 12, 31);
            zoned_end_of_day(last, &tz)
        }

        BoundaryKind::Sopd => {
            let yesterday = today
                .checked_sub(Span::new().days(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_midnight(yesterday, &tz)
        }
        BoundaryKind::Eopd => {
            let yesterday = today
                .checked_sub(Span::new().days(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_end_of_day(yesterday, &tz)
        }
        BoundaryKind::Sopw => {
            let this_monday = today
                .checked_sub(Span::new().days(current_wd))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let last_monday = this_monday
                .checked_sub(Span::new().weeks(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_midnight(last_monday, &tz)
        }
        BoundaryKind::Eopw => {
            let this_monday = today
                .checked_sub(Span::new().days(current_wd))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let last_sunday = this_monday
                .checked_sub(Span::new().days(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_end_of_day(last_sunday, &tz)
        }
        BoundaryKind::Sopm => {
            let (year, month) = prev_month(today.year(), today.month());
            let first = civil::date(year, month, 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eopm => {
            let (year, month) = prev_month(today.year(), today.month());
            let first = civil::date(year, month, 1);
            let last_day = first.days_in_month();
            let last = civil::date(year, month, last_day);
            zoned_end_of_day(last, &tz)
        }
        BoundaryKind::Sopq => {
            let q = (today.month() - 1) / 3 + 1;
            let (year, prev_q) = if q == 1 {
                (today.year() - 1, 4i8)
            } else {
                (today.year(), q - 1)
            };
            let start_month = (prev_q - 1) * 3 + 1;
            let first = civil::date(year, start_month, 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eopq => {
            let q = (today.month() - 1) / 3 + 1;
            let (year, prev_q) = if q == 1 {
                (today.year() - 1, 4i8)
            } else {
                (today.year(), q - 1)
            };
            let end_month = prev_q * 3;
            let end_date = civil::date(year, end_month, 1);
            let last_day = end_date.days_in_month();
            let last = civil::date(year, end_month, last_day);
            zoned_end_of_day(last, &tz)
        }
        BoundaryKind::Sopy => {
            let first = civil::date(today.year() - 1, 1, 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eopy => {
            let last = civil::date(today.year() - 1, 12, 31);
            zoned_end_of_day(last, &tz)
        }

        BoundaryKind::Sond => {
            let tomorrow = today
                .checked_add(Span::new().days(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_midnight(tomorrow, &tz)
        }
        BoundaryKind::Eond => {
            let tomorrow = today
                .checked_add(Span::new().days(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_end_of_day(tomorrow, &tz)
        }
        BoundaryKind::Sonw => {
            let this_monday = today
                .checked_sub(Span::new().days(current_wd))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let next_monday = this_monday
                .checked_add(Span::new().weeks(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_midnight(next_monday, &tz)
        }
        BoundaryKind::Eonw => {
            let this_monday = today
                .checked_sub(Span::new().days(current_wd))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            let next_sunday = this_monday
                .checked_add(Span::new().weeks(1))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?
                .checked_add(Span::new().days(6))
                .map_err(|e| ParseError::resolution(format!("overflow: {e}")))?;
            zoned_end_of_day(next_sunday, &tz)
        }
        BoundaryKind::Sonm => {
            let (year, month) = next_month(today.year(), today.month());
            let first = civil::date(year, month, 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eonm => {
            let (year, month) = next_month(today.year(), today.month());
            let first = civil::date(year, month, 1);
            let last_day = first.days_in_month();
            let last = civil::date(year, month, last_day);
            zoned_end_of_day(last, &tz)
        }
        BoundaryKind::Sonq => {
            let q = (today.month() - 1) / 3 + 1;
            let (year, next_q) = if q == 4 {
                (today.year() + 1, 1i8)
            } else {
                (today.year(), q + 1)
            };
            let start_month = (next_q - 1) * 3 + 1;
            let first = civil::date(year, start_month, 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eonq => {
            let q = (today.month() - 1) / 3 + 1;
            let (year, next_q) = if q == 4 {
                (today.year() + 1, 1i8)
            } else {
                (today.year(), q + 1)
            };
            let end_month = next_q * 3;
            let end_date = civil::date(year, end_month, 1);
            let last_day = end_date.days_in_month();
            let last = civil::date(year, end_month, last_day);
            zoned_end_of_day(last, &tz)
        }
        BoundaryKind::Sony => {
            let first = civil::date(today.year() + 1, 1, 1);
            zoned_midnight(first, &tz)
        }
        BoundaryKind::Eony => {
            let last = civil::date(today.year() + 1, 12, 31);
            zoned_end_of_day(last, &tz)
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
/// Accepts a `now` reference so that `SameTime` can extract the current time.
fn apply_time_or_midnight(
    date: civil::Date,
    time: &Option<TimeExpr>,
    now: &Zoned,
) -> civil::DateTime {
    match time {
        Some(t) => apply_time(date, t, Some(now)),
        None => date.at(0, 0, 0, 0),
    }
}

/// Apply a TimeExpr to a date.
/// When `now` is provided and the time is `SameTime`, uses the time from `now`.
fn apply_time(date: civil::Date, time: &TimeExpr, now: Option<&Zoned>) -> civil::DateTime {
    match time {
        TimeExpr::HourMinute(h, m) => date.at(*h, *m, 0, 0),
        TimeExpr::HourMinuteSecond(h, m, s) => date.at(*h, *m, *s, 0),
        TimeExpr::HourOnly(h) => date.at(*h, 0, 0, 0),
        TimeExpr::SameTime => {
            if let Some(now) = now {
                let t = now.datetime().time();
                date.at(t.hour(), t.minute(), t.second(), t.subsec_nanosecond())
            } else {
                date.at(0, 0, 0, 0)
            }
        }
    }
}

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
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Friday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-20T00:00:00");
    }

    #[test]
    fn resolve_next_sunday_on_sunday_advances_7() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Sunday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-22T00:00:00");
    }

    #[test]
    fn resolve_this_sunday_on_sunday_is_today() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::This, Weekday::Sunday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T00:00:00");
    }

    #[test]
    fn resolve_last_sunday_on_sunday_goes_back_7() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::Last, Weekday::Sunday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-08T00:00:00");
    }

    #[test]
    fn resolve_offset_future_3_days() {
        let now = make_now();
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
        let now = make_now();
        let result = resolve(
            &DateExpr::Absolute(
                AbsoluteDate {
                    year: 0,
                    month: 3,
                    day: 24,
                },
                None,
            ),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-03-24T00:00:00");
    }

    #[test]
    fn resolve_time_only() {
        let now = make_now();
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
        let now = make_now();
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
        assert_eq!(format_zoned(&result), "2025-06-15T21:00:00");
    }

    #[test]
    fn resolve_arithmetic_add() {
        let now = make_now();
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
        assert_eq!(format_zoned(&result), "2025-06-16T03:00:00");
    }

    #[test]
    fn resolve_arithmetic_chained() {
        let now = make_now();
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
        assert_eq!(format_zoned(&result), "2025-06-16T14:30:00");
    }

    #[test]
    fn resolve_arithmetic_month_clamping() {
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
        let now = make_now();
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
        assert_eq!(format_zoned(&result), "2025-06-13T00:00:00");
    }

    #[test]
    fn resolve_range_returns_start_of_period() {
        let now = make_wednesday();
        let result = resolve(&DateExpr::Range(RangeExpr::ThisWeek), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn resolve_range_start_next_month() {
        let now = make_wednesday();
        let result = resolve(&DateExpr::Range(RangeExpr::NextMonth), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-07-01T00:00:00");
    }

    #[test]
    fn resolve_this_monday_on_sunday() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::This, Weekday::Monday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn resolve_this_friday_on_sunday() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::This, Weekday::Friday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-20T00:00:00");
    }

    #[test]
    fn resolve_this_saturday_on_sunday() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::This, Weekday::Saturday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-21T00:00:00");
    }

    #[test]
    fn resolve_last_saturday_on_sunday() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::Last, Weekday::Saturday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-14T00:00:00");
    }

    #[test]
    fn resolve_last_monday_on_sunday() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::Last, Weekday::Monday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-09T00:00:00");
    }

    #[test]
    fn resolve_next_monday_on_sunday() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Monday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn resolve_next_saturday_on_sunday() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Saturday, None),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-21T00:00:00");
    }

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

    fn format_range(start: &Zoned, end: &Zoned) -> (String, String) {
        (format_zoned(start), format_zoned(end))
    }

    fn make_wednesday() -> Zoned {
        let dt = civil::date(2025, 6, 18).at(12, 0, 0, 0);
        utc().to_ambiguous_zoned(dt).compatible().unwrap()
    }

    #[test]
    fn resolve_range_last_week() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::LastWeek, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-09T00:00:00");
        assert_eq!(e, "2025-06-15T23:59:59");
    }

    #[test]
    fn resolve_range_this_week() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::ThisWeek, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-16T00:00:00");
        assert_eq!(e, "2025-06-22T23:59:59");
    }

    #[test]
    fn resolve_range_next_week() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::NextWeek, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-23T00:00:00");
        assert_eq!(e, "2025-06-29T23:59:59");
    }

    #[test]
    fn resolve_range_this_month() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::ThisMonth, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-01T00:00:00");
        assert_eq!(e, "2025-06-30T23:59:59");
    }

    #[test]
    fn resolve_range_last_month() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::LastMonth, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-05-01T00:00:00");
        assert_eq!(e, "2025-05-31T23:59:59");
    }

    #[test]
    fn resolve_range_next_month() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::NextMonth, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-07-01T00:00:00");
        assert_eq!(e, "2025-07-31T23:59:59");
    }

    #[test]
    fn resolve_range_next_year() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::NextYear, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2026-01-01T00:00:00");
        assert_eq!(e, "2026-12-31T23:59:59");
    }

    #[test]
    fn resolve_range_this_year() {
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::ThisYear, &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-01-01T00:00:00");
        assert_eq!(e, "2025-12-31T23:59:59");
    }

    #[test]
    fn resolve_range_last_year() {
        let now = make_wednesday();
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
        let now = make_wednesday();
        let (start, end) = resolve_range(&RangeExpr::Quarter(0, 1), &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-01-01T00:00:00");
        assert_eq!(e, "2025-03-31T23:59:59");
    }

    #[test]
    fn parse_tomorrow_plus_3_hours_e2e() {
        let now = make_now();
        let result = crate::parser::parse("tomorrow + 3 hours", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T03:00:00");
    }

    #[test]
    fn parse_now_plus_1_day_plus_3_hours_minus_30_minutes_e2e() {
        let now = make_now();
        let result = crate::parser::parse("now + 1 day + 3 hours - 30 minutes", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T14:30:00");
    }

    #[test]
    fn parse_3_hours_after_tomorrow_e2e() {
        let now = make_now();
        let result = crate::parser::parse("3 hours after tomorrow", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T03:00:00");
    }

    #[test]
    fn parse_2_days_before_next_friday_e2e() {
        let now = make_now();
        let result = crate::parser::parse("2 days before next friday", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T00:00:00");
    }

    #[test]
    fn parse_next_friday_minus_1_week_e2e() {
        let now = make_now();
        let result = crate::parser::parse("next friday - 1 week", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-13T00:00:00");
    }

    #[test]
    fn parse_verbal_and_infix_same_result() {
        let now = make_now();
        let verbal = crate::parser::parse("3 hours after tomorrow", &now).unwrap();
        let infix = crate::parser::parse("tomorrow + 3 hours", &now).unwrap();
        assert_eq!(format_zoned(&verbal), format_zoned(&infix));
    }

    #[test]
    fn parse_last_week_returns_period_start_e2e() {
        let now = make_wednesday();
        let result = crate::parser::parse("last week", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-09T00:00:00");
    }

    #[test]
    fn parse_range_last_week_produces_range_e2e() {
        let now = make_wednesday();
        let (start, end) = crate::parser::parse_range_with_granularity("last week", &now).unwrap();
        assert_eq!(format_zoned(&start), "2025-06-09T00:00:00");
        assert_eq!(format_zoned(&end), "2025-06-15T23:59:59");
    }

    #[test]
    fn parse_range_this_month_e2e() {
        let now = make_wednesday();
        let (start, end) = crate::parser::parse_range_with_granularity("this month", &now).unwrap();
        assert_eq!(format_zoned(&start), "2025-06-01T00:00:00");
        assert_eq!(format_zoned(&end), "2025-06-30T23:59:59");
    }

    #[test]
    fn parse_range_q3_2025_e2e() {
        let now = make_wednesday();
        let (start, end) = crate::parser::parse_range_with_granularity("Q3 2025", &now).unwrap();
        assert_eq!(format_zoned(&start), "2025-07-01T00:00:00");
        assert_eq!(format_zoned(&end), "2025-09-30T23:59:59");
    }

    #[test]
    fn parse_range_with_granularity_resolves_non_range_as_day() {
        let now = make_now();
        let (start, end) = crate::parser::parse_range_with_granularity("tomorrow", &now).unwrap();
        let start_str = format_zoned(&start);
        let end_str = format_zoned(&end);
        assert!(
            start_str.ends_with("T00:00:00"),
            "start should be midnight: {start_str}"
        );
        assert!(
            end_str.ends_with("T23:59:59"),
            "end should be end of day: {end_str}"
        );
    }

    fn boundary_now() -> Zoned {
        let dt = civil::date(2025, 6, 18).at(14, 30, 0, 0);
        utc().to_ambiguous_zoned(dt).compatible().unwrap()
    }

    #[test]
    fn resolve_boundary_sod() {
        let now = boundary_now();
        let result = resolve(&DateExpr::Boundary(BoundaryKind::Sod), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T00:00:00");
    }

    #[test]
    fn resolve_boundary_eod() {
        let now = boundary_now();
        let result = resolve(&DateExpr::Boundary(BoundaryKind::Eod), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T23:59:59");
    }

    #[test]
    fn resolve_boundary_sow() {
        let now = boundary_now();
        let result = resolve(&DateExpr::Boundary(BoundaryKind::Sow), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T00:00:00");
    }

    #[test]
    fn resolve_boundary_eow() {
        let now = boundary_now();
        let result = resolve(&DateExpr::Boundary(BoundaryKind::Eow), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-22T23:59:59");
    }

    #[test]
    fn resolve_boundary_soww_eoww() {
        let now = boundary_now();
        let soww = resolve(&DateExpr::Boundary(BoundaryKind::Soww), &now).unwrap();
        assert_eq!(format_zoned(&soww), "2025-06-16T00:00:00");
        let eoww = resolve(&DateExpr::Boundary(BoundaryKind::Eoww), &now).unwrap();
        assert_eq!(format_zoned(&eoww), "2025-06-20T23:59:59");
    }

    #[test]
    fn resolve_boundary_som_eom() {
        let now = boundary_now();
        let som = resolve(&DateExpr::Boundary(BoundaryKind::Som), &now).unwrap();
        assert_eq!(format_zoned(&som), "2025-06-01T00:00:00");
        let eom = resolve(&DateExpr::Boundary(BoundaryKind::Eom), &now).unwrap();
        assert_eq!(format_zoned(&eom), "2025-06-30T23:59:59");
    }

    #[test]
    fn resolve_boundary_soq_eoq() {
        let now = boundary_now();
        let soq = resolve(&DateExpr::Boundary(BoundaryKind::Soq), &now).unwrap();
        assert_eq!(format_zoned(&soq), "2025-04-01T00:00:00");
        let eoq = resolve(&DateExpr::Boundary(BoundaryKind::Eoq), &now).unwrap();
        assert_eq!(format_zoned(&eoq), "2025-06-30T23:59:59");
    }

    #[test]
    fn resolve_boundary_soy_eoy() {
        let now = boundary_now();
        let soy = resolve(&DateExpr::Boundary(BoundaryKind::Soy), &now).unwrap();
        assert_eq!(format_zoned(&soy), "2025-01-01T00:00:00");
        let eoy = resolve(&DateExpr::Boundary(BoundaryKind::Eoy), &now).unwrap();
        assert_eq!(format_zoned(&eoy), "2025-12-31T23:59:59");
    }

    #[test]
    fn resolve_boundary_sopd_eopd() {
        let now = boundary_now();
        let sopd = resolve(&DateExpr::Boundary(BoundaryKind::Sopd), &now).unwrap();
        assert_eq!(format_zoned(&sopd), "2025-06-17T00:00:00");
        let eopd = resolve(&DateExpr::Boundary(BoundaryKind::Eopd), &now).unwrap();
        assert_eq!(format_zoned(&eopd), "2025-06-17T23:59:59");
    }

    #[test]
    fn resolve_boundary_sonw_eonw() {
        let now = boundary_now();
        let sonw = resolve(&DateExpr::Boundary(BoundaryKind::Sonw), &now).unwrap();
        assert_eq!(format_zoned(&sonw), "2025-06-23T00:00:00");
        let eonw = resolve(&DateExpr::Boundary(BoundaryKind::Eonw), &now).unwrap();
        assert_eq!(format_zoned(&eonw), "2025-06-29T23:59:59");
    }

    #[test]
    fn resolve_boundary_sopw_eopw() {
        let now = boundary_now();
        let sopw = resolve(&DateExpr::Boundary(BoundaryKind::Sopw), &now).unwrap();
        assert_eq!(format_zoned(&sopw), "2025-06-09T00:00:00");
        let eopw = resolve(&DateExpr::Boundary(BoundaryKind::Eopw), &now).unwrap();
        assert_eq!(format_zoned(&eopw), "2025-06-15T23:59:59");
    }

    #[test]
    fn resolve_boundary_sopm_eopm() {
        let now = boundary_now();
        let sopm = resolve(&DateExpr::Boundary(BoundaryKind::Sopm), &now).unwrap();
        assert_eq!(format_zoned(&sopm), "2025-05-01T00:00:00");
        let eopm = resolve(&DateExpr::Boundary(BoundaryKind::Eopm), &now).unwrap();
        assert_eq!(format_zoned(&eopm), "2025-05-31T23:59:59");
    }

    #[test]
    fn resolve_boundary_sonm_eonm() {
        let now = boundary_now();
        let sonm = resolve(&DateExpr::Boundary(BoundaryKind::Sonm), &now).unwrap();
        assert_eq!(format_zoned(&sonm), "2025-07-01T00:00:00");
        let eonm = resolve(&DateExpr::Boundary(BoundaryKind::Eonm), &now).unwrap();
        assert_eq!(format_zoned(&eonm), "2025-07-31T23:59:59");
    }

    #[test]
    fn resolve_boundary_sopq_eopq() {
        let now = boundary_now();
        let sopq = resolve(&DateExpr::Boundary(BoundaryKind::Sopq), &now).unwrap();
        assert_eq!(format_zoned(&sopq), "2025-01-01T00:00:00");
        let eopq = resolve(&DateExpr::Boundary(BoundaryKind::Eopq), &now).unwrap();
        assert_eq!(format_zoned(&eopq), "2025-03-31T23:59:59");
    }

    #[test]
    fn resolve_boundary_sonq_eonq() {
        let now = boundary_now();
        let sonq = resolve(&DateExpr::Boundary(BoundaryKind::Sonq), &now).unwrap();
        assert_eq!(format_zoned(&sonq), "2025-07-01T00:00:00");
        let eonq = resolve(&DateExpr::Boundary(BoundaryKind::Eonq), &now).unwrap();
        assert_eq!(format_zoned(&eonq), "2025-09-30T23:59:59");
    }

    #[test]
    fn resolve_boundary_sopy_eopy() {
        let now = boundary_now();
        let sopy = resolve(&DateExpr::Boundary(BoundaryKind::Sopy), &now).unwrap();
        assert_eq!(format_zoned(&sopy), "2024-01-01T00:00:00");
        let eopy = resolve(&DateExpr::Boundary(BoundaryKind::Eopy), &now).unwrap();
        assert_eq!(format_zoned(&eopy), "2024-12-31T23:59:59");
    }

    #[test]
    fn resolve_boundary_sony_eony() {
        let now = boundary_now();
        let sony = resolve(&DateExpr::Boundary(BoundaryKind::Sony), &now).unwrap();
        assert_eq!(format_zoned(&sony), "2026-01-01T00:00:00");
        let eony = resolve(&DateExpr::Boundary(BoundaryKind::Eony), &now).unwrap();
        assert_eq!(format_zoned(&eony), "2026-12-31T23:59:59");
    }

    #[test]
    fn resolve_boundary_sond_eond() {
        let now = boundary_now();
        let sond = resolve(&DateExpr::Boundary(BoundaryKind::Sond), &now).unwrap();
        assert_eq!(format_zoned(&sond), "2025-06-19T00:00:00");
        let eond = resolve(&DateExpr::Boundary(BoundaryKind::Eond), &now).unwrap();
        assert_eq!(format_zoned(&eond), "2025-06-19T23:59:59");
    }

    #[test]
    fn resolve_hour_only_time() {
        let now = boundary_now();
        let result = resolve(
            &DateExpr::Relative(RelativeDate::Today, Some(TimeExpr::HourOnly(18))),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T18:00:00");
    }

    #[test]
    fn parse_eod_e2e() {
        let now = boundary_now();
        let result = crate::parser::parse("eod", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T23:59:59");
    }

    #[test]
    fn parse_sod_e2e() {
        let now = boundary_now();
        let result = crate::parser::parse("sod", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T00:00:00");
    }

    #[test]
    fn parse_eod_plus_1h_e2e() {
        let now = boundary_now();
        let result = crate::parser::parse("eod + 1h", &now).unwrap();
        let formatted = result.strftime("%Y-%m-%dT%H:%M").to_string();
        assert_eq!(formatted, "2025-06-19T00:59");
    }

    #[test]
    fn parse_plus_3h_e2e() {
        let now = boundary_now();
        let result = crate::parser::parse("+3h", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T17:30:00");
    }

    #[test]
    fn parse_minus_1d_e2e() {
        let now = boundary_now();
        let result = crate::parser::parse("-1d", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-17T14:30:00");
    }

    #[test]
    fn parse_today_18h_e2e() {
        let now = boundary_now();
        let result = crate::parser::parse("today 18h", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-18T18:00:00");
    }

    #[test]
    fn parse_now_plus_13h30_e2e() {
        let now = boundary_now();
        let result = crate::parser::parse("now+13h30", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-19T04:00:00");
    }

    #[test]
    fn parse_now_plus_colon_duration_e2e() {
        let now = boundary_now();
        let result = crate::parser::parse("now+13:30", &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-19T04:00:00");
    }

    #[test]
    fn range_granularity_day() {
        let now = make_now();
        let (start, end) =
            resolve_range_with_granularity(&DateExpr::Relative(RelativeDate::Tomorrow, None), &now)
                .unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-16T00:00:00");
        assert_eq!(e, "2025-06-16T23:59:59");
    }

    #[test]
    fn range_granularity_hour() {
        let now = make_now();
        let (start, end) = resolve_range_with_granularity(
            &DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourOnly(18))),
            &now,
        )
        .unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-16T18:00:00");
        assert_eq!(e, "2025-06-16T18:59:59");
    }

    #[test]
    fn range_granularity_minute() {
        let now = make_now();
        let (start, end) = resolve_range_with_granularity(
            &DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourMinute(18, 30))),
            &now,
        )
        .unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-16T18:30:00");
        assert_eq!(e, "2025-06-16T18:30:59");
    }

    #[test]
    fn range_granularity_second_is_instant() {
        let now = make_now();
        let (start, end) = resolve_range_with_granularity(
            &DateExpr::Relative(
                RelativeDate::Tomorrow,
                Some(TimeExpr::HourMinuteSecond(18, 30, 45)),
            ),
            &now,
        )
        .unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-16T18:30:45");
        assert_eq!(e, "2025-06-16T18:30:45");
    }

    #[test]
    fn range_granularity_now_is_instant() {
        let now = make_now();
        let (start, end) = resolve_range_with_granularity(&DateExpr::Now, &now).unwrap();
        assert_eq!(start, end);
        assert_eq!(format_zoned(&start), "2025-06-15T12:00:00");
    }

    #[test]
    fn range_granularity_this_week_uses_resolve_range() {
        let now = make_wednesday();
        let (start, end) =
            resolve_range_with_granularity(&DateExpr::Range(RangeExpr::ThisWeek), &now).unwrap();
        let (s, e) = format_range(&start, &end);
        assert_eq!(s, "2025-06-16T00:00:00");
        assert_eq!(e, "2025-06-22T23:59:59");
    }

    #[test]
    fn range_granularity_boundary_is_instant() {
        let now = make_wednesday();
        let (start, end) =
            resolve_range_with_granularity(&DateExpr::Boundary(BoundaryKind::Eod), &now).unwrap();
        assert_eq!(start, end);
        assert_eq!(format_zoned(&start), "2025-06-18T23:59:59");
    }

    #[test]
    fn resolve_tomorrow_at_same_time() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::SameTime)),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T12:00:00");
    }

    #[test]
    fn resolve_yesterday_at_same_time() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Relative(RelativeDate::Yesterday, Some(TimeExpr::SameTime)),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-14T12:00:00");
    }

    #[test]
    fn resolve_next_friday_at_same_time() {
        let now = make_now();
        let result = resolve(
            &DateExpr::DayRef(Direction::Next, Weekday::Friday, Some(TimeExpr::SameTime)),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-20T12:00:00");
    }

    #[test]
    fn resolve_same_time_preserves_seconds() {
        let dt = civil::date(2025, 6, 15).at(12, 34, 56, 0);
        let now = utc().to_ambiguous_zoned(dt).compatible().unwrap();
        let result = resolve(
            &DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::SameTime)),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T12:34:56");
    }

    #[test]
    fn resolve_3pm_time_only() {
        let now = make_now();
        let result = resolve(&DateExpr::TimeOnly(TimeExpr::HourMinute(15, 0)), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T15:00:00");
    }

    #[test]
    fn resolve_12am_time_only() {
        let now = make_now();
        let result = resolve(&DateExpr::TimeOnly(TimeExpr::HourMinute(0, 0)), &now).unwrap();
        assert_eq!(format_zoned(&result), "2025-06-15T00:00:00");
    }

    #[test]
    fn resolve_tomorrow_at_3pm() {
        let now = make_now();
        let result = resolve(
            &DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::HourMinute(15, 0))),
            &now,
        )
        .unwrap();
        assert_eq!(format_zoned(&result), "2025-06-16T15:00:00");
    }

    #[test]
    fn range_granularity_same_time_is_instant() {
        let now = make_now();
        let (start, end) = resolve_range_with_granularity(
            &DateExpr::Relative(RelativeDate::Tomorrow, Some(TimeExpr::SameTime)),
            &now,
        )
        .unwrap();
        assert_eq!(start, end);
        assert_eq!(format_zoned(&start), "2025-06-16T12:00:00");
    }
}
