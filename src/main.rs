//! TARDIS binary entry-point.
#![deny(clippy::unwrap_used, clippy::expect_used)]

use std::io::{self, IsTerminal};

use tardis_cli::{
    Result,
    cli::{
        Cli, Command, ConfigAction, ConvertArgs, DiffArgs, DiffOutput, InfoArgs, RangeArgs,
        ShellType, SubCmd, TzArgs,
    },
    config::Config,
    core::{self, App},
    parser, user_input_error,
};

/// Check if stderr supports color output.
fn stderr_use_color() -> bool {
    io::stderr().is_terminal() && std::env::var("NO_COLOR").is_err()
}

/// Print a colored verbose diagnostic line to stderr.
macro_rules! verbose {
    ($tag:expr, $($arg:tt)*) => {{
        if stderr_use_color() {
            let color = match $tag {
                "config" => "\x1b[36m",
                "parse" => "\x1b[34m",
                "resolve" => "\x1b[32m",
                "timing" => "\x1b[33m",
                _ => "\x1b[0m",
            };
            eprintln!("{}[{}]\x1b[0m {}", color, $tag, format_args!($($arg)*));
        } else {
            eprintln!("[{}] {}", $tag, format_args!($($arg)*));
        }
    }};
}

fn main() {
    if let Err(err) = run() {
        err.exit();
    }
}

