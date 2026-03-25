use std::{
    env,
    ffi::OsString,
    io::{self, IsTerminal, Read},
};

use clap::{
    Parser, Subcommand, ValueEnum,
    builder::styling::{AnsiColor, Styles},
};
use color_print::cstr;
use jiff::Timestamp;

use crate::{Result, user_input_error};

pub const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

pub const AFTER_LONG_HELP: &str = cstr!(
    r#"
<green><bold>Environment Variables:</bold></green>
  <bold><blue>TARDIS_FORMAT</blue></bold>     Default output format or preset name.
  <bold><blue>TARDIS_TIMEZONE</blue></bold>   Default IANA time zone (e.g. America/Sao_Paulo).

<green><bold>Configuration File:</bold></green>
  <blue><bold>$XDG_CONFIG_HOME</bold>/tardis/config.toml</blue>

  if XDG_CONFIG_HOME is unset:
        • Linux:   ~/.config/tardis/config.toml
        • macOS:   ~/Library/Application Support/tardis/config.toml
        • Windows: %APPDATA%\tardis\config.toml

  The file is created automatically on first run and contains commented
  examples for every field.


<green><bold>Precedence:</bold></green>
  CLI flags → env vars → config file

For more info, visit <underline>https://github.com/hvpaiva/tardis-cli</underline>
"#
);

pub const INPUT_HELP: &str = cstr!(
    r#"
<bold>A natural-language expression</bold> like <underline>"next Friday at 9:30"</underline>.
If omitted and STDIN is a pipe, reads from it. If omitted in a terminal, defaults to <bold>"now"</bold>.

Supports <bold>@&lt;epoch&gt;</bold> syntax for Unix timestamps (e.g. <bold>@1719244800</bold>).
Smart precision: seconds, milliseconds, microseconds, and nanoseconds auto-detected.

Supports arithmetic (<bold>"tomorrow + 3 hours"</bold>) and range expressions (<bold>"last week"</bold>).
"#
);

const FORMAT_HELP: &str = cstr!(
    r#"
<bold>Output format.</bold>

Accepts strftime patterns (e.g. <bold>"%Y‑%m‑%d"</bold>) or a named
preset defined in the config file.

Special values: <bold>"epoch"</bold> or <bold>"unix"</bold> output a Unix timestamp (seconds).

Reference:
<underline>https://docs.rs/jiff/latest/jiff/fmt/strtime/index.html</underline>

If not provided, tries to read from <bold><blue>TARDIS_FORMAT</blue></bold> and
falls back to the default format defined in the config file.
"#
);

pub const TIMEZONE_HELP: &str = cstr!(
    r#"
<bold>Time‑zone to apply</bold> (IANA/Olson ID). If not provided, uses system local time.

Examples: <italic>"UTC", "America/Sao_Paulo", "Europe/London".</italic>

Reference:
<underline>https://www.iana.org/time-zones</underline>

If not provided, tries to read from <bold><blue>TARDIS_TIMEZONE</blue></bold> and
falls back to the default time zone defined in the config file.
"#
);

pub const NOW_HELP: &str = cstr!(
    r#"
Override "now". Format <bold>RFC 3339</bold>, e.g. <italic>2025‑06‑24T09:00:00Z</italic>.
"#
);

pub const ABOUT_HELP: &str = cstr!(
    r#"
<magenta>TARDIS — Time And Relative Date Input Simplifier</magenta>

Translates natural-language time expressions into formatted datetimes.

A lightweight CLI tool for converting human-readable date and time phrases
like <bold>"next Friday at 2:00"</bold> or <bold>"in 3 days"</bold> into machine-usable output.
"#
);

/// TARDIS — Time And Relative Date Input Simplifier
#[derive(Debug, Parser)]
#[command(
    name = "td",
    about,
    long_about = ABOUT_HELP,
    version,
    color = clap::ColorChoice::Auto,
    after_long_help = AFTER_LONG_HELP,
    after_help = cstr!("For more information, visit <underline>https://github.com/hvpaiva/tardis-cli</underline>"),
    styles = STYLES,
)]
pub struct Cli {
    #[arg(help = INPUT_HELP)]
    pub input: Option<String>,

    /// Output format.
    #[arg(value_name = "FMT", short, long, long_help = FORMAT_HELP)]
    pub format: Option<String>,

    /// Time-zone to apply (IANA/Olson ID). If not provided, uses system local time.
    #[arg(value_name = "TZ", short, long, long_help = TIMEZONE_HELP)]
    pub timezone: Option<String>,

