use clap::{
    Parser, Subcommand, ValueEnum,
    builder::styling::{AnsiColor, Styles},
};
use color_print::cstr;

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

    /// Print verbose diagnostics to stderr (config, parse steps, timing).
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// In batch mode, skip lines that fail to parse instead of aborting.
    /// Errors are printed to stderr; stdout gets an empty line to preserve alignment.
    /// Exit code is 1 if any line failed.
    #[arg(long)]
    pub skip_errors: bool,

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
    /// Expand a date expression into a start/end range.
    Range(RangeArgs),
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

/// Arguments for the `range` subcommand.
#[derive(Debug, clap::Args)]
pub struct RangeArgs {
    /// Date expression to expand as range
    pub input: String,
    /// Output format (strftime pattern or preset name)
    #[arg(value_name = "FMT", short, long)]
    pub format: Option<String>,
    /// Time-zone to apply (IANA/Olson ID)
    #[arg(value_name = "TZ", short, long)]
    pub timezone: Option<String>,
    /// Override "now" reference (RFC 3339)
    #[arg(long)]
    pub now: Option<String>,
    /// Delimiter between start and end in plain-text output (default: newline).
    #[arg(short = 'd', long, default_value = "\n")]
    pub delimiter: String,
    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
    /// Suppress trailing newline
    #[arg(short = 'n', long = "no-newline")]
    pub no_newline: bool,
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