fn run() -> Result<()> {
    let cli = <Cli as clap::Parser>::parse();

    if let Some(subcmd) = cli.subcmd {
        return handle_subcmd(subcmd);
    }

    let is_terminal = io::stdin().is_terminal();
    let cmd = Command::from_raw_cli(cli, io::stdin(), is_terminal)?;
    let cfg = Config::load()?;

    if cmd.verbose {
        verbose!(
            "config",
            "path={}",
            tardis_cli::config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".into())
        );
        verbose!("config", "format={} timezone={}", cfg.format, cfg.timezone);
    }

    let lines: Vec<&str> = cmd.input.lines().collect();
    if lines.len() > 1 {
        let mut had_error = false;
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let single_cmd = cmd.with_input(line.to_owned());
            let result = process_and_print(&single_cmd, &cfg);
            if let Err(e) = result {
                if cmd.skip_errors {
                    eprintln!("{e}");
                    if !io::stdout().is_terminal() {
                        println!();
                    }
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
        process_and_print(&cmd, &cfg)?;
    }

    Ok(())
}

fn process_and_print(cmd: &Command, cfg: &Config) -> Result<()> {
    let start = std::time::Instant::now();
    let app = App::from_cli(cmd, cfg)?;

    if cmd.verbose {
        verbose!("parse", "input={:?}", cmd.input);
        verbose!(
            "parse",
            "effective_format={} timezone={}",
            app.format,
            app.timezone.iana_name().unwrap_or("system")
        );
    }

    let result = core::process(&app, &cfg.presets())?;

    if cmd.verbose {
        let elapsed = start.elapsed();
        verbose!(
            "resolve",
            "output={:?} epoch={}",
            result.formatted,
            result.epoch
        );
        verbose!("timing", "{:.3}ms", elapsed.as_secs_f64() * 1000.0);
    }

    if cmd.json {
        let json = serde_json::json!({
            "input": cmd.input,
            "output": result.formatted,
            "epoch": result.epoch,
            "timezone": app.timezone.iana_name().unwrap_or("Unknown"),
            "format": app.format,
        });
        emit_json(&json, cmd.no_newline);
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
        SubCmd::Range(args) => handle_range(args),
        _ => unreachable!(),
    }
}

/// Resolve the `--now` argument to an optional `jiff::Timestamp`.
fn resolve_now(now_arg: &Option<String>) -> Result<Option<jiff::Timestamp>> {
    let effective = now_arg
        .clone()
        .or_else(|| std::env::var("TARDIS_NOW").ok().filter(|s| !s.is_empty()));
    effective
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
fn resolve_now_zoned(now_arg: &Option<String>, tz: &jiff::tz::TimeZone) -> Result<jiff::Zoned> {
    match resolve_now(now_arg)? {
        Some(ts) => Ok(ts.to_zoned(tz.clone())),
        None => Ok(jiff::Zoned::now().with_time_zone(tz.clone())),
    }
}

/// Resolve a builtin format name to a strftime pattern.
///
/// Case-insensitive lookup for well-known names (iso8601, rfc3339, etc.);
/// custom strftime patterns are returned verbatim (preserving case of `%Y` etc.).
fn resolve_builtin_format(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "iso8601" | "iso" => "%Y-%m-%dT%H:%M:%S%:z".to_string(),
        "rfc3339" => "%Y-%m-%dT%H:%M:%S%:z".to_string(),
        "rfc2822" => "%a, %d %b %Y %H:%M:%S %z".to_string(),
        "epoch" | "unix" => "epoch".to_string(),
        _ => name.to_string(),
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

/// Emit a JSON value to stdout with TTY-aware formatting.
///
/// Pretty-prints with syntax colors when stdout is a TTY and `NO_COLOR` is unset;
/// emits compact single-line JSON otherwise.
fn emit_json(value: &serde_json::Value, no_newline: bool) {
    let text = if io::stdout().is_terminal() && std::env::var("NO_COLOR").is_err() {
        colored_json::to_colored_json_auto(value)
            .unwrap_or_else(|_| serde_json::to_string_pretty(value).unwrap_or_default())
    } else {
        value.to_string()
    };

    if no_newline {
        print!("{text}");
    } else {
        println!("{text}");
    }
}

/// Handle `td range <expression>` -- expand expression to start/end pair.
fn handle_range(args: RangeArgs) -> Result<()> {
    let start_instant = std::time::Instant::now();
    let tz = resolve_timezone(&args.timezone)?;
    let now = resolve_now_zoned(&args.now, &tz)?;
    let cfg = Config::load()?;
    let fmt = args
        .format
        .as_deref()
        .map(resolve_builtin_format)
        .unwrap_or_else(|| cfg.format.clone());

    if args.verbose {
        verbose!("parse", "input={:?}", args.input);
        verbose!("parse", "timezone={}", tz.iana_name().unwrap_or("system"));
    }

    let (start, end) = parser::parse_range_with_granularity(&args.input, &now)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;

    let start_str = start.strftime(&fmt).to_string();
    let end_str = end.strftime(&fmt).to_string();

    if args.verbose {
        verbose!("resolve", "start={} end={}", start_str, end_str);
        let elapsed = start_instant.elapsed();
        verbose!("timing", "{:.3}ms", elapsed.as_secs_f64() * 1000.0);
    }

    if args.json {
        let json = serde_json::json!({
            "input": args.input,
            "start": start_str,
            "end": end_str,
            "start_epoch": start.timestamp().as_second(),
            "end_epoch": end.timestamp().as_second(),
            "timezone": tz.iana_name().unwrap_or("Unknown"),
            "format": fmt,
            "delimiter": args.delimiter,
        });
        emit_json(&json, args.no_newline);
    } else {
        output_value(
            &format!("{start_str}{}{end_str}", args.delimiter),
            args.no_newline,
        );
    }
    Ok(())
}

/// Handle `td diff <date1> <date2>` -- compute calendar-aware duration.
fn handle_diff(args: DiffArgs) -> Result<()> {
    let start_instant = std::time::Instant::now();
    let tz = resolve_timezone(&args.timezone)?;
    let now = resolve_now_zoned(&args.now, &tz)?;

    if args.verbose {
        verbose!("parse", "date1={:?} date2={:?}", args.date1, args.date2);
        verbose!("parse", "timezone={}", tz.iana_name().unwrap_or("system"));
    }

    let z1 = parser::parse(&args.date1, &now)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;
    let z2 = parser::parse(&args.date2, &now)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;

    let span = z1
        .until(jiff::ZonedDifference::new(&z2).largest(jiff::Unit::Year))
        .map_err(|e| user_input_error!(InvalidDateFormat, "diff failed: {}", e))?;

    let total_secs = z2.timestamp().as_second() - z1.timestamp().as_second();

    if args.verbose {
        verbose!(
            "resolve",
            "human={:?} seconds={} iso={}",
            format!("{:#}", span),
            total_secs,
            format!("{}", span)
        );
        let elapsed = start_instant.elapsed();
        verbose!("timing", "{:.3}ms", elapsed.as_secs_f64() * 1000.0);
    }

    if args.json {
        let json = serde_json::json!({
            "human": format!("{:#}", span),
            "seconds": total_secs,
            "iso8601": format!("{}", span),
        });
        emit_json(&json, args.no_newline);
    } else {
        let text = match args.output {
            DiffOutput::Human => format!("{:#}", span),
            DiffOutput::Seconds => total_secs.to_string(),
            DiffOutput::Iso => format!("{}", span),
        };
        output_value(&text, args.no_newline);
    }
    Ok(())
}

/// Handle `td convert <input> --to <format>` -- format conversion.
fn handle_convert(args: ConvertArgs) -> Result<()> {
    let start_instant = std::time::Instant::now();
    let tz = resolve_timezone(&args.timezone)?;
    let now = resolve_now_zoned(&args.now, &tz)?;

    if args.verbose {
        verbose!(
            "parse",
            "input={:?} from={:?} to={:?}",
            args.input,
            args.from,
            args.to
        );
        verbose!("parse", "timezone={}", tz.iana_name().unwrap_or("system"));
    }

    let zoned = if let Some(ref from_fmt) = args.from {
        let pattern = resolve_builtin_format(from_fmt);
        jiff::Zoned::strptime(&pattern, &args.input).map_err(|e| {
            user_input_error!(
                InvalidDateFormat,
                "failed to parse with format '{}': {}",
                from_fmt,
                e
            )
        })?
    } else {
        if let Ok(ts) = args.input.parse::<jiff::Timestamp>() {
            ts.to_zoned(tz.clone())
        } else if let Ok(epoch) = args.input.trim().parse::<i64>() {
            let abs = epoch.unsigned_abs();
            let ts = if abs < 1_000_000_000_000 {
                jiff::Timestamp::from_second(epoch)
            } else if abs < 1_000_000_000_000_000 {
                jiff::Timestamp::from_millisecond(epoch)
            } else if abs < 1_000_000_000_000_000_000 {
                jiff::Timestamp::from_microsecond(epoch)
            } else {
                jiff::Timestamp::from_nanosecond(i128::from(epoch))
            }
            .map_err(|e| user_input_error!(InvalidDateFormat, "invalid epoch: {}", e))?;
            ts.to_zoned(tz.clone())
        } else {
            parser::parse(&args.input, &now)
                .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?
        }
    };

    let to_fmt = resolve_builtin_format(&args.to);
    let output = if to_fmt == "epoch" || to_fmt == "unix" {
        zoned.timestamp().as_second().to_string()
    } else {
        zoned.strftime(&to_fmt).to_string()
    };

    if args.verbose {
        verbose!("resolve", "output={:?}", output);
        let elapsed = start_instant.elapsed();
        verbose!("timing", "{:.3}ms", elapsed.as_secs_f64() * 1000.0);
    }

    if args.json {
        let json = serde_json::json!({
            "input": args.input,
            "output": output,
            "from_format": args.from.as_deref().unwrap_or("auto"),
            "to_format": args.to,
        });
        emit_json(&json, args.no_newline);
    } else {
        output_value(&output, args.no_newline);
    }
    Ok(())
}

/// Handle `td tz <datetime> --to <timezone>` -- timezone conversion.
fn handle_tz(args: TzArgs) -> Result<()> {
    let start_instant = std::time::Instant::now();
    let from_tz = resolve_timezone(&args.from)?;
    let now = resolve_now_zoned(&args.now, &from_tz)?;

    if args.verbose {
        verbose!(
            "parse",
            "input={:?} from={:?} to={:?}",
            args.input,
            args.from,
            args.to
        );
    }

    let zoned = parser::parse(&args.input, &now)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;

    let target_tz = jiff::tz::TimeZone::get(&args.to)
        .map_err(|e| user_input_error!(UnsupportedTimezone, "{}", e))?;
    let converted = zoned.with_time_zone(target_tz);

    if args.verbose {
        verbose!(
            "resolve",
            "original={} converted={}",
            zoned.strftime("%Y-%m-%dT%H:%M:%S%:z"),
            converted.strftime("%Y-%m-%dT%H:%M:%S%:z")
        );
        let elapsed = start_instant.elapsed();
        verbose!("timing", "{:.3}ms", elapsed.as_secs_f64() * 1000.0);
    }

    if args.json {
        let json = serde_json::json!({
            "input": args.input,
            "from_timezone": zoned.time_zone().iana_name().unwrap_or("Unknown"),
            "to_timezone": args.to,
            "original": zoned.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            "converted": converted.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
        });
        emit_json(&json, args.no_newline);
    } else {
        output_value(
            &converted.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            args.no_newline,
        );
    }
    Ok(())
}

/// Handle `td info <date>` -- calendar metadata card.
fn handle_info(args: InfoArgs) -> Result<()> {
    let start_instant = std::time::Instant::now();
    let tz = resolve_timezone(&args.timezone)?;
    let now = resolve_now_zoned(&args.now, &tz)?;

    if args.verbose {
        verbose!("parse", "input={:?}", args.input);
        verbose!("parse", "timezone={}", tz.iana_name().unwrap_or("system"));
    }

    let zoned = parser::parse(&args.input, &now)
        .map_err(|e| user_input_error!(InvalidDateFormat, "{}", e.format_message()))?;

    let iwd = zoned.date().iso_week_date();
    let quarter = (zoned.month() - 1) / 3 + 1;
    let day_of_year = zoned.date().day_of_year();
    let days_in_year = if zoned.date().in_leap_year() {
        366
    } else {
        365
    };
    let epoch_secs = zoned.timestamp().as_second();
    let jdn = epoch_secs as f64 / 86400.0 + 2_440_587.5;

    if args.verbose {
        verbose!(
            "resolve",
            "date={} weekday={:?} quarter=Q{}",
            zoned.strftime("%Y-%m-%d"),
            zoned.weekday(),
            quarter
        );
        let elapsed = start_instant.elapsed();
        verbose!("timing", "{:.3}ms", elapsed.as_secs_f64() * 1000.0);
    }

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
        emit_json(&json, args.no_newline);
        return Ok(());
    }

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
    lines.push(format!("{bold}{cyan}  Date{reset}         {date_str}"));
    lines.push(format!("{bold}{cyan}  Time{reset}         {time_str}"));
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
    lines.push(format!("{bold}{cyan}  Leap Year{reset}    {leap_str}"));
    lines.push(format!("{bold}{cyan}  Unix Epoch{reset}   {epoch_secs}"));
    lines.push(format!("{bold}{cyan}  Julian Day{reset}   {jdn:.2}"));

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
            let _ = Config::load()?;
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
        _ => unreachable!(),
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
        _ => unreachable!(),
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
        ($dt:expr) => {{ Some($dt.parse::<jiff::Timestamp>().unwrap().to_zoned(utc())) }};
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
        let a = app(
            "2030-12-31 00:00",
            "br",
            utc(),
            now!("2030-12-31T00:00:00Z"),
        );

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
