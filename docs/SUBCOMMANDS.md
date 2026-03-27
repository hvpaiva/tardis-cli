# Subcommand Reference

Complete reference for all `td` subcommands.

All examples run under a test harness that sets `TARDIS_NOW=2025-01-15T10:30:00Z`
and `TZ=UTC` for deterministic output.

---

## diff -- Compute date differences

Computes the duration between two date expressions.  Both dates accept the
same natural-language expressions as the main `td` command, including epoch
timestamps with `@` prefix.

**Usage:** `td diff DATE1 DATE2 [OPTIONS]`

### Options

| Flag | Long | Value | Description |
|------|------|-------|-------------|
| `-o` | `--output` | `human` \| `seconds` \| `iso` | Output format (default: human) |
| `-j` | `--json` | | Output as JSON |
| `-n` | `--no-newline` | | Suppress trailing newline |
| | `--now` | DATETIME | Override "now" (RFC 3339) |
| `-t` | `--timezone` | TZ | Timezone for resolution |
| `-v` | `--verbose` | | Print diagnostics to stderr |

### Examples

Human-readable difference:

```console
$ td diff "2025-01-01" "2025-03-15"
2mo 14d

```

Difference in seconds:

```console
$ td diff "2025-01-01" "2025-06-01" --output seconds
13046400

```

ISO 8601 duration:

```console
$ td diff "2025-01-01" "2025-03-15" --output iso
P2M14D

```

JSON output:

```console
$ td diff yesterday tomorrow --json
{"human":"2d","iso8601":"P2D","seconds":172800}

```

---

## convert -- Format conversion

Takes a date expression or formatted date string and re-renders it in a
target format.  When `--from` is provided, the input is parsed according to
that strptime pattern.  When omitted, the input format is auto-detected.

**Usage:** `td convert INPUT --to FORMAT [OPTIONS]`

### Built-in format names

- `iso8601` -- ISO 8601 format
- `rfc3339` -- RFC 3339 format
- `rfc2822` -- RFC 2822 format
- `epoch` / `unix` -- Unix timestamp (seconds)

### Options

| Flag | Long | Value | Description |
|------|------|-------|-------------|
| | `--from` | FORMAT | Input format (auto-detected if omitted) |
| | `--to` | FORMAT | Output format (required) |
| `-j` | `--json` | | Output as JSON |
| `-n` | `--no-newline` | | Suppress trailing newline |
| | `--now` | DATETIME | Override "now" (RFC 3339) |
| `-t` | `--timezone` | TZ | Timezone for resolution |
| `-v` | `--verbose` | | Print diagnostics to stderr |

### Examples

Convert to epoch:

```console
$ td convert "2025-06-24" --to epoch
1750723200

```

Convert from ISO to custom format:

```console
$ td convert "2025-06-24T09:00:00Z" --to "%d/%m/%Y %H:%M"
24/06/2025 09:00

```

Convert a bare epoch timestamp:

```console
$ td convert 1719244800 --to "%Y-%m-%d"
2024-06-24

```

With explicit input format (note: the `--from` format must include a
timezone specifier such as `%z` or `%:z` when converting to a
timezone-aware output):

```bash
td convert "24/06/2025 +0000" --from "%d/%m/%Y %z" --to iso8601 -t UTC
# 2025-06-24T00:00:00+00:00
```

---

## tz -- Timezone conversion

Converts a datetime from one timezone to another.  The source timezone is
auto-detected from the system or input when `--from` is omitted.

**Usage:** `td tz INPUT --to TIMEZONE [OPTIONS]`

### Options

| Flag | Long | Value | Description |
|------|------|-------|-------------|
| | `--from` | TZ | Source timezone (auto-detected if omitted) |
| | `--to` | TZ | Target timezone (required) |
| `-j` | `--json` | | Output as JSON |
| `-n` | `--no-newline` | | Suppress trailing newline |
| | `--now` | DATETIME | Override "now" (RFC 3339) |
| `-v` | `--verbose` | | Print diagnostics to stderr |

### Examples

Convert to UTC:

```console
$ td tz "now" --to UTC
2025-01-15T10:30:00+00:00

```

Convert from US Eastern to Sao Paulo:

```console
$ td tz "2025-06-24 12:00" --from America/New_York --to America/Sao_Paulo
2025-06-24T13:00:00-03:00

```

Convert to Tokyo time:

```console
$ td tz "tomorrow" --to Asia/Tokyo
2025-01-16T09:00:00+09:00

```

---

## info -- Calendar metadata

Displays detailed calendar metadata for a date expression.  When no
expression is provided, defaults to "now".

Output includes: year, month, day, weekday, week number, quarter, day of
year, Julian Day Number, Unix epoch, timezone, leap year status, and DST
status.

**Usage:** `td info [DATE] [OPTIONS]`

### Options

