use std::{
    env,
    ffi::OsString,
    io::{self, IsTerminal, Read},
};

use chrono::{DateTime, FixedOffset};
use clap::Parser;

use crate::{Result, user_input_error};

/// TARDIS — Time And Relative Date Input Simplifier
///
/// Translates natural-language time expressions into formatted datetimes.
///
/// A lightweight CLI tool for converting human-readable date and time phrases
/// like "next Friday at 2:00" or "in 3 days" into machine-usable output.
#[derive(Debug, Parser)]
#[command(
    name = "td",
    about,
    version,
    color = clap::ColorChoice::Auto,
    after_long_help = r#"
Environment Variables:
    TARDIS_FORMAT     Default output format or preset name.
    TARDIS_TIMEZONE   Default IANA time zone (e.g. America/Sao_Paulo).

Configuration File:
    $XDG_CONFIG_HOME/tardis/config.toml

    if XDG_CONFIG_HOME is unset:
        • Linux:   ~/.config/tardis/config.toml
        • macOS:   ~/Library/Application Support/tardis/config.toml
        • Windows: %APPDATA%\tardis\config.toml

    The file is created automatically on first run and contains commented
    examples for every field.

Precedence:
    CLI flags → environment variables → configuration file

For more information, visit https://github.com/hvpaiva/tardis
"#,
    after_help = "For more information, visit https://github.com/hvpaiva/tardis",
)]
pub struct Cli {
    /// A natural-language expression like "next Friday at 9:30".
    /// If omitted, the value is read from STDIN.
    ///
    /// Check the human-date-parser formats:
    /// https://github.com/technologicalMayhem/human-date-parser?tab=readme-ov-file#formats
    input: Option<String>,
    /// Output format.
    ///
    /// Accepts chrono-style formatting (e.g. "%Y-%m-%d")
    /// or a named format defined in the config file.
    ///
    /// Reference:
    /// https://docs.rs/chrono/latest/chrono/format/strftime/index.html
    ///
    /// If  not provided, trys to read from the environment variable `TARDIS_FORMAT` and
    /// falls back to the default format defined in the config file.
    #[arg(value_name = "FMT", short, long)]
    format: Option<String>,
    /// Time-zone to apply (IANA/Olson ID). If not provided, uses system local time.
    ///
    /// Examples: "UTC", "America/Sao_Paulo", "Europe/London".
    ///
    /// Reference:
    /// https://docs.rs/chrono-tz/latest/chrono_tz/enum.Tz.html
    ///
    /// If not provided, trys to read from the environment variable `TARDIS_TIMEZONE` and
    /// falls back to the default time zone defined in the config file.
    #[arg(value_name = "TZ", short, long)]
    timezone: Option<String>,
    /// Override “now”. Format **RFC 3339**, e.g. `2025-06-24T09:00:00Z`.
    #[arg(value_name = "DATETIME", long)]
    now: Option<String>,
}

/// Normalised user command ready for further processing.
#[derive(Debug)]
pub struct Command {
    pub input: String,
    pub format: Option<String>,
    pub timezone: Option<String>,
    pub now: Option<DateTime<FixedOffset>>,
}

impl Command {
    /// Parse from arbitrary arg iterator **and** an arbitrary reader for STDIN.
    /// Makes unit-testing easier by allowing injection of fake inputs.
    pub fn parse_from<I, S, R>(args: I, mut stdin: R) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString> + Clone,
        R: Read,
    {
        let cli = Cli::parse_from(args);
        Self::from_cli(cli, &mut stdin)
    }

    /// Parse using the real `env::args_os()` and the real `io::stdin()`.
    /// This is what the binary calls from `main`.
    pub fn parse() -> Result<Self> {
        Self::parse_from(env::args_os(), io::stdin())
    }

    /// Internal helper that converts a `Cli` into `Command`,
    /// reading STDIN if necessary.
    fn from_cli<R: Read>(cli: Cli, mut stdin: R) -> Result<Self> {
        let input = match cli.input {
            Some(s) if !s.is_empty() => s,
            None if !io::stdin().is_terminal() => {
                let mut buf = String::new();
                stdin.read_to_string(&mut buf).map_err(|e| {
                    user_input_error!(InvalidDateFormat, "failed to read from stdin: {}", e)
                })?;
                let trimmed = buf.trim();
                if trimmed.is_empty() {
                    return Err(user_input_error!(
                        InvalidDateFormat,
                        "no input provided in stdin; pass an argument or pipe data"
                    ));
                }
                trimmed.to_owned()
            }
            _ => {
                return Err(user_input_error!(
                    InvalidDateFormat,
                    "no input provided; pass an argument or pipe data"
                ));
            }
        };

        let now = cli
            .now
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|e| {
                user_input_error!(
                    InvalidNow,
                    "{} (expect RFC 3339, ex.: 2025-06-24T12:00:00Z)",
                    e
                )
            })?;

        Ok(Command {
            input,
            format: cli.format,
            timezone: cli.timezone,
            now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Cursor;

    fn parse_ok(argv: &[&str]) -> Command {
        Command::parse_from(argv, Cursor::new("")).expect("parse should succeed")
    }

    #[test]
    fn parses_all_flags() {
        let cmd = parse_ok(&[
            "td",
            "next friday",
            "-f",
            "%Y",
            "-t",
            "UTC",
            "--now",
            "2025-06-24T12:00:00Z",
        ]);

        assert_eq!(cmd.input, "next friday");
        assert_eq!(cmd.format.as_deref(), Some("%Y"));
        assert_eq!(cmd.timezone.as_deref(), Some("UTC"));
        assert_eq!(
            cmd.now,
            Some(DateTime::parse_from_rfc3339("2025-06-24T12:00:00Z").unwrap())
        );
    }

    #[test]
    fn defaults_none_when_only_input() {
        let cmd = parse_ok(&["td", "tomorrow"]);
        assert_eq!(cmd.format, None);
        assert_eq!(cmd.timezone, None);
        assert_eq!(cmd.now, None);
    }

    #[test]
    fn arg_takes_precedence_over_stdin() {
        let cmd = Command::parse_from(["td", "next monday"], Cursor::new("ignored")).unwrap();
        assert_eq!(cmd.input, "next monday");
    }

    #[test]
    fn stdin_empty_in_unit_path_gives_missing_input() {
        let err = Command::parse_from(["td"], Cursor::new("")).unwrap_err();
        use crate::{Error, errors::UserInputError};
        assert!(matches!(
            err,
            Error::UserInput(UserInputError::InvalidDateFormat(_))
        ));
    }
}
