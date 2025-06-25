//! Core transformation logic for **TARDIS**.
//!
//! Converts a natural-language date expression into a formatted string,
//! applying optional presets and an explicit time-zone/context “now”.

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use human_date_parser::{ParseResult, from_human_time};

use crate::{Error, Result, errors::UserInputError, user_input_error};

/// Immutable application context passed to [`process`].
#[derive(Debug)]
pub struct App {
    /// Raw human-readable expression (e.g. `"next Friday 10 am"`).
    pub date: String,
    /// Either a chrono-style format string *or* the name of a preset.
    pub format: String,
    /// Target time-zone for output.
    pub timezone: Tz,
    /// Optional “now” (useful for deterministic tests).
    pub now: Option<DateTime<Tz>>,
}

/// Pairing of a **named** preset with a chrono format string.
#[derive(Debug, Clone)]
pub struct Preset {
    pub name: String,
    pub format: String,
}

/// Parse `app.date`, resolve the effective format, and render a string.
///
/// * `presets` is passed as a slice to avoid unnecessary allocation.
/// * All error paths bubble up via [`Result`], ready for unit testing.
pub fn process(app: &App, presets: &[Preset]) -> Result<String> {
    let now = app
        .now
        .unwrap_or_else(|| app.timezone.from_utc_datetime(&Utc::now().naive_utc()));

    let parsed = from_human_time(&app.date, now.naive_local()).map_err(|e| {
        user_input_error!(
            InvalidDateFormat,
            "failed to parse human date '{}': {}",
            app.date,
            e
        )
    })?;

    let fmt = resolve_format(&app.format, presets)?;

    render_datetime(parsed, &fmt, now, app.timezone)
}

/// Return the chrono format corresponding to `input`.
///
/// *If* `input` matches the name of a preset, that preset’s format is returned;
/// otherwise `input` itself is treated as the format string.
fn resolve_format(input: &str, presets: &[Preset]) -> Result<String> {
    if input.is_empty() {
        return Err(user_input_error!(MissingArgument, "empty --format"));
    }

    Ok(presets
        .iter()
        .find(|p| p.name == input)
        .map(|p| p.format.clone())
        .unwrap_or_else(|| input.to_owned()))
}

/// Convert the parsed result into a `DateTime<Tz>` and format it.
///
/// Any failure in `chrono`’s formatting machinery is converted into a
/// user-visible error.
fn render_datetime(parsed: ParseResult, fmt: &str, now: DateTime<Tz>, tz: Tz) -> Result<String> {
    use std::fmt::Write;

    let naive = match parsed {
        ParseResult::Date(d) => d.and_hms_opt(0, 0, 0).ok_or(std::fmt::Error)?,
        ParseResult::DateTime(dt) => dt,
        ParseResult::Time(t) => chrono::NaiveDateTime::new(now.date_naive(), t),
    };

    let zoned = tz
        .from_local_datetime(&naive)
        .single()
        .ok_or(std::fmt::Error)?;

    // HACK: Safe formatting (captures chrono’s formatting errors as `fmt::Error`)
    let mut out = String::new();
    write!(&mut out, "{}", zoned.format(fmt))?;
    Ok(out)
}

impl App {
    #[inline]
    pub fn new(date: String, format: String, timezone: Tz, now: Option<DateTime<Tz>>) -> Self {
        Self {
            date,
            format,
            timezone,
            now,
        }
    }
}

impl Preset {
    #[inline]
    pub fn new(name: String, format: String) -> Self {
        Self { name, format }
    }
}

