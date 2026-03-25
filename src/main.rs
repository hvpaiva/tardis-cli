//! TARDIS binary entry-point.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use std::io::{self, IsTerminal};

use tardis_cli::{
    Result,
    cli::{Cli, Command, ConfigAction, ConvertArgs, DiffArgs, InfoArgs, ShellType, SubCmd, TzArgs},
    config::Config,
    core::{self, App},
    locale::{self, LocaleKeywords},
    parser,
    user_input_error,
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
        let mut had_error = false;
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let single_cmd = cmd.with_input(line.to_owned());
            let result = if try_range_output(&single_cmd, &cfg).unwrap_or(false) {
                Ok(())
            } else {
                process_and_print(&single_cmd, &cfg)
            };
            if let Err(e) = result {
                if cmd.skip_errors {
                    eprintln!("{e}");
                    println!();
                    had_error = true;
                } else {
                    return Err(e);
                }
            }
        }
        if had_error {
            std::process::exit(1);
        }
    } else {
        // Try range expression first (per D-09, PARS-05)
        if !try_range_output(&cmd, &cfg)? {
            process_and_print(&cmd, &cfg)?;
        }
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
        SubCmd::Diff(args) => handle_diff(args),
        SubCmd::Convert(args) => handle_convert(args),
        SubCmd::Tz(args) => handle_tz(args),
        SubCmd::Info(args) => handle_info(args),
    }
}

/// Resolve the `--now` argument to an optional `jiff::Timestamp`.
fn resolve_now(now_arg: &Option<String>) -> Result<Option<jiff::Timestamp>> {
    now_arg
        .as_deref()
        .map(|s| s.parse::<jiff::Timestamp>())
        .transpose()
        .map_err(|e| user_input_error!(InvalidNow, "{} (expect RFC 3339)", e))
}

/// Resolve a timezone argument or fall back to the system timezone.
fn resolve_timezone(tz_arg: &Option<String>) -> Result<jiff::tz::TimeZone> {
    match tz_arg {
        Some(name) => jiff::tz::TimeZone::get(name)
            .map_err(|e| user_input_error!(UnsupportedTimezone, "{}", e)),
        None => Ok(jiff::tz::TimeZone::system()),
    }
}

/// Resolve a `Zoned` "now" reference from the `--now` arg and timezone.
fn resolve_now_zoned(
    now_arg: &Option<String>,
    tz: &jiff::tz::TimeZone,
) -> Result<jiff::Zoned> {
    match resolve_now(now_arg)? {
        Some(ts) => Ok(ts.to_zoned(tz.clone())),
        None => Ok(jiff::Zoned::now().with_time_zone(tz.clone())),
    }
}

/// Resolve a builtin format name to a strftime pattern.
fn resolve_builtin_format(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "iso8601" | "iso" => "%Y-%m-%dT%H:%M:%S%:z".to_string(),
        "rfc3339" => "%Y-%m-%dT%H:%M:%S%:z".to_string(),
        "rfc2822" => "%a, %d %b %Y %H:%M:%S %z".to_string(),
        "epoch" | "unix" => "epoch".to_string(),
        other => other.to_string(),
    }
}

/// Print a value respecting the `--no-newline` flag.
fn output_value(value: &str, no_newline: bool) {
    if no_newline {
        print!("{value}");
    } else {
        println!("{value}");
    }
}

/// Try to handle the input as a range expression (per D-09).
/// Returns `Ok(true)` if it was a range and was output, `Ok(false)` if not a range.
fn try_range_output(cmd: &Command, cfg: &Config) -> Result<bool> {
    let app = App::from_cli(cmd, cfg)?;
    let now = app
        .now
        .clone()
        .unwrap_or_else(|| jiff::Zoned::now().with_time_zone(app.timezone.clone()));

    let locale_ref = locale::get_locale(&app.locale_code);
    let locale_kw = LocaleKeywords::from_locale(locale_ref);

    match parser::parse_range(&cmd.input, &now, &locale_kw) {
        Ok((start, end)) => {
            let fmt = &app.format;
            let start_str = start.strftime(fmt).to_string();
            let end_str = end.strftime(fmt).to_string();

            if cmd.json {
                let json = serde_json::json!({
                    "input": cmd.input,
                    "start": start_str,
                    "end": end_str,
                    "start_epoch": start.timestamp().as_second(),
                    "end_epoch": end.timestamp().as_second(),
                });
                output_value(&format!("{json}"), cmd.no_newline);
            } else {
                // Two lines: start then end (per D-09)
                output_value(&format!("{start_str}\n{end_str}"), cmd.no_newline);
            }
            Ok(true)
        }
        Err(_) => Ok(false), // Not a range expression, fall through
    }
}

