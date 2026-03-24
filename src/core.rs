//! Core transformation logic for **TARDIS**.
//!
//! Converts a natural-language date expression into a formatted string,
//! applying optional presets and an explicit time-zone/context "now".

use chrono::{Datelike, Timelike};
use jiff::{Timestamp, Zoned, civil, tz::TimeZone};
use human_date_parser::{ParseResult, from_human_time};

use crate::{Result, cli::Command, config::Config, user_input_error};

/// Immutable application context passed to [`process`].
#[derive(Debug)]
pub struct App {
    /// Raw human-readable expression (e.g. `"next Friday 10 am"`).
    pub date: String,
    /// Either a strftime-style format string *or* the name of a preset.
    pub format: String,
    /// Target time-zone for output.
    pub timezone: TimeZone,
    /// Optional "now" (useful for deterministic tests).
    pub now: Option<Zoned>,
}

/// Pairing of a **named** preset with a strftime format string.
#[derive(Debug, Clone)]
pub struct Preset {
    pub name: String,
    pub format: String,
}

/// Result of processing a date expression.
#[derive(Debug)]
pub struct ProcessOutput {
    /// Formatted date string.
    pub formatted: String,
    /// Unix epoch timestamp (seconds).
    pub epoch: i64,
}

/// Parse `app.date`, resolve the effective format, and render a string.
///
/// * `presets` is passed as a slice to avoid unnecessary allocation.
/// * All error paths bubble up via [`Result`], ready for unit testing.
pub fn process(app: &App, presets: &[Preset]) -> Result<ProcessOutput> {
    let now = app
        .now
        .clone()
        .unwrap_or_else(|| Zoned::now().with_time_zone(app.timezone.clone()));

    let fmt = resolve_format(&app.format, presets)?;

    // Handle @epoch input syntax.
    if let Some(epoch_str) = app.date.strip_prefix('@') {
        let ts_val: i64 = epoch_str.trim().parse().map_err(|_| {
            user_input_error!(InvalidDateFormat, "invalid epoch timestamp: {}", epoch_str)
        })?;
        let timestamp = Timestamp::from_second(ts_val).map_err(|_| {
            user_input_error!(InvalidDateFormat, "epoch timestamp out of range: {}", ts_val)
        })?;
        let zoned = timestamp.to_zoned(app.timezone.clone());
        let formatted = format_output(&zoned, &fmt)?;
        return Ok(ProcessOutput {
            formatted,
            epoch: zoned.timestamp().as_second(),
        });
    }

    // Bridge: jiff::Zoned -> chrono::NaiveDateTime for human-date-parser
    let now_civil = now.datetime();
    let now_naive = chrono::NaiveDate::from_ymd_opt(
        i32::from(now_civil.year()),
        u32::from(now_civil.month() as u8),
        u32::from(now_civil.day() as u8),
    )
    .ok_or_else(|| {
        user_input_error!(InvalidDate, "internal: invalid date components from jiff")
    })?
    .and_hms_opt(
        u32::from(now_civil.hour() as u8),
        u32::from(now_civil.minute() as u8),
        u32::from(now_civil.second() as u8),
    )
    .ok_or_else(|| {
        user_input_error!(InvalidDate, "internal: invalid time components from jiff")
    })?;

    let parsed = from_human_time(&app.date, now_naive).map_err(|e| {
        user_input_error!(
            InvalidDateFormat,
            "failed to parse human date '{}': {}",
            app.date,
            e
        )
    })?;

    render_datetime(parsed, &fmt, &now, &app.timezone)
}

/// Format a zoned datetime, handling special "epoch"/"unix" format.
fn format_output(zoned: &Zoned, fmt: &str) -> Result<String> {
    if fmt == "epoch" || fmt == "unix" {
        return Ok(zoned.timestamp().as_second().to_string());
    }

    let output = zoned.strftime(fmt).to_string();

    // Validate that the format string was meaningful: if the output contains
    // an unrecognized specifier (i.e., jiff passes through unknown % sequences
    // as literals), detect and error on unknown %-specifiers for backward compat.
    validate_format_output(fmt, &output)?;

    Ok(output)
}

