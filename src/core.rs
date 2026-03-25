//! Core transformation logic for **TARDIS**.
//!
//! Converts a natural-language date expression into a formatted string,
//! applying optional presets and an explicit time-zone/context “now”.

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use human_date_parser::{ParseResult, from_human_time};

use crate::{Result, cli::Command, config::Config, system_error, user_input_error};

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
        .unwrap_or_else(|| app.timezone.from_utc_datetime(&Utc::now().naive_utc()));

    let fmt = resolve_format(&app.format, presets)?;

    // Handle @epoch input syntax.
    if let Some(epoch_str) = app.date.strip_prefix('@') {
        let ts: i64 = epoch_str.trim().parse().map_err(|_| {
            user_input_error!(InvalidDateFormat, "invalid epoch timestamp: {}", epoch_str)
        })?;
        let dt = DateTime::from_timestamp(ts, 0).ok_or_else(|| {
            user_input_error!(InvalidDateFormat, "epoch timestamp out of range: {}", ts)
        })?;
        let zoned = dt.with_timezone(&app.timezone);
        let formatted = format_output(zoned, &fmt)?;
        return Ok(ProcessOutput {
            formatted,
            epoch: zoned.timestamp(),
        });
    }

    let parsed = from_human_time(&app.date, now.naive_local()).map_err(|e| {
        user_input_error!(
            InvalidDateFormat,
            "failed to parse human date '{}': {}",
            app.date,
            e
        )
    })?;

    render_datetime(parsed, &fmt, now, app.timezone)
}