/// Handle `td diff <date1> <date2>` -- compute calendar-aware duration (D-01, SUBCMD-01).
fn handle_diff(args: DiffArgs) -> Result<()> {
    let tz = resolve_timezone(&args.timezone)?;
    let now = resolve_now_zoned(&args.now, &tz)?;

    // Subcommands use EN locale by default (no --locale flag on subcommands yet)
    let locale_ref = locale::get_locale("en");
    let locale_kw = LocaleKeywords::from_locale(locale_ref);

    let z1 = parser::parse(&args.date1, &now, &locale_kw)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;
    let z2 = parser::parse(&args.date2, &now, &locale_kw)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;

    // Calendar-aware span (human-readable and ISO 8601)
    let span = z1
        .until(jiff::ZonedDifference::new(&z2).largest(jiff::Unit::Year))
        .map_err(|e| user_input_error!(InvalidDateFormat, "diff failed: {}", e))?;

    // Total seconds (absolute duration)
    let total_secs = z2.timestamp().as_second() - z1.timestamp().as_second();

    if args.json {
        let json = serde_json::json!({
            "human": format!("{:#}", span),
            "seconds": total_secs,
            "iso8601": format!("{}", span),
        });
        output_value(&format!("{json}"), args.no_newline);
    } else {
        // Multi-format output per D-01
        let human = format!("{:#}", span);
        let iso = format!("{}", span);
        let output = format!("{human}\n{total_secs} seconds\n{iso}");
        output_value(&output, args.no_newline);
    }
    Ok(())
}

/// Handle `td convert <input> --to <format>` -- format conversion (D-02, SUBCMD-02).
fn handle_convert(args: ConvertArgs) -> Result<()> {
    let tz = resolve_timezone(&args.timezone)?;
    let now = resolve_now_zoned(&args.now, &tz)?;

    // Subcommands use EN locale by default
    let locale_ref = locale::get_locale("en");
    let locale_kw = LocaleKeywords::from_locale(locale_ref);

    // Parse input: if --from is specified, use strptime; otherwise auto-detect via parser
    let zoned = if let Some(ref from_fmt) = args.from {
        let pattern = resolve_builtin_format(from_fmt);
        jiff::Zoned::strptime(&pattern, &args.input)
            .map_err(|e| user_input_error!(InvalidDateFormat, "failed to parse with format '{}': {}", from_fmt, e))?
    } else {
        parser::parse(&args.input, &now, &locale_kw)
            .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?
    };

    // Format output using --to
    let to_fmt = resolve_builtin_format(&args.to);
    let output = if to_fmt == "epoch" || to_fmt == "unix" {
        zoned.timestamp().as_second().to_string()
    } else {
        zoned.strftime(&to_fmt).to_string()
    };

    if args.json {
        let json = serde_json::json!({
            "input": args.input,
            "output": output,
            "from_format": args.from.as_deref().unwrap_or("auto"),
            "to_format": args.to,
        });
        output_value(&format!("{json}"), args.no_newline);
    } else {
        output_value(&output, args.no_newline);
    }
    Ok(())
}

