use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use std::{env, fs, path::PathBuf};

/// Minimal CLI mirror for man page and shell completion generation.
/// The real `Cli` lives in `src/cli.rs` — keep args in sync.
#[derive(Debug, Parser)]
#[command(
    name = "td",
    about = "TARDIS - Time And Relative Date Input Simplifier",
    version
)]
struct Cli {
    /// Natural-language date expression (e.g. "next Friday at 9:30").
    input: Option<String>,

    /// Output format (chrono strftime pattern or preset name).
    #[arg(value_name = "FMT", short, long)]
    format: Option<String>,

    /// Time-zone (IANA/Olson ID).
    #[arg(value_name = "TZ", short, long)]
    timezone: Option<String>,

    /// Override "now" (RFC 3339).
    #[arg(value_name = "DATETIME", long)]
    now: Option<String>,

    /// Output as JSON.
    #[arg(short, long)]
    json: bool,

    /// Suppress trailing newline.
    #[arg(short = 'n', long = "no-newline")]
    no_newline: bool,

    #[command(subcommand)]
    subcmd: Option<SubCmd>,
}

#[derive(Debug, Subcommand)]
enum SubCmd {
    /// Manage configuration file.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Generate shell completions.
    Completions {
        /// Shell type.
        shell: ShellType,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigAction {
    /// Print config file path.
    Path,
    /// Display effective configuration.
    Show,
    /// Open config in $EDITOR.
    Edit,
    /// List format presets.
    Presets,
}

#[derive(Debug, Clone, ValueEnum)]
enum ShellType {
    Bash,
    Zsh,
    Fish,
    Elvish,
    Powershell,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir: PathBuf = env::var("OUT_DIR")?.into();
    let cmd = Cli::command();

    // Generate man page.
    let mut man_buf: Vec<u8> = Vec::new();
    let man = clap_mangen::Man::new(cmd.clone());
    man.render(&mut man_buf)?;
    fs::write(out_dir.join("td.1"), man_buf)?;

    // Generate shell completions.
    use clap_complete::{Shell, generate_to};
    for shell in [
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::Elvish,
        Shell::PowerShell,
    ] {
        generate_to(shell, &mut cmd.clone(), "td", &out_dir)?;
    }

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