/// Detect unknown strftime specifiers by checking if any `%X` sequence
/// in the format string was passed through unchanged to the output.
///
/// jiff's strftime passes through unrecognized specifiers as literals,
/// but we want to error on them (matching chrono's old behavior).
fn validate_format_output(fmt: &str, output: &str) -> Result<()> {
    // Known strftime specifiers (both jiff and POSIX)
    const KNOWN_SPECIFIERS: &[char] = &[
        'A', 'a', 'B', 'b', 'C', 'c', 'D', 'd', 'e', 'F', 'G', 'g',
        'H', 'h', 'I', 'j', 'k', 'l', 'M', 'm', 'N', 'n', 'P', 'p',
        'R', 'r', 'S', 's', 'T', 't', 'U', 'u', 'V', 'v', 'W', 'w',
        'X', 'x', 'Y', 'y', 'Z', 'z',
        // jiff extensions
        'f',
        // Modifiers (these precede another specifier)
        '-', '0', '_', ':',
        // Literal percent
        '%',
    ];

    let bytes = fmt.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            i += 1;
            if i >= bytes.len() {
                // Trailing % at end of format
                return Err(user_input_error!(
                    UnsupportedFormat,
                    "invalid format string: {}",
                    fmt
                ));
            }
            // Skip modifiers
            while i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'0' || bytes[i] == b'_') {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(user_input_error!(
                    UnsupportedFormat,
                    "invalid format string: {}",
                    fmt
                ));
            }
            // Handle %: prefix (for %:z, %::z, etc.)
            if bytes[i] == b':' {
                i += 1;
                // skip additional colons for %::z, %:::z
                while i < bytes.len() && bytes[i] == b':' {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'z' {
                    i += 1;
                    continue;
                }
                return Err(user_input_error!(
                    UnsupportedFormat,
                    "invalid format string: {}",
                    fmt
                ));
            }
            let c = bytes[i] as char;
            if !KNOWN_SPECIFIERS.contains(&c) {
                return Err(user_input_error!(
                    UnsupportedFormat,
                    "invalid format string: {}",
                    fmt
                ));
            }
        }
        i += 1;
    }
    let _ = output; // output not needed for format-level validation
    Ok(())
}

/// Return the format string corresponding to `input`.
///
/// *If* `input` matches the name of a preset, that preset's format is returned;
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

/// Convert the parsed result into a `Zoned` datetime and format it.
fn render_datetime(
    parsed: ParseResult,
    fmt: &str,
    now: &Zoned,
    tz: &TimeZone,
) -> Result<ProcessOutput> {
    let civil_dt = match parsed {
        ParseResult::Date(d) => {
            civil::date(
                d.year() as i16,
                d.month() as i8,
                d.day() as i8,
            )
            .at(0, 0, 0, 0)
        }
        ParseResult::DateTime(dt) => {
            civil::date(
                dt.date().year() as i16,
                dt.date().month() as i8,
                dt.date().day() as i8,
            )
            .at(
                dt.time().hour() as i8,
                dt.time().minute() as i8,
                dt.time().second() as i8,
                0,
            )
        }
        ParseResult::Time(t) => {
            let today = now.datetime().date();
            today.at(
                t.hour() as i8,
                t.minute() as i8,
                t.second() as i8,
                0,
            )
        }
    };

    let ambiguous = tz.to_ambiguous_zoned(civil_dt);
    if ambiguous.is_ambiguous() {
        return Err(user_input_error!(
            AmbiguousDateTime,
            "the datetime {} is ambiguous or invalid in timezone {}",
            civil_dt,
            tz.iana_name().unwrap_or("Unknown")
        ));
    }
    let zoned = ambiguous.compatible().map_err(|_| {
        user_input_error!(
            AmbiguousDateTime,
            "the datetime {} is invalid in timezone {}",
            civil_dt,
            tz.iana_name().unwrap_or("Unknown")
        )
    })?;

    let formatted = format_output(&zoned, fmt)?;
    Ok(ProcessOutput {
        formatted,
        epoch: zoned.timestamp().as_second(),
    })
}

