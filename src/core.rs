//! Core transformation logic for **TARDIS**.
//!
//! Converts a natural-language date expression into a formatted string,
//! applying optional presets and an explicit time-zone/context "now".

use jiff::{Zoned, tz::TimeZone};

use crate::{Result, cli::Command, config::Config, locale, parser, user_input_error};

/// Immutable application context passed to [`process`].
#[non_exhaustive]
#[derive(Debug)]
pub struct App {
    /// Raw human-readable expression (e.g. `"next Friday 10 am"`).
    pub date: String,
    /// Either a strftime-style format string *or* the name of a preset.
    pub format: String,
    /// Target time-zone for output.
    pub timezone: TimeZone,
    /// Locale code for input parsing (e.g. "en", "pt").
    pub locale_code: String,
    /// Optional "now" (useful for deterministic tests).
    pub now: Option<Zoned>,
}

/// Pairing of a **named** preset with a strftime format string.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Preset {
    pub name: String,
    pub format: String,
}

/// Result of processing a date expression.
#[non_exhaustive]
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

    // Resolve locale and build keyword table for parser
    let locale_ref = locale::get_locale(&app.locale_code);
    let locale_kw = locale::LocaleKeywords::from_locale(locale_ref);

    // Parse with detected locale; fall back to EN if non-EN locale fails
    let zoned = match parser::parse(&app.date, &now, &locale_kw) {
        Ok(z) => z,
        Err(e) if app.locale_code != "en" => {
            let en_ref = locale::get_locale("en");
            let en_kw = locale::LocaleKeywords::from_locale(en_ref);
            parser::parse(&app.date, &now, &en_kw)
                .map_err(|_| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?
        }
        Err(e) => {
            return Err(user_input_error!(
                InvalidDateFormat,
                "{}",
                e.format_message()
            ));
        }
    };

    let formatted = format_output(&zoned, &fmt)?;
    Ok(ProcessOutput {
        formatted,
        epoch: zoned.timestamp().as_second(),
    })
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
        'A', 'a', 'B', 'b', 'C', 'c', 'D', 'd', 'e', 'F', 'G', 'g', 'H', 'h', 'I', 'j', 'k', 'l',
        'M', 'm', 'N', 'n', 'P', 'p', 'R', 'r', 'S', 's', 'T', 't', 'U', 'u', 'V', 'v', 'W', 'w',
        'X', 'x', 'Y', 'y', 'Z', 'z', // jiff extensions
        'f', // Modifiers (these precede another specifier)
        '-', '0', '_', ':', // Literal percent
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

impl App {
    #[inline]
    pub fn new(date: String, format: String, timezone: TimeZone, now: Option<Zoned>) -> Self {
        Self {
            date,
            format,
            timezone,
            locale_code: "en".to_string(),
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

        // Resolve locale via D-06 precedence: CLI > config > env > EN
        let locale_ref = locale::resolve_locale(cmd.locale.as_deref(), cfg.locale.as_deref());
        let locale_code = locale_ref.code().to_string();

        let now = cmd.now.map(|ts| ts.to_zoned(timezone.clone()));

        Ok(Self {
            date: cmd.input.clone(),
            format,
            timezone,
            locale_code,
            now,
        })
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
    use jiff::{Timestamp, civil, tz::TimeZone};
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
            locale: None,
            now: now.map(|s| s.parse::<Timestamp>().unwrap()),
            json: false,
            no_newline: false,
            verbose: false,
            skip_errors: false,
        }
    }

    fn make_cfg(format: &str, timezone: &str) -> Config {
        Config {
            format: format.to_string(),
            timezone: timezone.to_string(),
            locale: None,
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
            locale: None,
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
    fn epoch_input_smart_precision() {
        // The new parser detects 99999999999999999 (17 digits) as microseconds
        // and resolves it to a valid datetime (~year 5138).
        let tz = utc();
        let app = App::new("@99999999999999999".into(), "%Y".into(), tz, None);
        let out = process(&app, &[]).unwrap();
        // 99999999999999999 microseconds = ~year 5138
        assert!(!out.formatted.is_empty());
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
