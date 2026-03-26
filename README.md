# TARDIS

![Crates.io](https://img.shields.io/crates/v/tardis-cli)
![Docs.rs](https://img.shields.io/docsrs/tardis-cli)
![CI](https://github.com/hvpaiva/tardis-cli/actions/workflows/ci.yml/badge.svg)
[![codecov](https://codecov.io/gh/hvpaiva/tardis-cli/graph/badge.svg)](https://codecov.io/gh/hvpaiva/tardis-cli)
![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)

<p align="center">
  <img src="./assets/tardis.png" alt="TARDIS logo" width="200">
</p>

> **TARDIS** -- *Time And Relative Date Input Simplifier*.
> Like the Doctor's ship in *Doctor Who*, it translates human-friendly time
> expressions into precise datetimes right from your terminal.

---

## See it in action

```bash
td "next friday at 3pm" --now "2025-01-01T12:00:00Z" -t UTC
# 2025-01-03T15:00:00+00:00

td "tomorrow + 3 hours" -f "%Y-%m-%d %H:%M" --now "2025-01-01T12:00:00Z" -t UTC
# 2025-01-02 15:00

td @1735689600 -f "%Y-%m-%d" -t UTC
# 2025-01-01

td diff "2025-01-01" "2025-03-15" --now "2025-01-01T12:00:00Z" -t UTC
# 2 months, 14 days
# 6307200 seconds
# P2M14D

td info "2025-07-04" -t UTC --now "2025-01-01T12:00:00Z"
#   Date         Friday, July  4, 2025
#   Time         00:00:00 UTC
#   Week         W27, 2025
#   Quarter      Q3
#   ...
```

---

## Installation

### From crates.io

```bash
cargo install tardis-cli --locked
```

The binary is called `td`.

### From source

```bash
git clone https://github.com/hvpaiva/tardis-cli.git
cd tardis-cli
cargo install --path . --locked
```

### Verify

```bash
td --version
```

---

## Feature matrix

| Feature | Description |
|---------|-------------|
| Natural-language parsing | `"next friday"`, `"in 2 hours"`, `"tomorrow 14:30"` |
| Absolute dates | `"2025-03-15"`, `"March 15 2025"`, `"15/03/2025"` |
| Epoch input | `@1735689600` with smart precision (s/ms/us/ns) |
| Arithmetic | `"tomorrow + 3 hours"`, `"next friday - 30 minutes"` |
| Range expressions | `"last week"`, `"this month"`, `"Q3 2025"` |
| Language | English |
| Format presets | Named formats in config (e.g. `br` -> `%d/%m/%Y`) |
| Timezone conversion | Any IANA timezone via `--timezone` or `td tz` |
| JSON output | `--json` for scripting with `jq` |
| Batch mode | Pipe multiple expressions, one per line |
| `--skip-errors` | Continue on parse failure in batch mode |
| Verbose diagnostics | `--verbose` / `-v` with tagged stderr output |
| Subcommands | `diff`, `convert`, `tz`, `info`, `config`, `completions` |
| Shell completions | Bash, Zsh, Fish, Elvish, PowerShell |
| Cross-platform | Linux, macOS, Windows |

---

## Expression types

### Relative dates

```bash
td "today" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2025-01-01

td "tomorrow" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2025-01-02

td "yesterday" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2024-12-31

td "overmorrow" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2025-01-03
```

### Day references

```bash
td "next friday" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2025-01-03

td "last monday" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2024-12-30

td "this sunday at 18:00" --now "2025-01-01T12:00:00Z" -t UTC
# 2025-01-05T18:00:00+00:00
```

### Duration offsets

```bash
td "in 3 days" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2025-01-04

td "2 hours ago" --now "2025-01-01T12:00:00Z" -t UTC -f "%H:%M"
# 10:00

td "in 1 year 2 months 3 days" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2026-03-04
```

### Absolute dates

```bash
td "2025-03-15" -t UTC -f "%A, %B %d, %Y"
# Saturday, March 15, 2025

td "March 15 2025" -t UTC -f "%Y-%m-%d"
# 2025-03-15

td "2025-06-24 10:30" -t UTC -f "%Y-%m-%dT%H:%M:%S"
# 2025-06-24T10:30:00
```

### Epoch input

```bash
td @1735689600 -f "%Y-%m-%d" -t UTC
# 2025-01-01

td @1735689600000 -f "%Y-%m-%d" -t UTC
# 2025-01-01 (milliseconds auto-detected)

td @-86400 -f "%Y-%m-%d" -t UTC
# 1969-12-31
```

### Arithmetic

```bash
td "tomorrow + 3 hours" --now "2025-01-01T12:00:00Z" -t UTC
# 2025-01-02T15:00:00+00:00

td "next friday - 30 minutes" --now "2025-01-01T12:00:00Z" -t UTC -f "%H:%M"
# 23:30

td "yesterday + 1 year 2 months" --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2026-02-28
```

### Range expressions

Range expressions output two lines: start and end of the range.

```bash
td "last week" --now "2025-01-15T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2025-01-06
# 2025-01-12

td "this month" --now "2025-03-15T12:00:00Z" -t UTC -f "%Y-%m-%d"
# 2025-03-01
# 2025-03-31

td "Q3 2025" -t UTC -f "%Y-%m-%d"
# 2025-07-01
# 2025-09-30
```

### Time only

```bash
td "15:30" --now "2025-01-01T12:00:00Z" -t UTC
# 2025-01-01T15:30:00+00:00

td "9:00" --now "2025-01-01T12:00:00Z" -t UTC -f "%H:%M"
# 09:00
```

---

## Subcommands

### `td diff` -- Compute the difference between two dates

```bash
td diff "2025-01-01" "2025-03-15" --now "2025-01-01T12:00:00Z" -t UTC
# 2 months, 14 days
# 6307200 seconds
# P2M14D

td diff "2025-01-01" "2025-01-01 06:30" --now "2025-01-01T12:00:00Z" -t UTC
# 6 hours, 30 minutes
# 23400 seconds
# PT6H30M

td diff "2025-01-01" "2026-01-01" --now "2025-01-01T12:00:00Z" -t UTC --json
# {"human":"1 year","iso8601":"P1Y","seconds":31536000}
```

### `td convert` -- Convert between date formats

```bash
td convert "2025-01-01T12:00:00+00:00" --to "%d/%m/%Y" -t UTC
# 01/01/2025

td convert "01/01/2025" --from "%d/%m/%Y" --to iso8601 -t UTC
# 2025-01-01T00:00:00+00:00

td convert "2025-01-01" --to epoch -t UTC
# 1735689600
```

### `td tz` -- Convert a datetime between timezones

```bash
td tz "2025-01-01 12:00" --to "America/Sao_Paulo" --now "2025-01-01T12:00:00Z"
# 2025-01-01T09:00:00 -03

td tz "2025-01-01 09:00" --from "America/New_York" --to "Europe/London"
# 2025-01-01T14:00:00 GMT

td tz "now" --to "Asia/Tokyo" --now "2025-01-01T12:00:00Z" --json
# {"input":"now","from_timezone":"UTC","to_timezone":"Asia/Tokyo","original":"...","converted":"..."}
```

### `td info` -- Display calendar metadata for a date

```bash
td info "2025-07-04" -t UTC --now "2025-01-01T12:00:00Z"
#   Date         Friday, July  4, 2025
#   Time         00:00:00 UTC
#   Week         W27, 2025
#   Quarter      Q3
#   Day of Year  185/365
#   Leap Year    No
#   Unix Epoch   1751587200
#   Julian Day   2460861.50

td info "2024-02-29" -t UTC --now "2025-01-01T12:00:00Z" --json
# {"date":"2024-02-29","day_of_year":60,"days_in_year":366,"iso_week":"W09",
#  "iso_week_year":2024,"julian_day":"2460370.50","leap_year":true,
#  "quarter":1,"time":"00:00:00","timezone":"UTC","unix_epoch":1709164800,
#  "weekday":"Thursday"}
```

### `td config` -- Manage configuration

```bash
td config path      # Print config file location
td config show      # Dump effective configuration
td config edit      # Open config in $EDITOR
td config presets   # List all named format presets
```

### `td completions` -- Generate shell completions

```bash
td completions bash
td completions zsh
td completions fish
td completions elvish
td completions powershell
```

---

## Verbose mode

Use `-v` / `--verbose` to print diagnostics to stderr. Tags are grep-friendly.

```bash
td "tomorrow" -v --now "2025-01-01T12:00:00Z" -t UTC -f "%Y-%m-%d"
# stderr:
#   [config] path=/home/user/.config/tardis/config.toml
#   [config] format=%Y-%m-%d timezone=UTC
#   [parse]  input="tomorrow"
#   [parse]  effective_format=%Y-%m-%d timezone=UTC
#   [resolve] output="2025-01-02" epoch=1735776000
#   [timing] 0.142ms
# stdout:
#   2025-01-02
```

Filter specific phases:

```bash
td "tomorrow" -v 2>&1 1>/dev/null | grep '\[timing\]'
# [timing] 0.142ms
```

---

## Options reference

| Flag | Short | Description |
|------|-------|-------------|
| `--format <FMT>` | `-f` | Strftime pattern, preset name, `"epoch"`, or `"unix"` |
| `--timezone <TZ>` | `-t` | IANA timezone (e.g. `UTC`, `America/Sao_Paulo`) |
| `--now <DATETIME>` | | Override "now" (RFC 3339 format) for deterministic output |
| `--json` | `-j` | Output as JSON object |
| `--no-newline` | `-n` | Suppress trailing newline |
| `--verbose` | `-v` | Print diagnostics to stderr |
| `--skip-errors` | | In batch mode, skip unparseable lines instead of aborting |
| `--version` | `-V` | Print version |
| `--help` | `-h` | Print help |

**Format reference:** [jiff strftime](https://docs.rs/jiff/latest/jiff/fmt/strtime/index.html)
**Timezone reference:** [IANA Time Zone Database](https://www.iana.org/time-zones)

---

## Configuration

On first run, TARDIS creates a config file automatically.

| Platform | Path |
|----------|------|
| `$XDG_CONFIG_HOME` set | `$XDG_CONFIG_HOME/tardis/config.toml` |
| Linux (default) | `~/.config/tardis/config.toml` |
| macOS | `~/Library/Application Support/tardis/config.toml` |
| Windows | `%APPDATA%\tardis\config.toml` |

### Precedence

**CLI flags > Environment variables > Config file**

### Config file format

```toml
# Default output format (strftime pattern)
format = "%Y-%m-%dT%H:%M:%S"

# Default timezone (IANA name). Empty = system local.
timezone = ""

[formats]
# Named presets usable with --format
br       = "%d/%m/%Y"
short    = "%d/%m"
hour     = "%H:%M"
taskline = "%d.%m.%Y %H:%M"
```

### Using presets

```bash
td "today" -f br --now "2025-01-01T12:00:00Z" -t UTC
# 01/01/2025

td config presets
# NAME         FORMAT
# ----         ------
# br           %d/%m/%Y
# short        %d/%m
# hour         %H:%M
```

---

## Pipes and automation

### Batch mode

Pipe multiple expressions (one per line):

```bash
echo -e "today\ntomorrow\nnext friday" | td -f "%Y-%m-%d" -t UTC --now "2025-01-01T12:00:00Z"
# 2025-01-01
# 2025-01-02
# 2025-01-03
```

### `--skip-errors`

In batch mode, unparseable lines produce an error on stderr and an empty line on stdout (to preserve alignment). Exit code is 1 if any line failed.

```bash
echo -e "today\n???\ntomorrow" | td -f "%Y-%m-%d" -t UTC --now "2025-01-01T12:00:00Z" --skip-errors
# 2025-01-01
#                     (empty line for failed input)
# 2025-01-02
# stderr: Invalid date format: ...
```

### Scripting examples

```bash
# Embed a formatted date without trailing newline
echo "Deadline: $(td 'next monday 9am' -f '%H:%M' -n --now '2025-01-01T12:00:00Z' -t UTC)"
# Deadline: 09:00

# JSON + jq
td "tomorrow" --json --now "2025-01-01T12:00:00Z" -t UTC | jq -r .output
# 2025-01-02T12:00:00

# Convert epoch to human-readable in a pipeline
echo "@1735689600" | td -f "%A, %B %d, %Y" -t UTC
# Wednesday, January 01, 2025

# Range to CSV
td "Q3 2025" -f "%Y-%m-%d" -t UTC --json | jq -r '[.start, .end] | @csv'
# "2025-07-01","2025-09-30"
```

---

## Environment variables

| Variable | Purpose |
|----------|---------|
| `TARDIS_FORMAT` | Default format when `--format` is omitted |
| `TARDIS_TIMEZONE` | Default timezone when `--timezone` is omitted |
| `XDG_CONFIG_HOME` | Override config directory base path |
| `EDITOR` | Editor used by `td config edit` (default: `vi`) |
| `NO_COLOR` | Disable colored output in `td info` |

```bash
export TARDIS_FORMAT="%d/%m/%Y"
td "today" --now "2025-01-01T12:00:00Z" -t UTC
# 01/01/2025
```

---

## Library usage

`tardis-cli` is also a Rust library. Add it to your `Cargo.toml`:

```toml
[dependencies]
tardis-cli = "0.1"
```

```rust
use tardis_cli::core::{App, Preset, process};
use jiff::tz::TimeZone;

fn main() -> tardis_cli::Result<()> {
    let tz = TimeZone::get("UTC").unwrap();
    let app = App::new(
        "2025-03-15".to_string(),
        "%Y-%m-%d".to_string(),
        tz,
        None,
    );
    let output = process(&app, &[])?;
    println!("{}", output.formatted); // 2025-03-15
    println!("{}", output.epoch);     // 1742025600
    Ok(())
}
```

**Public modules:** `cli`, `config`, `core`, `errors`, `parser`

See [docs.rs/tardis-cli](https://docs.rs/tardis-cli) for the full API reference.

---

## Comparison

| Feature | `td` (TARDIS) | GNU `date` | `dateutils` | `dateparser` (Python) |
|---------|:---:|:---:|:---:|:---:|
| Natural language input | Yes | No | No | Yes |
| Arithmetic expressions | Yes | Limited (`-d "+3 days"`) | Yes | No |
| Range expressions | Yes | No | No | No |
| Natural-language parsing | Yes | No | No | Limited |
| Named format presets | Yes | No | No | No |
| Smart epoch detection | Yes (s/ms/us/ns) | No | No | No |
| JSON output | Yes | No | No | No |
| Batch mode | Yes | No | Yes | No |
| Calendar info card | Yes (`td info`) | No | No | No |
| Timezone conversion | Yes (`td tz`) | Yes | Yes | No |
| Date diff | Yes (`td diff`) | No | Yes | No |
| Format conversion | Yes (`td convert`) | Yes | Yes | No |
| Config file | Yes | No | No | No |
| Shell completions | Yes (5 shells) | No | No | N/A |
| Single static binary | Yes | Yes | Yes | No (Python) |
| Verbose diagnostics | Yes | No | No | No |

---

## Shell completions

Generate and install completions for your shell:

```bash
# Bash
td completions bash > ~/.local/share/bash-completion/completions/td

# Zsh (add ~/.zfunc to fpath if needed)
td completions zsh > ~/.zfunc/_td

# Fish
td completions fish > ~/.config/fish/completions/td.fish

# Elvish
td completions elvish > ~/.config/elvish/lib/td.elv

# PowerShell
td completions powershell > $PROFILE.CurrentUserAllHosts
```

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Batch mode: at least one line failed (with `--skip-errors`) |
| `64` | Usage error (bad input, unsupported format, invalid timezone) |
| `74` | I/O error |
| `78` | Configuration error |

---

## Contributing

Bug reports and pull requests are welcome.
See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for guidelines.

Quick start:

```bash
git clone https://github.com/hvpaiva/tardis-cli.git
cd tardis-cli
./scripts/dev-setup.sh   # installs tooling + git hooks
just check               # format + lint + test + audit
```

---

## License

[MIT License](./LICENCE.md)

---

## Trivia

The name **TARDIS** pays homage to the iconic, bigger-on-the-inside
time machine from *Doctor Who*. This CLI helps you navigate time too --
minus the wibbly-wobbly stuff.