impl App {
    #[inline]
    pub fn new(date: String, format: String, timezone: TimeZone, now: Option<Zoned>) -> Self {
        Self {
            date,
            format,
            timezone,
            now,
        }
    }

    /// Build an [`App`] from the parsed CLI and loaded configuration.
    ///
    /// * CLI values **override** config values.
    /// * If no time-zone is provided anywhere, falls back to the OS local TZ.
    pub fn from_cli(cmd: &Command, cfg: &Config) -> Result<Self> {
        let format = cmd.format.clone().unwrap_or_else(|| cfg.format.clone());

        if format.trim().is_empty() {
            return Err(user_input_error!(
                MissingArgument,
                "no output format specified"
            ));
        }

        let tz_raw = cmd
            .timezone
            .clone()
            .unwrap_or_else(|| cfg.timezone.clone())
            .trim()
            .to_owned();

        let timezone: TimeZone = if tz_raw.is_empty() {
            TimeZone::system()
        } else {
            TimeZone::get(&tz_raw).map_err(|_| {
                user_input_error!(UnsupportedTimezone, "invalid timezone ID: {}", tz_raw)
            })?
        };

        let now = cmd.now.map(|ts| ts.to_zoned(timezone.clone()));

        Ok(Self::new(cmd.input.clone(), format, timezone, now))
    }
}

impl Preset {
    #[inline]
    pub fn new(name: String, format: String) -> Self {
        Self { name, format }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::Error;
    use jiff::{Timestamp, tz::TimeZone};
    use human_date_parser::ParseResult;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    fn utc() -> TimeZone {
        TimeZone::get("UTC").unwrap()
    }

    fn zoned_utc(year: i16, month: i8, day: i8, hour: i8, min: i8, sec: i8) -> Zoned {
        let dt = civil::date(year, month, day).at(hour, min, sec, 0);
        utc().to_ambiguous_zoned(dt).compatible().unwrap()
    }

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
        let tz = utc();
        let now = zoned_utc(2025, 6, 24, 12, 0, 0);
        let parsed = ParseResult::Date(chrono::NaiveDate::from_ymd_opt(2025, 6, 30).unwrap());
        let out = super::render_datetime(parsed, "%Y-%m-%d", &now, &tz)
            .unwrap()
            .formatted;
        assert_eq!(out, "2025-06-30");
    }

    #[test]
    fn render_datetime_from_time() {
        let tz = utc();
        let now = zoned_utc(2025, 6, 24, 0, 0, 0);
        let parsed = ParseResult::Time(chrono::NaiveTime::from_hms_opt(15, 30, 0).unwrap());
        let out = super::render_datetime(parsed, "%Y-%m-%dT%H:%M:%S", &now, &tz)
            .unwrap()
            .formatted;
        assert_eq!(out, "2025-06-24T15:30:00");
    }