impl From<std::fmt::Error> for Error {
    fn from(err: std::fmt::Error) -> Self {
        Error::UserInput(UserInputError::UnsupportedFormat(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    #[test]
    fn resolve_format_returns_preset_when_found() {
        let presets = [
            Preset::new("iso".into(), "%Y-%m-%d".into()),
            Preset::new("time".into(), "%H:%M".into()),
        ];
        let out = super::resolve_format("iso", &presets).unwrap();
        assert_eq!(out, "%Y-%m-%d");
    }

    #[test]
    fn resolve_format_returns_raw_when_not_preset() {
        let presets = [Preset::new("iso".into(), "%Y-%m-%d".into())];
        let out = super::resolve_format("%H:%M", &presets).unwrap();
        assert_eq!(out, "%H:%M");
    }

    #[test]
    fn resolve_format_fails_on_empty() {
        let presets: [Preset; 0] = [];
        assert!(super::resolve_format("", &presets).is_err());
    }

    #[test]
    fn render_datetime_from_date() {
        let ny = chrono_tz::UTC;
        let now = ny.with_ymd_and_hms(2025, 6, 24, 12, 0, 0).unwrap();
        let parsed = ParseResult::Date(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap());
        let out = super::render_datetime(parsed, "%Y-%m-%d", now, ny).unwrap();
        assert_eq!(out, "2025-06-30");
    }

    #[test]
    fn render_datetime_from_time() {
        let tz = chrono_tz::UTC;
        let now = tz.with_ymd_and_hms(2025, 6, 24, 0, 0, 0).unwrap();
        let parsed = ParseResult::Time(NaiveTime::from_hms_opt(15, 30, 0).unwrap());
        let out = super::render_datetime(parsed, "%Y-%m-%dT%H:%M:%S", now, tz).unwrap();
        assert_eq!(out, "2025-06-24T15:30:00");
    }

    #[test]
    fn render_datetime_handles_datetime_directly() {
        let tz = chrono_tz::UTC;
        let now = tz.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let parsed_dt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2030, 1, 15).unwrap(),
            NaiveTime::from_hms_opt(5, 45, 0).unwrap(),
        );
        let parsed = ParseResult::DateTime(parsed_dt);
        let out = super::render_datetime(parsed, "%Y-%m-%d %H:%M", now, tz).unwrap();
        assert_eq!(out, "2030-01-15 05:45");
    }

    #[test]
    fn render_datetime_fails_on_ambiguous_local_time() {
        let tz = chrono_tz::America::New_York;
        let now = tz.with_ymd_and_hms(2025, 11, 1, 12, 0, 0).unwrap();
        let ambiguous = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 11, 2).unwrap(),
            NaiveTime::from_hms_opt(1, 30, 0).unwrap(),
        );
        let parsed = ParseResult::DateTime(ambiguous);

        let err = super::render_datetime(parsed, "%Y-%m-%d %H:%M", now, tz).unwrap_err();
        assert!(matches!(err, Error::UserInput(_)));
    }

    #[test]
    fn process_with_preset_full_flow() {
        let tz = chrono_tz::UTC;
        let app = App::new("2025-06-24 10:00".into(), "iso".into(), tz, None);
        let presets = [Preset::new("iso".into(), "%Y-%m-%dT%H:%M:%S".into())];
        let out = process(&app, &presets).unwrap();
        assert_eq!(out, "2025-06-24T10:00:00");
    }

    #[test]
    fn process_with_raw_format() {
        let tz = chrono_tz::UTC;
        let now = tz.with_ymd_and_hms(2025, 6, 24, 0, 0, 0).unwrap();
        let app = App::new("tomorrow".into(), "%Y-%m-%d".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out, "2025-06-25");
    }

    #[test]
    fn process_errors_on_bad_date_expression() {
        let tz = chrono_tz::UTC;
        let app = App::new("???".into(), "%Y".into(), tz, None);
        assert!(process(&app, &[]).is_err());
    }

    #[test]
    fn process_errors_on_empty_format() {
        let tz = chrono_tz::UTC;
        let app = App::new("today".into(), "".into(), tz, None);
        let err = process(&app, &[]).unwrap_err();
        assert!(matches!(err, Error::UserInput(_)));
    }
}
