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

## Features

- Parse expressions such as
  `next Monday at 09:00`, `in 2 hours`, `tomorrow 14:30`
- Output using custom formats (`--format`) or named presets
- Convert or force output time-zones (`--timezone`)
- Unix epoch input (`@1719244800`) and output (`--format epoch`)
- JSON output (`--json`) for scripting with `jq`
- Batch mode: pipe multiple expressions, one per line
- Shell completions for bash, zsh, fish, elvish, powershell
- Config management subcommand (`td config`)
- Commented `TOML` configuration with named presets
- Cross-platform (Linux, macOS, Windows)

---

## Installation

```bash
cargo install tardis-cli --locked
```

---

## Quick start

```bash
td "tomorrow 15:00"
# 2025-06-26T15:00:00+01:00

td "next Friday" --format br
# 26/06/2025

td "in 2 hours" --timezone Europe/London
# 2025-06-25T17:30:00+01:00

echo "next Monday at 09:00" | td
# 2025-06-30T09:00:00+01:00

td now -f "%H:%M"
# 15:30

td
# (shows current datetime -- no args defaults to "now")

td @1735689600 -f "%Y-%m-%d" -t UTC
# 2025-01-01

td "yesterday" -f epoch -t UTC --now "2025-06-26T15:30:00+01:00"
# 1719244800

td "today" --json -t UTC --now "2025-01-01T00:00:00Z"
# {"format":"%Y-%m-%dT%H:%M:%S","input":"today","output":"2025-01-01T00:00:00","timezone":"UTC"}

echo -e "today\ntomorrow" | td -f "%Y-%m-%d" -t UTC --now "2025-01-01T00:00:00Z"
# 2025-01-01
# 2025-01-02
```

---

## Options

```text
ARGS:
  [EXPRESSION]   Natural-language date, or @epoch. Defaults to "now".

FLAGS:
  -f, --format     chrono format string, preset name, "epoch", or "unix"
  -t, --timezone   output time-zone (IANA name)
  --now            reference datetime (RFC3339) for deterministic output
  -j, --json       output as JSON object
  -n, --no-newline suppress trailing newline
  --help           show CLI usage
  --version        show binary version

SUBCOMMANDS:
  config           Manage configuration (path, show, edit, presets)
  completions      Generate shell completions (bash, zsh, fish, elvish, powershell)
```

---

## Configuration

On first run, a config file is generated automatically:

| Platform                          | Path                              |
|----------------------------------|-----------------------------------|
| `$XDG_CONFIG_HOME` is set        | `$XDG_CONFIG_HOME/tardis/config.toml` |
| Linux (default)                  | `~/.config/tardis/config.toml`    |
| macOS                            | `~/Library/Application Support/tardis/config.toml` |
| Windows                          | `%APPDATA%\tardis\config.toml`  |

Precedence: **CLI flags > Environment variables > Config file**.

```toml
# config.toml
format = "%Y-%m-%dT%H:%M:%S%:z"
timezone = ""            # empty = use local OS time-zone

[formats]
taskline = "%d.%m.%Y %H:%M"
br       = "%d/%m/%Y"
iso      = "%Y-%m-%dT%H:%M:%S"
```

### Config subcommand

```bash
td config path      # show config file location
td config show      # dump effective config
td config edit      # open in $EDITOR
td config presets   # list available format presets
```

---

## Shell completions

```bash
# Bash
td completions bash > ~/.local/share/bash-completion/completions/td

# Zsh
td completions zsh > ~/.zfunc/_td

# Fish
td completions fish > ~/.config/fish/completions/td.fish
```

---

## Pipes & automation

```bash
# Schedule a reminder
tl t "Prepare slides" -d $(td "next Friday at 12:00" -f taskline)

# Embed without trailing newline
echo "Deadline: $(td 'next monday 9am' -f '%H:%M' -n)"

# Batch convert
cat dates.txt | td -f "%Y-%m-%d" -t UTC

# JSON + jq
td "tomorrow" --json | jq -r .output
```

---

## Environment variables

| Variable           | Purpose                                              |
|--------------------|------------------------------------------------------|
| `TARDIS_FORMAT`    | fallback format when `--format` is omitted           |
| `TARDIS_TIMEZONE`  | fallback time-zone when `--timezone` is omitted      |

---

## License

[MIT License](./LICENCE.md)

---

## Contributing

Bug reports and pull requests are welcome!
See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for guidelines.

---

## Trivia

The name **TARDIS** pays homage to the iconic, bigger-on-the-inside
time machine from *Doctor Who*. This CLI helps you navigate time too --
minus the wibbly-wobbly stuff.