    #[test]
    fn render_datetime_handles_datetime_directly() {
        let tz = utc();
        let now = zoned_utc(2025, 1, 1, 0, 0, 0);
        let parsed_dt = chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2030, 1, 15).unwrap(),
            chrono::NaiveTime::from_hms_opt(5, 45, 0).unwrap(),
        );
        let parsed = ParseResult::DateTime(parsed_dt);
        let out = super::render_datetime(parsed, "%Y-%m-%d %H:%M", &now, &tz)
            .unwrap()
            .formatted;
        assert_eq!(out, "2030-01-15 05:45");
    }

    #[test]
    fn render_datetime_fails_on_ambiguous_local_time() {
        let tz = TimeZone::get("America/New_York").unwrap();
        let now = {
            let dt = civil::date(2025, 11, 1).at(12, 0, 0, 0);
            tz.to_ambiguous_zoned(dt).compatible().unwrap()
        };
        let ambiguous = chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2025, 11, 2).unwrap(),
            chrono::NaiveTime::from_hms_opt(1, 30, 0).unwrap(),
        );
        let parsed = ParseResult::DateTime(ambiguous);

        let err = super::render_datetime(parsed, "%Y-%m-%d %H:%M", &now, &tz).unwrap_err();
        assert!(matches!(
            err,
            Error::UserInput(crate::errors::UserInputError::AmbiguousDateTime(_))
        ));
    }

    #[test]
    fn process_with_preset_full_flow() {
        let tz = utc();
        let app = App::new("2025-06-24 10:00".into(), "iso".into(), tz, None);
        let presets = [Preset::new("iso".into(), "%Y-%m-%dT%H:%M:%S".into())];
        let out = process(&app, &presets).unwrap();
        assert_eq!(out.formatted, "2025-06-24T10:00:00");
    }

    #[test]
    fn process_with_raw_format() {
        let tz = utc();
        let now = zoned_utc(2025, 6, 24, 0, 0, 0);
        let app = App::new("tomorrow".into(), "%Y-%m-%d".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "2025-06-25");
    }

    #[test]
    fn process_errors_on_bad_date_expression() {
        let tz = utc();
        let app = App::new("???".into(), "%Y".into(), tz, None);
        assert!(process(&app, &[]).is_err());
    }

    #[test]
    fn process_errors_on_empty_format() {
        let tz = utc();
        let app = App::new("today".into(), "".into(), tz, None);
        let err = process(&app, &[]).unwrap_err();
        assert!(matches!(err, Error::UserInput(_)));
    }

    // --- from_cli tests ---

    fn make_cmd(
        input: &str,
        format: Option<&str>,
        timezone: Option<&str>,
        now: Option<&str>,
    ) -> Command {
        Command {
            input: input.to_string(),
            format: format.map(|s| s.to_string()),
            timezone: timezone.map(|s| s.to_string()),
            now: now.map(|s| s.parse::<Timestamp>().unwrap()),
            json: false,
            no_newline: false,
        }
    }

    fn make_cfg(format: &str, timezone: &str) -> Config {
        Config {
            format: format.to_string(),
            timezone: timezone.to_string(),
            formats: None,
        }
    }

    fn tz_name(tz: &TimeZone) -> &str {
        tz.iana_name().unwrap_or("Unknown")
    }

    #[test]
    fn cli_overrides_config_format() {
        let cli = make_cmd("2025-01-01", Some("%Y"), None, None);
        let cfg = make_cfg("%F", "UTC");
        let app = App::from_cli(&cli, &cfg).unwrap();
        assert_eq!(app.format, "%Y");
        assert_eq!(tz_name(&app.timezone), "UTC");
    }

    #[test]
    fn empty_format_is_error() {
        let cli = make_cmd("2025-01-01", Some("   "), None, None);
        let cfg = make_cfg("%F", "UTC");
        let err = App::from_cli(&cli, &cfg).unwrap_err();
        assert!(matches!(
            err,
            Error::UserInput(crate::errors::UserInputError::MissingArgument { .. })
        ));
    }

    #[test]
    fn cli_overrides_config_timezone() {
        let cli = make_cmd("2025-01-01", Some("%Y"), Some("Europe/London"), None);
        let cfg = make_cfg("%Y", "UTC");
        let app = App::from_cli(&cli, &cfg).unwrap();
        assert_eq!(tz_name(&app.timezone), "Europe/London");
    }

    #[test]
    fn invalid_timezone_returns_error() {
        let cli = make_cmd("2025-01-01", Some("%Y"), Some("Mars/Olympus"), None);
        let cfg = make_cfg("%Y", "UTC");
        let err = App::from_cli(&cli, &cfg).unwrap_err();
        assert!(matches!(
            err,
            Error::UserInput(crate::errors::UserInputError::UnsupportedTimezone { .. })
        ));
    }

    #[test]
    fn preset_name_kept_in_app() {
        let cli = make_cmd("2030-12-31", Some("br"), None, None);
        let mut fmts = HashMap::new();
        fmts.insert("br".into(), "%d/%m/%Y".into());
        let cfg = Config {
            format: "%F".into(),
            timezone: "UTC".into(),
            formats: Some(fmts),
        };
        let app = App::from_cli(&cli, &cfg).unwrap();
        assert_eq!(app.format, "br");
    }

    #[test]
    fn from_cli_with_now_override() {
        let cli = make_cmd(
            "today",
            Some("%Y"),
            Some("UTC"),
            Some("2025-06-24T12:00:00Z"),
        );
        let cfg = make_cfg("%Y", "UTC");
        let app = App::from_cli(&cli, &cfg).unwrap();
        assert!(app.now.is_some());
    }

    // --- Epoch input tests ---

    #[test]
    fn epoch_input_valid() {
        let tz = utc();
        let app = App::new("@1735689600".into(), "%Y-%m-%d".into(), tz, None);
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "2025-01-01");
        assert_eq!(out.epoch, 1735689600);
    }

    #[test]
    fn epoch_input_invalid_not_a_number() {
        let tz = utc();
        let app = App::new("@abc".into(), "%Y".into(), tz, None);
        let err = process(&app, &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::UserInput(crate::errors::UserInputError::InvalidDateFormat(_))
        ));
    }

    #[test]
    fn epoch_input_out_of_range() {
        let tz = utc();
        let app = App::new("@99999999999999999".into(), "%Y".into(), tz, None);
        let err = process(&app, &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::UserInput(crate::errors::UserInputError::InvalidDateFormat(_))
        ));
    }

    #[test]
    fn epoch_output_format() {
        let tz = utc();
        let now = zoned_utc(2025, 1, 1, 0, 0, 0);
        let app = App::new("today".into(), "epoch".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "1735689600");
    }

    #[test]
    fn unix_output_format() {
        let tz = utc();
        let now = zoned_utc(2025, 1, 1, 0, 0, 0);
        let app = App::new("today".into(), "unix".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "1735689600");
    }

    #[test]
    fn epoch_input_with_epoch_output() {
        let tz = utc();
        let app = App::new("@1735689600".into(), "epoch".into(), tz, None);
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "1735689600");
        assert_eq!(out.epoch, 1735689600);
    }

    #[test]
    fn process_output_includes_epoch() {
        let tz = utc();
        let now = zoned_utc(2025, 6, 24, 0, 0, 0);
        let app = App::new("tomorrow".into(), "%Y-%m-%d".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "2025-06-25");
        assert_eq!(out.epoch, 1750809600);
    }

    #[test]
    fn epoch_negative_timestamp() {
        let tz = utc();
        let app = App::new("@-86400".into(), "%Y-%m-%d".into(), tz, None);
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "1969-12-31");
    }

    // --- format_output edge cases ---

    #[test]
    fn format_output_with_literal_text() {
        let zoned = zoned_utc(2025, 1, 1, 0, 0, 0);
        let out = super::format_output(&zoned, "Year: %Y").unwrap();
        assert_eq!(out, "Year: 2025");
    }

    #[test]
    fn format_output_epoch() {
        let zoned = zoned_utc(2025, 1, 1, 0, 0, 0);
        let out = super::format_output(&zoned, "epoch").unwrap();
        assert_eq!(out, "1735689600");
    }

    #[test]
    fn format_output_unix() {
        let zoned = zoned_utc(2025, 1, 1, 0, 0, 0);
        let out = super::format_output(&zoned, "unix").unwrap();
        assert_eq!(out, "1735689600");
    }
}
