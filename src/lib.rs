pub mod cli;
pub mod config;
pub mod core;
pub mod errors;

use core::App;

use chrono_tz::Tz;
use cli::Command;
use config::Config;
pub use errors::{Error, Failable, Result};

impl App {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset};
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    fn cmd(
        input: &str,
        format: Option<&str>,
        timezone: Option<&str>,
        now: Option<&str>,
    ) -> cli::Command {
        cli::Command {
            input: input.to_string(),
            format: format.map(|s| s.to_string()),
            timezone: timezone.map(|s| s.to_string()),
            now: now.map(|s| {
                DateTime::parse_from_rfc3339(s)
                    .unwrap()
                    .with_timezone(&FixedOffset::east_opt(0).unwrap())
            }),
        }
    }

    fn cfg(format: &str, timezone: &str) -> config::Config {
        config::Config {
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
        let cli = cmd("2025-01-01", Some("%Y"), None, None);
        let cfg = cfg("%F", "UTC");

        let app = App::from_cli(&cli, &cfg).unwrap();

        assert_eq!(app.format, "%Y");
        assert_eq!(tz_name(&app.timezone), "UTC");
    }

    #[test]
    fn empty_format_is_error() {
        let cli = cmd("2025-01-01", Some("   "), None, None);
        let cfg = cfg("%F", "UTC");

        let err = App::from_cli(&cli, &cfg).unwrap_err();

        assert!(matches!(
            err,
            Error::UserInput(errors::UserInputError::MissingArgument { .. })
        ));
    }

    #[test]
    fn cli_overrides_config_timezone() {
        let cli = cmd("2025-01-01", Some("%Y"), Some("Europe/London"), None);
        let cfg = cfg("%Y", "UTC");

        let app = App::from_cli(&cli, &cfg).unwrap();

        assert_eq!(tz_name(&app.timezone), "Europe/London");
    }

    #[test]
    fn invalid_timezone_returns_error() {
        let cli = cmd("2025-01-01", Some("%Y"), Some("Mars/Olympus"), None);
        let cfg = cfg("%Y", "UTC");

        let err = App::from_cli(&cli, &cfg).unwrap_err();

        assert!(matches!(
            err,
            Error::UserInput(errors::UserInputError::UnsupportedTimezone { .. })
        ));
    }

    #[test]
    fn preset_name_kept_in_app() {
        let cli = cmd("2030-12-31", Some("br"), None, None);

        let mut fmts = HashMap::new();
        fmts.insert("br".into(), "%d/%m/%Y".into());
        let cfg = config::Config {
            format: "%F".into(),
            timezone: "UTC".into(),
            formats: Some(fmts),
        };

        let app = App::from_cli(&cli, &cfg).unwrap();
        assert_eq!(app.format, "br");
    }
}