/// Format a zoned datetime, handling special "epoch"/"unix" format.
fn format_output(zoned: DateTime<Tz>, fmt: &str) -> Result<String> {
    if fmt == "epoch" || fmt == "unix" {
        return Ok(zoned.timestamp().to_string());
    }
    use std::fmt::Write;
    let mut out = String::new();
    write!(&mut out, "{}", zoned.format(fmt))
        .map_err(|_| user_input_error!(UnsupportedFormat, "invalid format string: {}", fmt))?;
    Ok(out)
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
fn render_datetime(
    parsed: ParseResult,
    fmt: &str,
    now: DateTime<Tz>,
    tz: Tz,
) -> Result<ProcessOutput> {
    let naive = match parsed {
        ParseResult::Date(d) => d.and_hms_opt(0, 0, 0).ok_or_else(|| {
            user_input_error!(InvalidDate, "could not construct midnight for date {}", d)
        })?,
        ParseResult::DateTime(dt) => dt,
        ParseResult::Time(t) => chrono::NaiveDateTime::new(now.date_naive(), t),
    };

    let zoned = tz.from_local_datetime(&naive).single().ok_or_else(|| {
        user_input_error!(
            AmbiguousDateTime,
            "the datetime {} is ambiguous or invalid in timezone {}",
            naive,
            tz.name()
        )
    })?;

    let formatted = format_output(zoned, fmt)?;
    Ok(ProcessOutput {
        formatted,
        epoch: zoned.timestamp(),
    })
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

        let timezone: Tz = if tz_raw.is_empty() {
            let local = iana_time_zone::get_timezone()
                .map_err(|e| system_error!(Config, "failed to read local timezone: {}", e))?;
            local.parse().map_err(|_| {
                user_input_error!(UnsupportedTimezone, "invalid timezone ID: {}", local)
            })?
        } else {
            tz_raw.parse().map_err(|_| {
                user_input_error!(UnsupportedTimezone, "invalid timezone ID: {}", tz_raw)
            })?
        };

        let now = cmd.now.map(|dt| dt.with_timezone(&timezone));

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
    use super::*;
    use crate::Error;
    use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

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
        let out = super::render_datetime(parsed, "%Y-%m-%d", now, ny)
            .unwrap()
            .formatted;
        assert_eq!(out, "2025-06-30");
    }

    #[test]
    fn render_datetime_from_time() {
        let tz = chrono_tz::UTC;
        let now = tz.with_ymd_and_hms(2025, 6, 24, 0, 0, 0).unwrap();
        let parsed = ParseResult::Time(NaiveTime::from_hms_opt(15, 30, 0).unwrap());
        let out = super::render_datetime(parsed, "%Y-%m-%dT%H:%M:%S", now, tz)
            .unwrap()
            .formatted;
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
        let out = super::render_datetime(parsed, "%Y-%m-%d %H:%M", now, tz)
            .unwrap()
            .formatted;
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
        assert!(matches!(
            err,
            Error::UserInput(crate::errors::UserInputError::AmbiguousDateTime(_))
        ));
    }

    #[test]
    fn process_with_preset_full_flow() {
        let tz = chrono_tz::UTC;
        let app = App::new("2025-06-24 10:00".into(), "iso".into(), tz, None);
        let presets = [Preset::new("iso".into(), "%Y-%m-%dT%H:%M:%S".into())];
        let out = process(&app, &presets).unwrap();
        assert_eq!(out.formatted, "2025-06-24T10:00:00");
    }

    #[test]
    fn process_with_raw_format() {
        let tz = chrono_tz::UTC;
        let now = tz.with_ymd_and_hms(2025, 6, 24, 0, 0, 0).unwrap();
        let app = App::new("tomorrow".into(), "%Y-%m-%d".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "2025-06-25");
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
            now: now.map(|s| {
                DateTime::parse_from_rfc3339(s)
                    .unwrap()
                    .with_timezone(&FixedOffset::east_opt(0).unwrap())
            }),
            json: false,
            no_newline: false,
            verbose: false,
        }
    }

    fn make_cfg(format: &str, timezone: &str) -> Config {
        Config {
            format: format.to_string(),
            timezone: timezone.to_string(),
            formats: None,
        }
    }

    fn tz_name(tz: &Tz) -> &'static str {
        tz.name()
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
        let tz = chrono_tz::UTC;
        let app = App::new("@1735689600".into(), "%Y-%m-%d".into(), tz, None);
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "2025-01-01");
        assert_eq!(out.epoch, 1735689600);
    }

    #[test]
    fn epoch_input_invalid_not_a_number() {
        let tz = chrono_tz::UTC;
        let app = App::new("@abc".into(), "%Y".into(), tz, None);
        let err = process(&app, &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::UserInput(crate::errors::UserInputError::InvalidDateFormat(_))
        ));
    }

    #[test]
    fn epoch_input_out_of_range() {
        let tz = chrono_tz::UTC;
        let app = App::new("@99999999999999999".into(), "%Y".into(), tz, None);
        let err = process(&app, &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::UserInput(crate::errors::UserInputError::InvalidDateFormat(_))
        ));
    }

    #[test]
    fn epoch_output_format() {
        let tz = chrono_tz::UTC;
        let now = tz.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let app = App::new("today".into(), "epoch".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "1735689600");
    }

    #[test]
    fn unix_output_format() {
        let tz = chrono_tz::UTC;
        let now = tz.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let app = App::new("today".into(), "unix".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "1735689600");
    }

    #[test]
    fn epoch_input_with_epoch_output() {
        let tz = chrono_tz::UTC;
        let app = App::new("@1735689600".into(), "epoch".into(), tz, None);
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "1735689600");
        assert_eq!(out.epoch, 1735689600);
    }

    #[test]
    fn process_output_includes_epoch() {
        let tz = chrono_tz::UTC;
        let now = tz.with_ymd_and_hms(2025, 6, 24, 0, 0, 0).unwrap();
        let app = App::new("tomorrow".into(), "%Y-%m-%d".into(), tz, Some(now));
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "2025-06-25");
        assert_eq!(out.epoch, 1750809600);
    }

    #[test]
    fn epoch_negative_timestamp() {
        let tz = chrono_tz::UTC;
        let app = App::new("@-86400".into(), "%Y-%m-%d".into(), tz, None);
        let out = process(&app, &[]).unwrap();
        assert_eq!(out.formatted, "1969-12-31");
    }

    // --- format_output edge cases ---

    #[test]
    fn format_output_with_literal_text() {
        let tz = chrono_tz::UTC;
        let dt = tz.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let out = super::format_output(dt, "Year: %Y").unwrap();
        assert_eq!(out, "Year: 2025");
    }

    #[test]
    fn format_output_epoch() {
        let tz = chrono_tz::UTC;
        let dt = tz.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let out = super::format_output(dt, "epoch").unwrap();
        assert_eq!(out, "1735689600");
    }

    #[test]
    fn format_output_unix() {
        let tz = chrono_tz::UTC;
        let dt = tz.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let out = super::format_output(dt, "unix").unwrap();
        assert_eq!(out, "1735689600");
    }
}