    /// Override "now". Format **RFC 3339**, e.g. 2025-06-24T09:00:00Z.
    #[arg(value_name = "DATETIME", long, long_help = NOW_HELP)]
    pub now: Option<String>,

    /// Output as JSON instead of plain text.
    #[arg(short, long)]
    pub json: bool,

    /// Suppress trailing newline.
    #[arg(short = 'n', long = "no-newline")]
    pub no_newline: bool,

    /// Locale for input parsing (e.g. en, pt). Auto-detected if omitted.
    #[arg(
        short = 'L',
        long,
        help = "Locale for input parsing (e.g. en, pt). Auto-detected if omitted."
    )]
    pub locale: Option<String>,

    /// Print verbose diagnostics to stderr (config, parse steps, timing).
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// In batch mode, skip lines that fail to parse instead of aborting.
    /// Errors are printed to stderr; stdout gets an empty line to preserve alignment.
    /// Exit code is 1 if any line failed.
    #[arg(long)]
    pub skip_errors: bool,

    /// Generate man page to stdout (hidden, for packaging scripts).
    #[arg(long, hide = true, exclusive = true)]
    pub generate_man: bool,

    #[command(subcommand)]
    pub subcmd: Option<SubCmd>,
}

#[non_exhaustive]
#[derive(Debug, Subcommand)]
pub enum SubCmd {
    /// Manage configuration file.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Generate shell completions.
    Completions {
        /// Shell to generate completions for.
        shell: ShellType,
    },
    /// Compute the difference between two dates.
    Diff(DiffArgs),
    /// Convert a date between formats.
    Convert(ConvertArgs),
    /// Convert a datetime to a different timezone.
    Tz(TzArgs),
    /// Display calendar metadata for a date.
    Info(InfoArgs),
}

#[derive(Debug, clap::Args)]
pub struct DiffArgs {
    /// First date expression
    pub date1: String,
    /// Second date expression
    pub date2: String,
    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
    /// Suppress trailing newline
    #[arg(short = 'n', long = "no-newline")]
    pub no_newline: bool,
    /// Override "now" reference (RFC 3339)
    #[arg(long)]
    pub now: Option<String>,
    /// Time-zone for resolution
    #[arg(short, long)]
    pub timezone: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct ConvertArgs {
    /// Input date expression or formatted date string
    pub input: String,
    /// Input format (strptime pattern or preset name). Auto-detected if omitted.
    #[arg(long)]
    pub from: Option<String>,
    /// Output format (strftime pattern, preset name, or builtin: iso8601, rfc3339, rfc2822, epoch, unix)
    #[arg(long)]
    pub to: String,
    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
    /// Suppress trailing newline
    #[arg(short = 'n', long = "no-newline")]
    pub no_newline: bool,
    /// Override "now" reference (RFC 3339)
    #[arg(long)]
    pub now: Option<String>,
    /// Time-zone for resolution
    #[arg(short, long)]
    pub timezone: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct TzArgs {
    /// Input datetime expression
    pub input: String,
    /// Source timezone (auto-detected from system or input if omitted)
    #[arg(long)]
    pub from: Option<String>,
    /// Target timezone (required, IANA name like "America/Sao_Paulo")
    #[arg(long)]
    pub to: String,
    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
    /// Suppress trailing newline
    #[arg(short = 'n', long = "no-newline")]
    pub no_newline: bool,
    /// Override "now" reference (RFC 3339)
    #[arg(long)]
    pub now: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct InfoArgs {
    /// Date expression to inspect (defaults to "now")
    #[arg(default_value = "now")]
    pub input: String,
    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
    /// Suppress trailing newline
    #[arg(short = 'n', long = "no-newline")]
    pub no_newline: bool,
    /// Override "now" reference (RFC 3339)
    #[arg(long)]
    pub now: Option<String>,
    /// Time-zone for resolution
    #[arg(short, long)]
    pub timezone: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print the path to the configuration file.
    Path,
    /// Display the effective configuration.
    Show,
    /// Open the configuration file in $EDITOR.
    Edit,
    /// List all available format presets.
    Presets,
}

#[non_exhaustive]
#[derive(Debug, Clone, ValueEnum)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Elvish,
    Powershell,
}

/// Normalised user command ready for further processing.
#[non_exhaustive]
#[derive(Debug)]
pub struct Command {
    pub input: String,
    pub format: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
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
            locale: self.locale.clone(),
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
            locale: cli.locale,
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
