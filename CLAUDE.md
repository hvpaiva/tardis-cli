<!-- GSD:project-start source:PROJECT.md -->
## Project

**TARDIS CLI**

TARDIS (Time And Relative Date Input Simplifier) is a Rust CLI tool (`td`) that converts natural-language date expressions into machine-readable formats. It supports configurable output formats, timezone handling, preset management, batch processing, and JSON output. The goal is to evolve it from a functional but rough v0.1 into a polished, reference-quality v1.0 — a focused date/time Swiss army knife that replaces ad-hoc `date` commands and fragmented tooling.

**Core Value:** Parse any human date/time expression — in any supported locale — and produce the exact output format you need, fast and correctly. If this doesn't work reliably, nothing else matters.

### Constraints

- **Tech stack**: Rust, no async runtime — keep it fast and simple
- **CLI-first**: The `td` binary is the primary product; library API is secondary but should be clean
- **Backward compatibility**: All currently working expressions must continue to work after parser replacement
- **Minimum Rust version**: Follow `rust-toolchain.toml` (stable channel)
- **Dependency philosophy**: Fewer is better — justify every dependency
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust (Edition 2024) - All application code, tests, benchmarks, and build script
- Shell (POSIX sh / Bash) - Git hooks (`hooks/commit-msg.sh`, `hooks/pre-commit.sh`), dev scripts (`scripts/dev-setup.sh`, `scripts/guard-release-files.sh`)
- TOML - Configuration files, project metadata
## Runtime
- Rust stable toolchain (pinned via `rust-toolchain.toml`)
- Required components: `clippy`, `rustfmt`, `llvm-tools-preview`
- MSRV: 1.85 (enforced by `clippy.toml` and CI job `msrv`)
- Cargo (Rust standard)
- Lockfile: `Cargo.lock` present and committed
## Frameworks
- `clap` 4.5.40 - CLI argument parsing with derive macros. Features: `color`, `derive`, `env`
- `chrono` 0.4.41 - Date/time manipulation. Features: `serde`, `clock`
- `chrono-tz` 0.10.3 - IANA timezone support. Features: `serde`
- `human-date-parser` 0.3.1 - Natural-language date expression parsing (the core differentiator)
- `criterion` 0.6.0 - Benchmarking framework (`benches/parse.rs`)
- `assert_cmd` 2.0.17 - CLI integration testing (process spawning)
- `assert_fs` 1.1.3 - Filesystem fixtures for tests
- `predicates` 3.1.3 - Assertion matchers for CLI output
- `pretty_assertions` 1.4.1 - Diff-friendly assertion output
- `serial_test` 3.2.0 - Sequential test execution (for env-var-mutating tests)
- `tempfile` 3.20.0 - Temporary directories/files in tests
- `just` - Task runner (see `justfile`)
- `cocogitto` (cog) 7.x - Conventional commit enforcement and automated version bumping
- `cargo-deny` - License, advisory, and dependency audit (`deny.toml`)
- `cargo-vet` - Supply-chain security audits (`supply-chain/config.toml`)
- `cargo-nextest` - Parallel test runner
- `cargo-llvm-cov` - Code coverage via LLVM instrumentation
- `cargo-flamegraph` - Flamegraph profiling
- `hyperfine` - CLI benchmark tool
## Key Dependencies
- `clap` 4.5.40 - CLI framework. Uses derive macro for declarative argument definitions. Also provides `clap_complete` 4.5.54 for shell completion generation at runtime and build time.
- `human-date-parser` 0.3.1 - The core parsing engine. Converts natural-language strings like "next Friday" into `chrono` types via `from_human_time()`. Returns `ParseResult` enum (Date/DateTime/Time).
- `chrono` 0.4.41 - Date/time types (`DateTime`, `NaiveDateTime`), formatting via strftime patterns, RFC 3339 parsing.
- `chrono-tz` 0.10.3 - Full IANA timezone database compiled into the binary. Provides `Tz` enum for timezone-aware conversions.
- `config` 0.15.11 - Layered configuration loading (file + environment variable overlay). Used in `src/config.rs`.
- `serde` 1.0.219 - Serialization/deserialization derive for config structs. Features: `derive`.
- `toml` 0.8.23 - TOML format support for configuration files.
- `serde_json` 1.0 - JSON output mode (`--json` flag).
- `thiserror` 2.0.12 - Derive macro for `Error` enum in `src/errors.rs`.
- `dirs` 6.0.0 - Cross-platform config directory resolution (`dirs::config_dir()`).
- `iana-time-zone` 0.1.63 - Detects system local timezone when none is specified.
- `exitcode` 1.1.2 - Standard sysexits-compatible exit codes (USAGE, CONFIG, IOERR).
- `color-print` 0.3.7 - Compile-time ANSI color formatting for help text via `cstr!()` macro.
- `clap` 4.5.40 - Duplicated for `build.rs` to generate completions and man pages at compile time.
- `clap_complete` 4.5.54 - Shell completion script generation (Bash, Zsh, Fish, Elvish, PowerShell).
- `clap_mangen` 0.2.27 - Man page generation from clap command definition.
- `color-print` 0.3.7 - Required at build time because the `Cli` struct references `cstr!()` constants.
## Configuration
- `TARDIS_FORMAT` - Override default output format (env var, layered via `config` crate)
- `TARDIS_TIMEZONE` - Override default timezone (env var, layered via `config` crate)
- `XDG_CONFIG_HOME` - Override config directory base path
- `EDITOR` - Used by `td config edit` subcommand to open config file
- Config file location: `$XDG_CONFIG_HOME/tardis/config.toml` (or OS default via `dirs::config_dir()`)
- Config template embedded at compile time: `assets/config_template.toml`
- `build.rs` - Generates man page (`td.1`) and shell completions for all 5 shells into `$OUT_DIR`
- `rust-toolchain.toml` - Pins toolchain channel to `stable` with required components
- `clippy.toml` - Sets MSRV to 1.85 for lint compatibility
- `rustfmt.toml` - Edition 2024, max_width 100, field init shorthand, try shorthand
- `deny.toml` - License allowlist (MIT, Apache-2.0, BSD-3-Clause, ISC, MPL-2.0, Zlib, Unicode-3.0, BSL-1.0), advisory checks, ban checks, source checks
- LTO enabled (`lto = true`)
- Symbols stripped (`strip = "symbols"`)
## Toolchain & Infrastructure
- `just` with `justfile` at project root. Key recipes: `check`, `fmt`, `lint`, `test`, `audit`, `coverage`, `bench`, `flamegraph`, `hooks`, `setup`
- `rustfmt` via `cargo fmt --all`
- Config: `rustfmt.toml` (edition 2024, max_width 100)
- `clippy` via `cargo clippy --all-targets --all-features -- -D warnings`
- MSRV: 1.85 (set in `clippy.toml`)
- `cargo-deny` - License compliance, security advisories, duplicate crate detection (`deny.toml`)
- `cargo-vet` - Supply-chain integrity audits with imports from bytecode-alliance, Google, Mozilla (`supply-chain/config.toml`)
- `cocogitto` (cog) - Enforces conventional commits via `commit-msg` git hook
- Scopes: `core`, `cli`, `config`, `infra`, `deps` (defined in `cog.toml`)
- Automated version bumping and changelog generation via `cog bump --auto`
- GitHub Actions with 4 workflows:
- `cargo-llvm-cov` with `--fail-under-lines 80` threshold
- Codecov integration (`.codecov.yml`) with 80% target for both project and patch
- Dependabot configured for cargo (weekly) and github-actions (weekly) in `.github/dependabot.yml`
- Commit prefix conventions: `build` for cargo updates, `ci` for actions updates
- `commit-msg` (`hooks/commit-msg.sh`) - Runs `cog verify` on commit message
- `pre-commit` (`hooks/pre-commit.sh`) - Runs release-file guard, `cargo fmt --check`, `cargo clippy`, `cargo test`
- `scripts/guard-release-files.sh` prevents manual edits to `CHANGELOG.md` and `Cargo.toml` version field
- Bypass with `SKIP_RELEASE_GUARD=1` locally or `release-override` PR label in CI
## Key Technical Decisions
- Core value proposition of the CLI. Converts freeform English date/time expressions into structured chrono types.
- Lightweight (PEG parser, pest-based). No external service calls.
- Provides layered configuration: file source + environment variable overlay in a single builder pattern.
- `TARDIS_FORMAT` and `TARDIS_TIMEZONE` env vars automatically override config file values.
- Derive macros keep CLI definition declarative and type-safe in `src/cli.rs`.
- `build.rs` mirrors the CLI struct to generate man pages and shell completions at compile time without runtime cost.
- The `build.rs` CLI struct must be kept in sync with `src/cli.rs` manually (documented with a comment).
- Derive macro generates `Display` and `From` impls. Two-level error hierarchy: `UserInput` (exit code USAGE) vs `System` (exit code CONFIG/IOERR).
- Custom macros `user_input_error!` and `system_error!` reduce boilerplate.
- Latest Rust edition. Enables modern language features and formatting defaults.
- Provides sysexits-compatible exit codes for proper shell integration (64=USAGE, 78=CONFIG, 74=IOERR).
- `include_str!()` embeds config template at compile time (`src/config.rs` line 21)
- `cstr!()` macro from `color-print` embeds ANSI color codes in help text at compile time
- Binary name is `td` (short for TARDIS), defined as `[[bin]]` in `Cargo.toml`
- No async runtime - entirely synchronous, single-threaded CLI
## Platform Requirements
- Rust stable toolchain (components: rustfmt, clippy, llvm-tools-preview)
- `just`, `cog`, `cargo-deny`, `cargo-nextest`, `cargo-llvm-cov` (installable via `scripts/dev-setup.sh`)
- Optional: `hyperfine`, `cargo-flamegraph`, `cargo-audit`, `cargo-vet`
- Single static binary (`td`)
- Runs on Linux, macOS, Windows (smoke-tested in CI)
- No runtime dependencies beyond OS-provided timezone data (compiled into binary via `chrono-tz`)
- Config file auto-created on first run
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Conventions
- Use `snake_case.rs` for all Rust source files: `cli.rs`, `core.rs`, `config.rs`, `errors.rs`
- Integration test files also use `snake_case`: `cli_integration.rs`
- Benchmark files use `snake_case`: `parse.rs`
- Shell scripts use `kebab-case`: `dev-setup.sh`, `guard-release-files.sh`, `commit-msg.sh`, `pre-commit.sh`
- One module per file in a flat `src/` layout. No nested module directories.
- Module re-exports are centralised in `src/lib.rs`:
- Use `snake_case` for all functions: `from_cli`, `config_path`, `resolve_format`, `render_datetime`
- Private helper functions are module-level (not methods) when they do not need `self`: `create_config_if_missing`, `format_output`, `resolve_format`
- Constructor-style functions use `new` or `from_*` prefix: `App::new`, `App::from_cli`, `Command::from_raw_cli`, `Preset::new`
- Test helper functions are module-scoped inside `#[cfg(test)] mod tests`: `parse_ok`, `make_cmd`, `make_cfg`, `td_cmd`, `write_config`
- Use `PascalCase` for structs and enums: `App`, `Config`, `Command`, `Cli`, `SubCmd`, `ConfigAction`, `ShellType`
- Error enums use descriptive `PascalCase` variant names: `InvalidDateFormat`, `UnsupportedTimezone`, `AmbiguousDateTime`, `MissingArgument`
- Type aliases are declared at the crate root: `pub type Result<T> = std::result::Result<T, Error>;`
- Use `SCREAMING_SNAKE_CASE` for constants: `STYLES`, `AFTER_LONG_HELP`, `INPUT_HELP`, `APP_DIR`, `CONFIG_FILE`, `TEMPLATE`
- Module-private constants use `const` (not `static`): `const APP_DIR: &str = "tardis";`
- Use `snake_case` for all local variables and struct fields
- Abbreviations are acceptable when scoped tightly: `cfg`, `cmd`, `tz`, `fmt`, `dt`
## Code Style
- Formatter: `rustfmt` with custom configuration in `rustfmt.toml`
- Key `rustfmt.toml` settings:
- All code must pass `cargo fmt --all --check` before commit
- Linter: `clippy` with strict settings
- `clippy.toml` enforces: `msrv = "1.85"`
- Clippy is invoked with warnings as errors: `cargo clippy --all-targets --all-features -- -D warnings`
- In test modules where `expect()` is needed for test setup, suppress the lint explicitly:
- `.editorconfig` is present at the project root
- Key rules:
- `.gitattributes` forces LF line endings for all text files (`*.rs`, `*.toml`, `*.yml`, `*.yaml`, `*.sh`, `*.md`, etc.)
- Binary files (images, archives) are marked explicitly as binary
- `*.lock` files suppress diff output
## Import Organization
- Group related imports with a single `use` block and nested paths:
- Separate standard library, external crates, and crate-local imports with blank lines
- Use `crate::` prefix for intra-crate references, not relative paths
- None configured. All imports use literal crate paths.
## Error Handling Conventions
- All errors flow through a single `Error` enum defined in `src/errors.rs`
- Two top-level categories:
- Both sub-enums derive `thiserror::Error` for automatic `Display` implementations
- Crate-wide `Result<T>` alias: `pub type Result<T> = std::result::Result<T, Error>;`
- Use the `user_input_error!` and `system_error!` convenience macros instead of constructing error variants directly:
- Both macros support: literal string, format string with args, or no-arg (empty string)
- Use `?` operator with `From` implementations for automatic conversion
- `From<std::io::Error>` and `From<config::ConfigError>` are implemented in `src/config.rs`
- Map errors from external crates using `.map_err()` with the appropriate macro
- The `Error::exit()` method maps errors to specific exit codes via the `exitcode` crate:
- User errors print to stderr without prefix; system errors print with `"System error: "` prefix
## Documentation Patterns
- Every module starts with `//!` doc comments explaining its purpose:
- Use `**bold**` for project/module names in doc comments
- Public functions and types use `///` doc comments
- Use imperative mood: "Parse from arbitrary arg iterator", "Build an App from the parsed CLI"
- Include bullet points for parameter explanations:
- Reference other types with `[`backtick links`]`: `[`App`]`, `[`Preset`]`, `[`process`]`, `[`Result`]`
- Use `//` for brief contextual clarification, especially in `main.rs` flow control
- Avoid redundant comments that merely restate the code
- Long help strings use `color_print::cstr!` macro for ANSI-colored terminal output
- Defined as module-level `const` values: `ABOUT_HELP`, `AFTER_LONG_HELP`, `INPUT_HELP`, etc.
- Help text includes hyperlinks to external documentation (chrono strftime, GitHub readme)
- `cargo doc --no-deps` must pass with `RUSTDOCFLAGS="-D warnings"` (enforced in CI)
- `just doc` opens generated documentation in the browser
## Git and Release Conventions
- Strictly enforced [Conventional Commits](https://www.conventionalcommits.org/) via cocogitto
- Allowed scopes defined in `cog.toml`: `core`, `cli`, `config`, `infra`, `deps`
- Examples from git log:
- The `commit-msg` git hook runs `cog verify --file "$1"` to validate every commit message
- `hooks/pre-commit.sh` runs on every commit:
- Hooks are installed by cocogitto: `cog install-hook --all --overwrite`
- Single `main` branch; only `main` is allowed for bumps (`branch_whitelist = ["main"]` in `cog.toml`)
- Feature branches forked from `main`, merged via PR
- PRs require CI to pass (format, lint, test, audit, commit validation, MSRV check)
- `CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format
- Automatically generated by cocogitto during the release workflow (`cog changelog`)
- Manual edits to `CHANGELOG.md` are blocked by `scripts/guard-release-files.sh` (pre-commit hook and CI)
- Bypass locally with `SKIP_RELEASE_GUARD=1 git commit ...`; in CI, add the `release-override` PR label
- Version lives in `Cargo.toml` (currently `0.1.0`)
- Manual edits to the `version` field in `Cargo.toml` are blocked by the release guard
- Automated version bumps via cocogitto `pre_bump_hooks` in `cog.toml`:
- Tag prefix: `v` (e.g. `v0.1.0`)
- Triggered manually via `workflow_dispatch` on `.github/workflows/release.yml`
- Steps: checkout -> format/lint/test -> `cog bump --auto` -> push tags -> generate release notes -> create GitHub Release -> `cargo publish` to crates.io
- Requires `CARGO_REGISTRY_TOKEN` secret for crates.io publishing
## Dependency Management
- `cargo-deny` with configuration in `deny.toml`:
- Supply chain verification via `cargo-vet` with audit data in `supply-chain/`
- Configured in `.github/dependabot.yml` for both `cargo` and `github-actions` ecosystems
- Weekly update schedule
- Commit messages use conventional format: `build` prefix for cargo, `ci` prefix for actions
## Project Governance
- `CONTRIBUTING.md` outlines the workflow: fork -> branch -> conventional commits -> PR -> CI
- Quick setup via `./scripts/dev-setup.sh` which installs all required tooling and git hooks
- Key guideline: "Don't edit CHANGELOG.md or version in Cargo.toml manually"
- `CODE_OF_CONDUCT.md` follows Contributor Covenant v2.1
- Enforcement contact: contact@hvpaiva.dev
- `SECURITY.md` provides vulnerability reporting instructions
- Preferred channel: GitHub Private Reporting
- Automated security tools: Dependabot, secret scanning, CodeQL, cargo-deny
- Only the latest version is supported
- `.github/CODEOWNERS`: `* @hvpaiva` (single owner for all files)
- Bug report: `.github/ISSUE_TEMPLATE/bug_report.yml` (structured YAML form)
- Feature request: `.github/ISSUE_TEMPLATE/feature_request.yml` (structured YAML form)
- `.github/pull_request_template.md` with Summary, Motivation, Changes, and Checklist sections
- Checklist items: `just check` passes, tests added, docs updated, conventional commits
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Flat module structure within a single Rust crate (no workspace, no nested modules)
- Library + binary split: core logic is in the library (`src/lib.rs`) and the binary (`src/main.rs`) consumes it
- Configuration layering with three-tier precedence: CLI flags > environment variables > config file
- Typed error hierarchy separating user-caused errors from system errors
- Synchronous, single-threaded execution (no async runtime)
## Layers
- Purpose: Parse command-line arguments, read stdin, dispatch subcommands, format output
- Location: `src/cli.rs`, `src/main.rs`
- Contains: `Cli` (clap derive struct), `Command` (normalized intermediate), `SubCmd`, `ConfigAction`, `ShellType` enums, styled help text constants
- Depends on: `errors` (for `Result`, `user_input_error!` macro)
- Used by: `main.rs` (binary entry point)
- Purpose: Parse natural-language date expressions, resolve format presets, apply timezone, render output
- Location: `src/core.rs`
- Contains: `App` (immutable context), `Preset` (named format), `ProcessOutput` (result struct), `process()` (main pipeline function), `resolve_format()`, `render_datetime()`, `format_output()`
- Depends on: `cli::Command`, `config::Config`, `errors` (for `Result`, macros)
- Used by: `main.rs` (via `core::process()`)
- Purpose: Load, merge, and bootstrap user configuration from TOML file + env vars
- Location: `src/config.rs`
- Contains: `Config` struct (deserialized TOML), `config_path()`, `create_config_if_missing()`
- Depends on: `errors` (for `Result`, `Error`, `SystemError`), `core::Preset`
- Used by: `main.rs`, `core.rs` (via `Config::load()` and `Config::presets()`)
- Purpose: Centralized error types, exit code mapping, convenience macros
- Location: `src/errors.rs`
- Contains: `Error` enum (top-level), `UserInputError` enum (7 variants), `SystemError` enum (2 variants), `Result<T>` alias, `user_input_error!` and `system_error!` macros
- Depends on: `thiserror`, `exitcode`
- Used by: Every other module
- Purpose: Generate man pages and shell completions at compile time
- Location: `build.rs`
- Contains: Minimal `Cli` mirror struct, man page generation via `clap_mangen`, completion generation via `clap_complete`
- Depends on: `clap`, `clap_complete`, `clap_mangen`
- Used by: Cargo build system (outputs to `$OUT_DIR`)
## Data Flow
- No persistent state. Each invocation is stateless.
- Configuration file is read-only (except auto-creation on first run).
- The `App` struct is an immutable snapshot of merged CLI + config values passed into `core::process()`.
## Key Abstractions
- Purpose: Bridge between raw clap-parsed `Cli` and the domain logic. Handles stdin reading, default "now" value, RFC 3339 parsing of `--now`.
- Defined in: `src/cli.rs`
- Pattern: Builder/factory - `Command::from_raw_cli()` and `Command::parse_from()` normalize multiple input sources into a uniform struct.
- Purpose: Fully resolved context for date processing: the date expression, output format, timezone, and optional "now" override.
- Defined in: `src/core.rs`
- Pattern: Value object constructed via `App::from_cli()` which merges `Command` + `Config`. Once constructed, it is read-only.
- Purpose: Carries both the formatted string and the Unix epoch timestamp for JSON output.
- Defined in: `src/core.rs`
- Pattern: Simple product type returned from `core::process()`.
- Purpose: Maps a short name (e.g., `"br"`) to a chrono strftime pattern (e.g., `"%d/%m/%Y"`).
- Defined in: `src/core.rs`
- Pattern: Extracted from `Config.formats` HashMap via `Config::presets()`.
- Purpose: Distinguish errors the user can fix (bad input, unsupported timezone) from system failures (IO, config corruption). Each variant maps to a specific exit code.
- Defined in: `src/errors.rs`
- Pattern: `thiserror`-derived enums with `#[from]` conversions. Two convenience macros (`user_input_error!`, `system_error!`) for ergonomic construction.
## Entry Points
- Location: `src/main.rs`
- Binary name: `td` (configured in `Cargo.toml` `[[bin]]` section)
- `main()` calls `run()`. If `run()` returns `Err`, the error's `.exit()` method prints to stderr and terminates with the appropriate exit code.
- `run()` orchestrates: CLI parse -> subcommand dispatch OR default flow (normalize input -> load config -> batch/single processing -> print output).
- Location: `src/lib.rs`
- Exports four public modules: `cli`, `config`, `core`, `errors`
- Re-exports `errors::Error` and `errors::Result` at crate root for convenience
- The library is designed to be consumed by the binary but is also published to crates.io as a reusable library.
- `clap::Parser::parse()` populates `Cli` struct from `std::env::args_os()`
- `Cli.subcmd` is checked first: if `Some(SubCmd::Config { .. })` or `Some(SubCmd::Completions { .. })`, the corresponding handler runs and returns
- Otherwise, `Command::from_raw_cli(cli, stdin, is_terminal)` normalizes the positional input: prefers explicit arg, falls back to stdin if piped, defaults to `"now"` in terminal mode
- `App::from_cli(&cmd, &cfg)` merges the normalized command with loaded config to produce the final processing context
- Location: `build.rs`
- Triggers: Rerun on `build.rs` changes
- Generates: `td.1` man page, shell completions for Bash/Zsh/Fish/Elvish/PowerShell into `$OUT_DIR`
- Uses a **separate minimal `Cli` mirror struct** (must be kept in sync with `src/cli.rs`)
## Error Handling
- All functions in the library return `Result<T>` (alias for `std::result::Result<T, Error>`)
- Errors are constructed via `user_input_error!` and `system_error!` macros which accept format-string syntax
- `thiserror` derives `Display` and `Error` traits automatically
- `Error::exit()` method on the top-level enum maps to process exit codes via the `exitcode` crate:
- `From<std::io::Error>` and `From<config::ConfigError>` are implemented for automatic `?` propagation
- The binary's `main()` catches the top-level `Result` and calls `err.exit()` on failure
- `UserInputError::InvalidDateFormat` - natural language expression could not be parsed
- `UserInputError::UnsupportedFormat` - chrono format string is invalid
- `UserInputError::InvalidDate` - date component is invalid
- `UserInputError::AmbiguousDateTime` - DST ambiguity in target timezone
- `UserInputError::UnsupportedTimezone` - IANA timezone ID not recognized
- `UserInputError::InvalidNow` - `--now` value is not valid RFC 3339
- `UserInputError::MissingArgument` - required value (format) not provided anywhere
## Cross-Cutting Concerns
- CLI layer: clap validates flag syntax; `Command::from_cli()` validates `--now` as RFC 3339
- Core layer: `App::from_cli()` validates timezone and format are non-empty; `process()` validates epoch syntax and human date parsing
- Config layer: `config::Config::load()` validates TOML structure via `try_deserialize()`
- Plain text (default): just the formatted date string
- JSON (`--json`): object with `input`, `output`, `epoch`, `timezone`, `format` fields
- No-newline (`-n`): suppresses trailing newline in either mode
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