/// Handle `td tz <datetime> --to <timezone>` -- timezone conversion (D-03, SUBCMD-03).
fn handle_tz(args: TzArgs) -> Result<()> {
    // Source timezone: --from > system default
    let from_tz = resolve_timezone(&args.from)?;
    let now = resolve_now_zoned(&args.now, &from_tz)?;

    // Subcommands use EN locale by default
    let locale_ref = locale::get_locale("en");
    let locale_kw = LocaleKeywords::from_locale(locale_ref);

    // Parse input in source timezone
    let zoned = parser::parse(&args.input, &now, &locale_kw)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;

    // Convert to target timezone
    let target_tz = jiff::tz::TimeZone::get(&args.to)
        .map_err(|e| user_input_error!(UnsupportedTimezone, "{}", e))?;
    let converted = zoned
        .with_time_zone(target_tz);

    if args.json {
        let json = serde_json::json!({
            "input": args.input,
            "from_timezone": zoned.time_zone().iana_name().unwrap_or("Unknown"),
            "to_timezone": args.to,
            "original": zoned.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            "converted": converted.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
        });
        output_value(&format!("{json}"), args.no_newline);
    } else {
        output_value(
            &converted.strftime("%Y-%m-%dT%H:%M:%S %Z").to_string(),
            args.no_newline,
        );
    }
    Ok(())
}

/// Handle `td info <date>` -- calendar metadata card (D-04, SUBCMD-04).
fn handle_info(args: InfoArgs) -> Result<()> {
    let tz = resolve_timezone(&args.timezone)?;
    let now = resolve_now_zoned(&args.now, &tz)?;

    // Subcommands use EN locale by default
    let locale_ref = locale::get_locale("en");
    let locale_kw = LocaleKeywords::from_locale(locale_ref);

    let zoned = parser::parse(&args.input, &now, &locale_kw)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;

    let iwd = zoned.date().iso_week_date();
    let quarter = (zoned.month() - 1) / 3 + 1;
    let day_of_year = zoned.date().day_of_year();
    let days_in_year = if zoned.date().in_leap_year() { 366 } else { 365 };
    let epoch_secs = zoned.timestamp().as_second();
    let jdn = epoch_secs as f64 / 86400.0 + 2_440_587.5;

    if args.json {
        let json = serde_json::json!({
            "date": zoned.strftime("%Y-%m-%d").to_string(),
            "time": zoned.strftime("%H:%M:%S").to_string(),
            "timezone": zoned.time_zone().iana_name().unwrap_or("Unknown"),
            "weekday": format!("{:?}", zoned.weekday()),
            "iso_week": format!("W{:02}", iwd.week()),
            "iso_week_year": iwd.year(),
            "quarter": quarter,
            "day_of_year": day_of_year,
            "days_in_year": days_in_year,
            "leap_year": zoned.date().in_leap_year(),
            "unix_epoch": epoch_secs,
            "julian_day": format!("{:.2}", jdn),
        });
        output_value(&format!("{json}"), args.no_newline);
        return Ok(());
    }

    // Colored output per D-04 (neofetch/fastfetch style)
    let use_color = io::stdout().is_terminal() && std::env::var("NO_COLOR").is_err();

    let (bold, cyan, yellow, green, reset) = if use_color {
        ("\x1b[1m", "\x1b[36m", "\x1b[33m", "\x1b[32m", "\x1b[0m")
    } else {
        ("", "", "", "", "")
    };

    let date_str = zoned.strftime("%A, %B %e, %Y").to_string();
    let time_str = zoned.strftime("%H:%M:%S %Z").to_string();
    let leap_str = if zoned.date().in_leap_year() {
        format!("{yellow}Yes{reset}")
    } else {
        "No".to_string()
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "{bold}{cyan}  Date{reset}         {date_str}"
    ));
    lines.push(format!(
        "{bold}{cyan}  Time{reset}         {time_str}"
    ));
    lines.push(format!(
        "{bold}{cyan}  Week{reset}         W{:02}, {}",
        iwd.week(),
        iwd.year()
    ));
    lines.push(format!(
        "{bold}{cyan}  Quarter{reset}      {green}Q{quarter}{reset}"
    ));
    lines.push(format!(
        "{bold}{cyan}  Day of Year{reset}  {day_of_year}/{days_in_year}"
    ));
    lines.push(format!(
        "{bold}{cyan}  Leap Year{reset}    {leap_str}"
    ));
    lines.push(format!(
        "{bold}{cyan}  Unix Epoch{reset}   {epoch_secs}"
    ));
    lines.push(format!(
        "{bold}{cyan}  Julian Day{reset}   {jdn:.2}"
    ));

    let output = lines.join("\n");
    output_value(&output, args.no_newline);
    Ok(())
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
