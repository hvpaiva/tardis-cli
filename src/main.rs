//! TARDIS binary entry-point.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use std::io::{self, IsTerminal};

use tardis_cli::{
    Result,
    cli::{Cli, Command, ConfigAction, ShellType, SubCmd},
    config::Config,
    core::{self, App},
};

fn main() {
    if let Err(err) = run() {
        err.exit();
    }
}

fn run() -> Result<()> {
    // Parse raw CLI first to check for subcommands.
    let cli = <Cli as clap::Parser>::parse();

    // Hidden flag: generate man page and exit.
    if cli.generate_man {
        generate_man_page()?;
        return Ok(());
    }

    if let Some(subcmd) = cli.subcmd {
        return handle_subcmd(subcmd);
    }

    // Default parse flow (backwards compatible).
    let is_terminal = io::stdin().is_terminal();
    let cmd = Command::from_raw_cli(cli, io::stdin(), is_terminal)?;
    let cfg = Config::load()?;

    // Batch mode: if input has multiple lines, process each.
    let lines: Vec<&str> = cmd.input.lines().collect();
    if lines.len() > 1 {
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let single_cmd = Command {
                input: line.to_owned(),
                format: cmd.format.clone(),
                timezone: cmd.timezone.clone(),
                now: cmd.now,
                json: cmd.json,
                no_newline: cmd.no_newline,
                skip_errors: cmd.skip_errors,
            };
            process_and_print(&single_cmd, &cfg)?;
        }
    } else {
        process_and_print(&cmd, &cfg)?;
    }

    Ok(())
}

fn process_and_print(cmd: &Command, cfg: &Config) -> Result<()> {
    let app = App::from_cli(cmd, cfg)?;
    let result = core::process(&app, &cfg.presets())?;

    if cmd.json {
        let json = serde_json::json!({
            "input": cmd.input,
            "output": result.formatted,
            "epoch": result.epoch,
            "timezone": app.timezone.iana_name().unwrap_or("Unknown"),
            "format": app.format,
        });
        if cmd.no_newline {
            print!("{json}");
        } else {
            println!("{json}");
        }
    } else if cmd.no_newline {
        print!("{}", result.formatted);
    } else {
        println!("{}", result.formatted);
    }

    Ok(())
}

fn handle_subcmd(subcmd: SubCmd) -> Result<()> {
    match subcmd {
        SubCmd::Config { action } => handle_config(action),
        SubCmd::Completions { shell } => {
            handle_completions(shell);
            Ok(())
        }
        SubCmd::Diff(_args) => Err(tardis_cli::user_input_error!(InvalidDateFormat, "diff subcommand not yet implemented")),
        SubCmd::Convert(_args) => Err(tardis_cli::user_input_error!(InvalidDateFormat, "convert subcommand not yet implemented")),
        SubCmd::Tz(_args) => Err(tardis_cli::user_input_error!(InvalidDateFormat, "tz subcommand not yet implemented")),
        SubCmd::Info(_args) => Err(tardis_cli::user_input_error!(InvalidDateFormat, "info subcommand not yet implemented")),
    }
}

fn handle_config(action: ConfigAction) -> Result<()> {
    use tardis_cli::config;

    match action {
        ConfigAction::Path => {
            println!("{}", config::config_path()?.display());
        }
        ConfigAction::Show => {
            let cfg = Config::load()?;
            println!("format   = \"{}\"", cfg.format);
            println!("timezone = \"{}\"", cfg.timezone);
            if let Some(fmts) = &cfg.formats {
                println!("\n[formats]");
                for (name, fmt) in fmts {
                    println!("{name:<10} = \"{fmt}\"");
                }
            }
        }
        ConfigAction::Edit => {
            let path = config::config_path()?;
            // Ensure the config file exists before opening.
            Config::load()?;
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            std::process::Command::new(&editor)
                .arg(&path)
                .status()
                .map_err(|e| {
                    tardis_cli::system_error!(Config, "failed to open editor '{}': {}", editor, e)
                })?;
        }
        ConfigAction::Presets => {
            let cfg = Config::load()?;
            let presets = cfg.presets();
            if presets.is_empty() {
                println!("No presets defined. Add them to [formats] in your config file.");
                println!("Config: {}", config::config_path()?.display());
            } else {
                println!("{:<12} FORMAT", "NAME");
                println!("{:<12} ------", "----");
                for p in &presets {
                    println!("{:<12} {}", p.name, p.format);
                }
            }
        }
    }
    Ok(())
}

