# TARDIS CLI (`td`)

![Crates.io](https://img.shields.io/crates/v/tardis-cli)
![Docs.rs](https://img.shields.io/docsrs/tardis-cli)
![CI](https://github.com/hvpaiva/tardis-cli/actions/workflows/ci.yml/badge.svg)
[![codecov](https://codecov.io/gh/hvpaiva/tardis-cli/graph/badge.svg)](https://codecov.io/gh/hvpaiva/tardis-cli)
![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)

> Translates natural-language time expressions into formatted datetimes.

```bash
td "next friday at 3pm" --now 2025-01-15T10:30:00Z -t UTC
# 2025-01-17T15:00:00

td "2 days ago" -f "%Y-%m-%d" --now 2025-01-15T10:30:00Z -t UTC
# 2025-01-13
```

## Install

```bash
cargo install tardis-cli --locked
```

From source:

```bash
git clone https://github.com/hvpaiva/tardis-cli.git
cd tardis-cli && cargo install --path . --locked
```

Shell completions:

```bash
td completions bash > ~/.local/share/bash-completion/completions/td
td completions zsh  > "${fpath[1]}/_td"
td completions fish > ~/.config/fish/completions/td.fish
```

## Quick Start

```bash
td "tomorrow 15:00" --now 2025-01-15T10:30:00Z -t UTC
# 2025-01-16T15:00:00

td "in 2 hours" -f "%H:%M" --now 2025-01-15T10:30:00Z -t UTC
# 12:30

td @1735689600 -f "%Y-%m-%d" -t UTC
# 2025-01-01

td "now + 3h" --now 2025-01-15T10:30:00Z -t UTC
# 2025-01-15T13:30:00

td eod --now 2025-01-15T10:30:00Z -t UTC
# 2025-01-15T23:59:59

td "today" --json -n --now 2025-01-15T00:00:00Z -t UTC
# {"epoch":1736899200,"format":"%Y-%m-%dT%H:%M:%S","input":"today","output":"2025-01-15T00:00:00","timezone":"UTC"}
```

## Features

| Feature | Example | Documentation |
|---------|---------|---------------|
| Natural language | `td "next friday at 3pm"` | [Expression Reference](docs/EXPRESSIONS.md) |
| Date arithmetic | `td "tomorrow + 3 hours"` | [Expression Reference](docs/EXPRESSIONS.md) |
| Format control | `td "now" -f "%Y-%m-%d"` | [Format Specifiers](docs/FORMAT-SPECIFIERS.md) |
| Timezone conversion | `td tz "3pm" --to UTC` | [Subcommands](docs/SUBCOMMANDS.md) |
| Date diff | `td diff "jan 1" "mar 15"` | [Subcommands](docs/SUBCOMMANDS.md) |
| JSON output | `td "now" --json` | [Subcommands](docs/SUBCOMMANDS.md) |
| Configuration | `td config show` | [Configuration](docs/CONFIGURATION.md) |
| Boundaries | `td eod`, `td sow` | [Expression Reference](docs/EXPRESSIONS.md) |
| Epoch input | `td @1735689600` | [Expression Reference](docs/EXPRESSIONS.md) |
| Batch mode | `cat dates.txt \| td` | [Expression Reference](docs/EXPRESSIONS.md) |

## Subcommands

| Command | Description |
|---------|-------------|
| `td diff` | Compute the duration between two dates |
| `td convert` | Re-format a date into a target format |
| `td tz` | Convert a datetime between timezones |
| `td info` | Display calendar metadata (week, quarter, Julian day) |
| `td range` | Expand a period expression into start/end datetimes |
| `td config` | Inspect and manage the configuration file |
| `td completions` | Generate shell completion scripts |

See the [Subcommand Reference](docs/SUBCOMMANDS.md) for full usage and examples.

## Documentation

- [Expression Reference](docs/EXPRESSIONS.md) -- All supported date/time expressions
- [Subcommand Reference](docs/SUBCOMMANDS.md) -- Complete subcommand documentation
- [Format Specifiers](docs/FORMAT-SPECIFIERS.md) -- strftime/strptime format reference
- [Configuration](docs/CONFIGURATION.md) -- Config file, environment variables, precedence

## vs. Other Tools

| Tool | Natural language | Subcommands | JSON | Config presets |
|------|:----------------:|:-----------:|:----:|:--------------:|
| `td` | Yes | diff, tz, convert, info, range | Yes | Yes |
| `date` | No | No | No | No |
| `dateutils` | No | Yes | No | No |
| `dateadd` | No | No | No | No |

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## License

[MIT](./LICENCE.md)
