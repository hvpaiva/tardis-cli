use std::{
    env,
    ffi::OsString,
    io::{self, IsTerminal, Read},
};

use clap::Parser;
use jiff::Timestamp;

use crate::{Result, user_input_error};

#[path = "cli_defs.rs"]
mod cli_defs_mod;
pub use cli_defs_mod::*;

/// Normalised user command ready for further processing.
#[non_exhaustive]
#[derive(Debug)]
pub struct Command {
    pub input: String,
    pub format: Option<String>,
    pub timezone: Option<String>,
    pub now: Option<Timestamp>,
    pub json: bool,
    pub no_newline: bool,
    pub verbose: bool,
    pub skip_errors: bool,
}

impl Command {
    /// Create a new Command with a different input, preserving all other fields.
    /// Used in batch mode to avoid manual field cloning.
    pub fn with_input(&self, input: String) -> Self {
        Command {
            input,
            format: self.format.clone(),
            timezone: self.timezone.clone(),
            now: self.now,
            json: self.json,
            no_newline: self.no_newline,
            verbose: self.verbose,
            skip_errors: self.skip_errors,
        }
    }
}

impl Command {
    /// Parse from arbitrary arg iterator **and** an arbitrary reader for STDIN.
    /// The `stdin_is_terminal` flag controls whether we attempt to read from
    /// the reader when no positional argument is given.
    pub fn parse_from<I, S, R>(args: I, stdin: R, stdin_is_terminal: bool) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString> + Clone,
        R: Read,
    {
        let cli = Cli::parse_from(args);
        Self::from_cli(cli, stdin, stdin_is_terminal)
    }

    /// Parse using the real `env::args_os()` and the real `io::stdin()`.
    pub fn parse() -> Result<Self> {
        let is_terminal = io::stdin().is_terminal();
        Self::parse_from(env::args_os(), io::stdin(), is_terminal)
    }

    /// Converts a `Cli` into `Command`, reading STDIN if necessary.
    pub fn from_raw_cli<R: Read>(cli: Cli, stdin: R, stdin_is_terminal: bool) -> Result<Self> {
        Self::from_cli(cli, stdin, stdin_is_terminal)
    }

    fn from_cli<R: Read>(cli: Cli, mut stdin: R, stdin_is_terminal: bool) -> Result<Self> {
        let input = match cli.input {
            Some(s) if !s.is_empty() => s,
            None if !stdin_is_terminal => {
                let mut buf = String::new();
                stdin.read_to_string(&mut buf).map_err(|e| {
                    user_input_error!(InvalidDateFormat, "failed to read from stdin: {}", e)
                })?;
                let trimmed = buf.trim();
                if trimmed.is_empty() {
                    "now".to_owned()
                } else {
                    trimmed.to_owned()
                }
            }
            _ => "now".to_owned(),
        };

        let now = cli
            .now
            .as_deref()
            .map(|s| s.parse::<Timestamp>())
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
            json: cli.json,
            no_newline: cli.no_newline,
            verbose: cli.verbose,
            skip_errors: cli.skip_errors,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Cursor;

    fn parse_ok(argv: &[&str]) -> Command {
        Command::parse_from(argv, Cursor::new(""), true).expect("parse should succeed")
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
            Some("2025-06-24T12:00:00Z".parse::<Timestamp>().unwrap())
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
        let cmd =
            Command::parse_from(["td", "next monday"], Cursor::new("ignored"), false).unwrap();
        assert_eq!(cmd.input, "next monday");
    }

    #[test]
    fn no_args_terminal_defaults_to_now() {
        let cmd = Command::parse_from(["td"], Cursor::new(""), true).unwrap();
        assert_eq!(cmd.input, "now");
    }

    #[test]
    fn empty_stdin_defaults_to_now() {
        let cmd = Command::parse_from(["td"], Cursor::new(""), false).unwrap();
        assert_eq!(cmd.input, "now");
    }

    #[test]
    fn stdin_with_content_is_read() {
        let cmd = Command::parse_from(["td"], Cursor::new("tomorrow\n"), false).unwrap();
        assert_eq!(cmd.input, "tomorrow");
    }

    #[test]
    fn json_flag_parsed() {
        let cmd = parse_ok(&["td", "now", "--json"]);
        assert!(cmd.json);
    }

    #[test]
    fn no_newline_flag_parsed() {
        let cmd = parse_ok(&["td", "now", "-n"]);
        assert!(cmd.no_newline);
    }

    #[test]
    fn skip_errors_flag_parsed() {
        let cmd = parse_ok(&["td", "now", "--skip-errors"]);
        assert!(cmd.skip_errors);
    }

    #[test]
    fn with_input_preserves_fields() {
        let cmd = parse_ok(&["td", "original", "-f", "%Y", "-t", "UTC", "--json", "-n"]);
        let new_cmd = cmd.with_input("replaced".to_string());
        assert_eq!(new_cmd.input, "replaced");
        assert_eq!(new_cmd.format.as_deref(), Some("%Y"));
        assert_eq!(new_cmd.timezone.as_deref(), Some("UTC"));
        assert!(new_cmd.json);
        assert!(new_cmd.no_newline);
    }
}
