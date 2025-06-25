//! Configuration loading and helpers for **TARDIS**.
//!
//! * Reads `config.toml` from the user-specific config directory
//!   (`$XDG_CONFIG_HOME/tardis` or OS default).
//! * Overlays values from environment variables prefixed with **`TARDIS_`**.
//! * Automatically bootstraps the file from an embedded template on first run.

use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use config::{Environment, File};
use serde::Deserialize;

use crate::{Error, Result, core::Preset, errors::SystemError, system_error};

const APP_DIR: &str = "tardis";
const CONFIG_FILE: &str = "config.toml";
const TEMPLATE: &str = include_str!("../assets/config_template.toml");

/// In-memory representation of the user configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Default output format (ISO-8601 by default).
    pub format: String,
    /// Time-zone identifier recognised by `chrono-tz` (e.g. `"America/Sao_Paulo"`).
    pub timezone: String,
    /// User-defined named formats.
    pub formats: Option<HashMap<String, String>>,
}

impl Config {
    /// Load the effective configuration, creating the file from the embedded
    /// template if it does not yet exist.
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        create_config_if_missing(&path)?;

        config::Config::builder()
            .add_source(File::from(path))
            .add_source(
                Environment::with_prefix("TARDIS")
                    .separator("_")
                    .ignore_empty(true),
            )
            .build()?
            .try_deserialize()
            .map_err(Into::into)
    }

    /// Convert the `[formats]` table into a list of [`Preset`]s.
    pub fn presets(&self) -> Vec<Preset> {
        self.formats
            .as_ref()
            .map(|m| {
                m.iter()
                    .map(|(name, fmt)| Preset::new(name.clone(), fmt.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Resolve the absolute path to `config.toml`.
fn config_path() -> Result<PathBuf> {
    let base_dir = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .ok_or_else(|| {
            system_error!(
                Config,
                "Could not locate configuration directory; set $XDG_CONFIG_HOME or ensure the OS default exists."
            )
        })?;

    Ok(base_dir.join(APP_DIR).join(CONFIG_FILE))
}

/// Create the configuration file (and parent directory) if it is missing.
fn create_config_if_missing(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, TEMPLATE.trim_start())?;
    Ok(())
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::System(SystemError::Io(e))
    }
}

impl From<config::ConfigError> for Error {
    fn from(e: config::ConfigError) -> Self {
        system_error!(Config, "{}", e)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use assert_fs::{TempDir, prelude::*};
    use serial_test::serial;
    use std::{env, ffi::OsString, fs};

    struct EnvGuard {
        key: &'static str,
        prior: Option<OsString>,
    }

    impl EnvGuard {
        /// Set env var to `value`, returning a guard that restores it later.
        fn set(key: &'static str, value: impl Into<OsString>) -> Self {
            let prior = env::var_os(key);

            unsafe { env::set_var(key, value.into()) };
            Self { key, prior }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prior {
                Some(val) => unsafe { env::set_var(self.key, val) },
                None => unsafe { env::remove_var(self.key) },
            }
        }
    }

    fn write_config(tmp: &TempDir, contents: &str) {
        let dir = tmp.child("tardis");
        dir.create_dir_all().unwrap();
        dir.child("config.toml").write_str(contents).unwrap();
    }

    #[test]
    #[serial]
    fn config_path_respects_xdg_config_home() {
        let tmp = TempDir::new().unwrap();
        let _home = EnvGuard::set("XDG_CONFIG_HOME", tmp.path());

        let path = super::config_path().expect("path resolution failed");
        assert!(path.starts_with(tmp.path()));
        assert!(path.ends_with("tardis/config.toml"));
    }

    #[test]
    #[serial]
    fn load_creates_file_if_missing() {
        let tmp = TempDir::new().unwrap();
        let _home = EnvGuard::set("XDG_CONFIG_HOME", tmp.path());

        let cfg_path = super::config_path().unwrap();
        assert!(!cfg_path.exists());

        let cfg = Config::load().expect("load must succeed");
        assert!(cfg_path.exists());
        let contents = fs::read_to_string(&cfg_path).unwrap();
        assert!(!contents.is_empty(), "template should be written");
        assert!(!cfg.format.is_empty());
        assert!(cfg.timezone.is_empty());
    }

    #[test]
    #[serial]
    fn load_reads_existing_file() {
        let tmp = TempDir::new().unwrap();
        let _home = EnvGuard::set("XDG_CONFIG_HOME", tmp.path());

        write_config(
            &tmp,
            r#"
format   = "%Y"
timezone = "UTC"

[formats]
short = "%H:%M"
"#,
        );
        let cfg = Config::load().unwrap();
        assert_eq!(cfg.format, "%Y");
        assert_eq!(cfg.timezone, "UTC");
        assert_eq!(cfg.presets().len(), 1);
        assert_eq!(cfg.presets()[0].name, "short");
    }

    #[test]
    #[serial]
    fn env_vars_override_config_file() {
        let tmp = TempDir::new().unwrap();
        let _home = EnvGuard::set("XDG_CONFIG_HOME", tmp.path());
        write_config(
            &tmp,
            r#"
        format = "%Y"
        timezone = "UTC"
        "#,
        );

        let _fmt = EnvGuard::set("TARDIS_FORMAT", "%d");

        let cfg = Config::load().unwrap();
        assert_eq!(cfg.format, "%d");
    }

    #[test]
    #[serial]
    fn blank_env_var_is_ignored() {
        let tmp = TempDir::new().unwrap();
        let _home = EnvGuard::set("XDG_CONFIG_HOME", tmp.path());
        write_config(
            &tmp,
            r#"
        format = "%d"
        timezone = "UTC"
        "#,
        );

        let _tz = EnvGuard::set("TARDIS_TIMEZONE", "");

        let cfg = Config::load().unwrap();
        assert_eq!(cfg.timezone, "UTC");
    }

    #[test]
    fn presets_conversion_from_formats_table() {
        let cfg = Config {
            format: "%Y".into(),
            timezone: "UTC".into(),
            formats: Some(
                [
                    ("iso".to_string(), "%Y-%m-%d".to_string()),
                    ("time".to_string(), "%H:%M".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
        };
        let presets = cfg.presets();
        assert_eq!(presets.len(), 2);
        assert!(presets.iter().any(|p| p.name == "iso"));
        assert!(presets.iter().any(|p| p.format == "%H:%M"));
    }

    #[test]
    fn presets_empty_when_none() {
        let cfg = Config {
            format: "%Y".into(),
            timezone: "UTC".into(),
            formats: None,
        };
        assert!(cfg.presets().is_empty());
    }

    #[test]
    #[serial]
    fn load_fails_on_invalid_toml() {
        let tmp = TempDir::new().unwrap();
        let _home = EnvGuard::set("XDG_CONFIG_HOME", tmp.path());
        write_config(&tmp, "not toml at all");

        assert!(Config::load().is_err());
    }

    #[test]
    fn create_config_is_noop_if_file_exists() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.child("config.toml");
        file.write_str("format=\"%Y\"").unwrap();

        let before = fs::read_to_string(&file).unwrap();
        super::create_config_if_missing(file.path()).unwrap();
        let after = fs::read_to_string(&file).unwrap();
        assert_eq!(before, after);
    }
}
