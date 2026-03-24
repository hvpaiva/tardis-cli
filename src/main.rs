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
            "timezone": app.timezone.name(),
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

    use chrono::DateTime;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    use chrono_tz::Tz;
    use tardis_cli::core::{self, App, Preset};

    macro_rules! now {
        () => {{
            Some(
                chrono::DateTime::parse_from_rfc3339("2025-01-01T12:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono_tz::UTC),
            )
        }};
        ($dt:expr) => {{
            Some(
                chrono::DateTime::parse_from_rfc3339($dt)
                    .unwrap()
                    .with_timezone(&chrono_tz::UTC),
            )
        }};
    }

    const UTC: Tz = chrono_tz::UTC;

    fn app(date: &str, fmt: &str, tz: Tz, now: Option<DateTime<Tz>>) -> App {
        App::new(date.to_string(), fmt.to_string(), tz, now)
    }

    fn run(app: &App, presets: &[Preset]) -> String {
        core::process(app, presets).unwrap().formatted
    }

    #[test]
    fn happy_path_basic() {
        let a = app("2025-01-01 12:00", "%Y", UTC, now!());
        let out = run(&a, &[]);
        assert_eq!(out, "2025");
    }

    #[test]
    fn resolves_preset() {
        let a = app("2030-12-31 00:00", "br", UTC, now!("2030-12-31T00:00:00Z"));

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
        let a = app("$$$", "%Y", UTC, now!());
        let res = core::process(&a, &[]);
        assert!(res.is_err());
    }
}
