//! TARDIS binary entry-point.
//!
//! 1. Parse CLI (`cli::Command`).
//! 2. Load configuration (`config::Config`).
//! 3. Merge both into an [`core::App`] context.
//! 4. Run the core pipeline and print the result.

use tardis_cli::{
    cli::Command,
    config::Config,
    core::{self, App},
    errors,
};

use errors::{Failable, Result};

/// Top-level execution wrapper.
/// Errors are funneled to `Failable::exit`, which prints a message and sets
/// the process’ exit-code.
fn main() {
    if let Err(err) = run() {
        err.exit();
    }
}

/// High-level flow, kept small for ease of unit/integration testing.
fn run() -> Result<()> {
    let cmd = Command::parse()?;
    let cfg = Config::load()?;

    let app = App::from_cli(&cmd, &cfg)?;
    let out = core::process(&app, &cfg.presets())?;

    println!("{out}");
    Ok(())
}

#[cfg(test)]
mod tests {
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
        core::process(app, presets).unwrap()
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