fn generate_man_page() -> Result<()> {
    use clap::CommandFactory;
    use std::io::Write;
    use tardis_cli::errors::SystemError;

    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd)
        .title("TD")
        .section("1")
        .manual("TARDIS Manual");

    let mut buf: Vec<u8> = Vec::new();
    man.render(&mut buf).map_err(SystemError::from)?;

    // Append custom sections for reference-quality man page (D-04)

    // EXAMPLES section
    buf.extend_from_slice(b".SH EXAMPLES\n");
    buf.extend_from_slice(b".TP\n\\fBBasic usage:\\fR\n");
    buf.extend_from_slice(
        b"td \"tomorrow\"\n.br\ntd \"next friday at 3pm\"\n.br\ntd \"in 2 hours\"\n",
    );
    buf.extend_from_slice(b".TP\n\\fBWith format:\\fR\n");
    buf.extend_from_slice(
        b"td \"today\" \\-f \"%Y\\-%m\\-%d\"\n.br\ntd \"now\" \\-f epoch\n",
    );
    buf.extend_from_slice(b".TP\n\\fBWith timezone:\\fR\n");
    buf.extend_from_slice(
        b"td \"now\" \\-t \"America/Sao_Paulo\"\n.br\ntd \"now\" \\-t UTC \\-f \"%Y\\-%m\\-%dT%H:%M:%S%:z\"\n",
    );
    buf.extend_from_slice(b".TP\n\\fBEpoch input:\\fR\n");
    buf.extend_from_slice(
        b"td @1735689600\n.br\ntd @1735689600 \\-f \"%Y\\-%m\\-%d\" \\-t UTC\n",
    );
    buf.extend_from_slice(b".TP\n\\fBJSON output:\\fR\n");
    buf.extend_from_slice(b"td \"tomorrow\" \\-\\-json\n");
    buf.extend_from_slice(b".TP\n\\fBBatch mode (pipe multiple lines):\\fR\n");
    buf.extend_from_slice(
        b"echo \\-e \"today\\\\ntomorrow\" | td \\-f \"%Y\\-%m\\-%d\" \\-t UTC\n",
    );
    buf.extend_from_slice(b".TP\n\\fBDeterministic output (for scripts):\\fR\n");
    buf.extend_from_slice(
        b"td \"next monday\" \\-\\-now 2025\\-01\\-01T00:00:00Z \\-f \"%Y\\-%m\\-%d\"\n",
    );

    // ENVIRONMENT section
    buf.extend_from_slice(b".SH ENVIRONMENT\n");
    buf.extend_from_slice(b".TP\n\\fBTARDIS_FORMAT\\fR\n");
    buf.extend_from_slice(
        b"Default output format or preset name. Overridden by \\fB\\-\\-format\\fR.\n",
    );
    buf.extend_from_slice(b".TP\n\\fBTARDIS_TIMEZONE\\fR\n");
    buf.extend_from_slice(
        b"Default IANA time zone (e.g. America/Sao_Paulo). Overridden by \\fB\\-\\-timezone\\fR.\n",
    );
    buf.extend_from_slice(b".TP\n\\fBXDG_CONFIG_HOME\\fR\n");
    buf.extend_from_slice(
        b"Override the base configuration directory. Default: ~/.config on Linux.\n",
    );
    buf.extend_from_slice(b".TP\n\\fBEDITOR\\fR\n");
    buf.extend_from_slice(b"Editor used by \\fBtd config edit\\fR. Default: vi.\n");

    // FILES section
    buf.extend_from_slice(b".SH FILES\n");
    buf.extend_from_slice(b".TP\n\\fB$XDG_CONFIG_HOME/tardis/config.toml\\fR\n");
    buf.extend_from_slice(
        b"User configuration file. Created automatically on first run with commented defaults.\n",
    );
    buf.extend_from_slice(
        b"Fields: \\fBformat\\fR (default output format), \\fBtimezone\\fR (default IANA timezone), \\fB[formats]\\fR (named preset table).\n",
    );

    // EXIT STATUS section
    buf.extend_from_slice(b".SH \"EXIT STATUS\"\n");
    buf.extend_from_slice(b".TP\n\\fB0\\fR\nSuccess.\n");
    buf.extend_from_slice(
        b".TP\n\\fB64\\fR\nUsage error (bad input, unsupported format, invalid timezone).\n",
    );
    buf.extend_from_slice(b".TP\n\\fB74\\fR\nI/O error.\n");
    buf.extend_from_slice(b".TP\n\\fB78\\fR\nConfiguration error.\n");

    std::io::stdout()
        .write_all(&buf)
        .map_err(SystemError::from)?;
    Ok(())
}

fn handle_completions(shell: ShellType) {
    use clap::CommandFactory;
    use clap_complete::{Shell, generate};

    let shell = match shell {
        ShellType::Bash => Shell::Bash,
        ShellType::Zsh => Shell::Zsh,
        ShellType::Fish => Shell::Fish,
        ShellType::Elvish => Shell::Elvish,
        ShellType::Powershell => Shell::PowerShell,
    };

    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "td", &mut io::stdout());
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    use jiff::{Zoned, tz::TimeZone};
    use tardis_cli::core::{self, App, Preset};

    fn utc() -> TimeZone {
        TimeZone::get("UTC").unwrap()
    }

    macro_rules! now {
        () => {{
            Some(
                "2025-01-01T12:00:00Z"
                    .parse::<jiff::Timestamp>()
                    .unwrap()
                    .to_zoned(utc()),
            )
        }};
        ($dt:expr) => {{
            Some(
                $dt.parse::<jiff::Timestamp>()
                    .unwrap()
                    .to_zoned(utc()),
            )
        }};
    }

    fn app(date: &str, fmt: &str, tz: TimeZone, now: Option<Zoned>) -> App {
        App::new(date.to_string(), fmt.to_string(), tz, now)
    }

    fn run(app: &App, presets: &[Preset]) -> String {
        core::process(app, presets).unwrap().formatted
    }

    #[test]
    fn happy_path_basic() {
        let a = app("2025-01-01 12:00", "%Y", utc(), now!());
        let out = run(&a, &[]);
        assert_eq!(out, "2025");
    }

    #[test]
    fn resolves_preset() {
        let a = app("2030-12-31 00:00", "br", utc(), now!("2030-12-31T00:00:00Z"));

        let mut map = HashMap::new();
        map.insert("br".to_string(), "%d/%m/%Y".to_string());
        let presets: Vec<_> = map
            .iter()
            .map(|(n, f)| Preset::new(n.clone(), f.clone()))
            .collect();

        let out = run(&a, &presets);
        assert_eq!(out, "31/12/2030");
    }

    #[test]
    fn invalid_date_expression() {
        let a = app("$$$", "%Y", utc(), now!());
        let res = core::process(&a, &[]);
        assert!(res.is_err());
    }
}
