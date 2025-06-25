
# Changelog
All notable changes to **TARDIS** will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] – 2025-06-25
### Added
- **Natural-language parsing** of date/time expressions via `human-date-parser`
  (e.g. `"next Monday at 09:00"`, `"in 2 hours"`, `"2025-12-31 23:59"`).
- **Custom output formats** (`--format/-f`) using `chrono` strftime syntax.
- **Named presets**: reusable formats declared under `[formats]` in
  `config.toml`.
- **Time-zone selection** (`--timezone/-t`) with full IANA/Olson database
  via `chrono-tz`; falls back to system local TZ if none given.
- **Reference clock override** (`--now`) for deterministic runs / tests
  (RFC 3339 input).
- **Config file** (`config.toml`) auto-created on first run:
  - Default `format` and `timezone`
  - Commented template for easy editing
  - Respects `XDG_CONFIG_HOME` or OS-specific config directory.
- **Environment-variable overrides**
  - `TARDIS_FORMAT`
  - `TARDIS_TIMEZONE`
- **Cross-platform shell completions** (bash, zsh, fish, PowerShell, elvish)
  and man-page generated at build time (`build.rs`).
- **Helpful error messages**
  - Unknown time-zone → `UnsupportedTimezone`
  - Empty/absent format → `MissingArgument`
  - Unparsable input → `InvalidDateFormat`
- **Extensive test suite**
  - Core logic, CLI merge rules, config loader, env-var guard.
- **Developer tooling**
  - `just` recipes (`lint_all`, `bench_quick`, `flamegraph`, etc.)
  - CI workflows for lint, test, audit, vet, and publish.
- **License**: MIT.

[0.1.0]: https://github.com/hvpaiva/tardis/releases/tag/v0.1.0