| Flag | Long | Value | Description |
|------|------|-------|-------------|
| `-j` | `--json` | | Output as JSON |
| `-n` | `--no-newline` | | Suppress trailing newline |
| | `--now` | DATETIME | Override "now" (RFC 3339) |
| `-t` | `--timezone` | TZ | Timezone for resolution |
| `-v` | `--verbose` | | Print diagnostics to stderr |

### Examples

Info for current date:

```console
$ td info
  Date         Wednesday, January 15, 2025
  Time         10:30:00 UTC
  Week         W03, 2025
  Quarter      Q1
  Day of Year  15/365
  Leap Year    No
  Unix Epoch   1736937000
  Julian Day   2460690.94

```

Info for a specific date:

```console
$ td info "2025-12-25"
  Date         Thursday, December 25, 2025
  Time         00:00:00 UTC
  Week         W52, 2025
  Quarter      Q4
  Day of Year  359/365
  Leap Year    No
  Unix Epoch   1766620800
  Julian Day   2461034.50

```

JSON output with all metadata:

```console
$ td info "2025-01-01" --json
{"date":"2025-01-01","day_of_year":1,"days_in_year":365,"iso_week":"W01","iso_week_year":2025,"julian_day":"2460676.50","leap_year":false,"quarter":1,"time":"00:00:00","timezone":"UTC","unix_epoch":1735689600,"weekday":"Wednesday"}

```

Info for a relative expression:

```console
$ td info "3 days ago"
  Date         Sunday, January 12, 2025
  Time         10:30:00 UTC
  Week         W02, 2025
  Quarter      Q1
  Day of Year  12/365
  Leap Year    No
  Unix Epoch   1736677800
  Julian Day   2460687.94

```

---

## range -- Date range expansion

Interprets a date expression as a time period and expands it into a start
and end datetime.

**Granularity expansion:** the range end is determined by the smallest
unspecified time unit.  For example, "this week" expands Monday through
Sunday, while "tomorrow at 3pm" expands to the full hour 15:00-15:59.

**Usage:** `td range EXPRESSION [OPTIONS]`

### Options

| Flag | Long | Value | Description |
|------|------|-------|-------------|
| `-f` | `--format` | FMT | Output format (strftime or preset) |
| `-t` | `--timezone` | TZ | Timezone to apply |
| | `--now` | DATETIME | Override "now" (RFC 3339) |
| `-d` | `--delimiter` | DELIM | Delimiter between start/end (default: newline) |
| `-j` | `--json` | | Output as JSON |
| `-n` | `--no-newline` | | Suppress trailing newline |
| `-v` | `--verbose` | | Print diagnostics to stderr |

### Examples

Expand "this week":

```console
$ td range "this week"
2025-01-13T00:00:00
2025-01-19T23:59:59

```

Expand "this month" with custom format:

```console
$ td range "this month" -f "%Y-%m-%d"
2025-01-01
2025-01-31

```

Custom delimiter:

```console
$ td range "this week" -d " / " -f "%Y-%m-%d"
2025-01-13 / 2025-01-19

```

JSON output:

```console
$ td range "this month" --json
{"delimiter":"/n","end":"2025-01-31T23:59:59","end_epoch":1738367999,"format":"%Y-%m-%dT%H:%M:%S","input":"this month","start":"2025-01-01T00:00:00","start_epoch":1735689600,"timezone":"UTC"}

```

Expand "today":

```console
$ td range "today"
2025-01-15T00:00:00
2025-01-15T23:59:59

```

---

## config -- Configuration management

Provides subcommands to inspect and manage the TARDIS configuration file.
The config file (TOML format) is created automatically on first run.

**Usage:** `td config SUBCOMMAND`

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `path` | Print the config file path |
| `show` | Display effective configuration |
| `edit` | Open config in `$EDITOR` (default: vi) |
| `presets` | List all format presets |

### Examples

Show config path:

```bash
td config path
# /home/user/.config/tardis/config.toml
```

Display effective configuration:

```bash
td config show
# format   = "%Y-%m-%dT%H:%M:%S"
# timezone = ""
```

List format presets:

```bash
td config presets
# NAME         FORMAT
# br           %d/%m/%Y
```

### Configuration file locations

- **Linux:** `~/.config/tardis/config.toml`
- **macOS:** `~/Library/Application Support/tardis/config.toml`
- **Windows:** `%APPDATA%\tardis\config.toml`

Override with `XDG_CONFIG_HOME` environment variable.

---

## completions -- Shell completion generation

Generates shell completion scripts for all `td` commands, subcommands, and
options.  Output is written to stdout; redirect to the appropriate file.

**Usage:** `td completions SHELL`

### Supported shells

`bash`, `zsh`, `fish`, `elvish`, `powershell`

### Examples

Install Bash completions:

```bash
mkdir -p ~/.local/share/bash-completion/completions
td completions bash > ~/.local/share/bash-completion/completions/td
```

Install Zsh completions:

```bash
td completions zsh > "${fpath[1]}/_td"
```

Install Fish completions:

```bash
mkdir -p ~/.config/fish/completions
td completions fish > ~/.config/fish/completions/td.fish
```
