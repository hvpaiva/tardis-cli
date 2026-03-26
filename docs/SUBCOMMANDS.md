# Subcommand Reference

Complete reference for all `td` subcommands.

All examples use `--now` and `-t UTC` for deterministic, timezone-independent
output.  In normal usage you can omit these flags.

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
$ td diff "2025-01-01" "2025-03-15" --now 2025-01-01T00:00:00Z -t UTC
```

Difference in seconds:

```console
$ td diff "2025-01-01" "2025-06-01" --output seconds --now 2025-01-01T00:00:00Z -t UTC
```

ISO 8601 duration:

```console
$ td diff "2025-01-01" "2025-03-15" --output iso --now 2025-01-01T00:00:00Z -t UTC
```

JSON output:

```console
$ td diff yesterday tomorrow --json --now 2025-06-24T12:00:00Z -t UTC
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
$ td convert "2025-06-24" --to epoch --now 2025-06-24T00:00:00Z -t UTC
```

Convert from ISO to custom format:

```console
$ td convert "2025-06-24T09:00:00Z" --to "%d/%m/%Y %H:%M" -t UTC
```

Convert a bare epoch timestamp:

```console
$ td convert 1719244800 --to "%Y-%m-%d" -t UTC
```

With explicit input format:

```console
$ td convert "24/06/2025" --from "%d/%m/%Y" --to iso8601 -t UTC
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
$ td tz "now" --to UTC --now 2025-06-24T09:00:00Z
```

Convert from US Eastern to Sao Paulo:

```console
$ td tz "2025-06-24T12:00:00" --from America/New_York --to America/Sao_Paulo
```

Convert to Tokyo time:

```console
$ td tz "tomorrow at 9am" --to Asia/Tokyo --now 2025-06-24T09:00:00Z
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
$ td info --now 2025-06-24T09:00:00Z -t UTC
```

Info for a specific date:

```console
$ td info "2025-12-25" --now 2025-06-24T09:00:00Z -t UTC
```

JSON output with all metadata:

```console
$ td info "2025-01-01" --json --now 2025-01-01T00:00:00Z -t UTC
```

Info for a relative expression:

```console
$ td info "3 days ago" --now 2025-06-24T09:00:00Z -t UTC
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
$ td range "this week" --now 2025-06-24T09:00:00Z -t UTC
```

Expand "this month" with custom format:

```console
$ td range "this month" -f "%Y-%m-%d" --now 2025-06-24T09:00:00Z -t UTC
```

Custom delimiter:

```console
$ td range "this week" -d " / " -f "%Y-%m-%d" --now 2025-06-24T09:00:00Z -t UTC
```

JSON output:

```console
$ td range "this month" --json --now 2025-06-24T09:00:00Z -t UTC
```

Expand "today":

```console
$ td range "today" --now 2025-06-24T09:00:00Z -t UTC
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

```console
$ td config path
```

Display effective configuration:

```console
$ td config show
```

List format presets:

```console
$ td config presets
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

```console
$ td completions bash > ~/.local/share/bash-completion/completions/td
```

Install Zsh completions:

```console
$ td completions zsh > "${fpath[1]}/_td"
```

Install Fish completions:

```console
$ td completions fish > ~/.config/fish/completions/td.fish
```
