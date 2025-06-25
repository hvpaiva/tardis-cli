use clap::{
    CommandFactory, Parser,
    builder::{Styles, styling::AnsiColor},
};
use color_print::cstr;
use std::{env, fs, path::PathBuf};

// HACK: Duplicated CLI documentation until I find a better solution

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

For more info, visit <underline>https://github.com/hvpaiva/tardis</underline>
"#
);

pub const INPUT_HELP: &str = cstr!(
    r#"
<bold>A natural-language expression</bold> like <underline>"next Friday at 9:30"</underline>.
If omitted, the value is read from <bold>STDIN</bold>.

Supported formats:
<underline>https://github.com/technologicalMayhem/human-date-parser?tab=readme-ov-file#formats</underline>
"#
);

const FORMAT_HELP: &str = cstr!(
    r#"
<bold>Output format.</bold>

Accepts chrono‑style strftime patterns (e.g. <bold>"%Y‑%m‑%d"</bold>) or a named
preset defined in the config file.

Reference:
<underline>https://docs.rs/chrono/latest/chrono/format/strftime/index.html</underline>

If not provided, tries to read from <bold><blue>TARDIS_FORMAT</blue></bold> and
falls back to the default format defined in the config file.
"#
);

pub const TIMEZONE_HELP: &str = cstr!(
    r#"
<bold>Time‑zone to apply</bold> (IANA/Olson ID). If not provided, uses system local time.

Examples: <italic>"UTC", "America/Sao_Paulo", "Europe/London".</italic>

Reference:
<underline>https://docs.rs/chrono-tz/latest/chrono_tz/enum.Tz.html</underline>

If not provided, tries to read from <bold><blue>TARDIS_TIMEZONE</blue></bold> and
falls back to the default time zone defined in the config file.
"#
);

pub const NOW_HELP: &str = cstr!(
    r#"
Override “now”. Format <bold>RFC 3339</bold>, e.g. <italic>2025‑06‑24T09:00:00Z</italic>.
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
    after_help = cstr!("For more information, visit <underline>https://github.com/hvpaiva/tardis</underline>"),
    styles=STYLES,
)]
pub struct Cli {
    #[arg(help = INPUT_HELP)]
    input: Option<String>,
    /// Output format.
    #[arg(value_name = "FMT", short, long, long_help = FORMAT_HELP)]
    format: Option<String>,
    /// Time-zone to apply (IANA/Olson ID). If not provided, uses system local time.
    #[arg(value_name = "TZ", short, long, long_help = TIMEZONE_HELP)]
    timezone: Option<String>,
    /// Override “now”. Format **RFC 3339**, e.g. 2025-06-24T09:00:00Z.
    #[arg(value_name = "DATETIME", long, long_help = NOW_HELP)]
    now: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir: PathBuf = env::var("OUT_DIR")?.into();
    let cmd = Cli::command();

    let mut man_buf: Vec<u8> = Vec::new();
    let man = clap_mangen::Man::new(cmd.clone());
    man.render(&mut man_buf)?;
    fs::write(out_dir.join("td.1"), man_buf)?;

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
