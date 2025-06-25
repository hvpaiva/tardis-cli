use clap::{CommandFactory, Parser};
use std::{env, fs, path::PathBuf};

// HACK: Duplicated CLI documentation untl I find a better solution

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
