# Configuration

`td` uses a layered configuration system with three-tier precedence.

---

## Precedence

**CLI flags > Environment variables > Config file**

When a value is set at multiple levels, the most specific source wins.
For example, `td -f "%Y" ...` overrides both `TARDIS_FORMAT` and the
`format` field in `config.toml`.

---

## Config File

### Location

`$XDG_CONFIG_HOME/tardis/config.toml`

Platform defaults when `XDG_CONFIG_HOME` is not set:

| Platform | Path                                        |
|----------|---------------------------------------------|
| Linux    | `~/.config/tardis/config.toml`              |
| macOS    | `~/Library/Application Support/tardis/config.toml` |
| Windows  | `%APPDATA%\tardis\config.toml`              |

The file is created automatically on first run with commented defaults.

### Fields

| Field      | Type   | Default                  | Description                                        |
|------------|--------|--------------------------|----------------------------------------------------|
| `format`   | string | `"%Y-%m-%dT%H:%M:%S"`   | Default output format (strftime pattern or preset)  |
| `timezone` | string | `""`                     | Default IANA timezone. Empty = system local timezone |

### Format Presets

Define named formats under `[formats]` for reuse with `td -f <name>`.

```toml
# config.toml

format = "%Y-%m-%dT%H:%M:%S"
timezone = ""

[formats]
br    = "%d/%m/%Y"
us    = "%m/%d/%Y"
short = "%d/%m"
hour  = "%H:%M"
iso   = "%Y-%m-%dT%H:%M:%S%:z"
time  = "%H:%M:%S"
```

Usage:

```console
$ td now --now "2025-01-15T10:30:00Z" -t UTC -f br
15/01/2025

```

See the [Format Specifiers](FORMAT-SPECIFIERS.md) reference for all
available strftime patterns and built-in format names.

---

## Environment Variables

| Variable           | Overrides          | Description                              |
|--------------------|--------------------|------------------------------------------|
| `TARDIS_FORMAT`    | `format` in config | Default output format (strftime or preset)|
| `TARDIS_TIMEZONE`  | `timezone` in config | Default IANA timezone                  |
| `XDG_CONFIG_HOME`  | Config directory   | Override config directory base path       |
| `EDITOR`           | (none)             | Used by `td config edit` to open editor   |
| `NO_COLOR`         | (none)             | Disables all ANSI color output            |

Environment variables take precedence over config file values but are
overridden by CLI flags.

Empty environment variables are ignored (treated as unset).

---

## Config Subcommands

Manage the configuration file without editing it manually.

### `td config path`

Print the absolute path to the configuration file.

```console
$ td config path
/home/user/.config/tardis/config.toml

```

### `td config show`

Display the effective configuration (merged from all sources).

### `td config edit`

Open the configuration file in `$EDITOR`. Creates the file first if it
does not exist.

### `td config presets`

List all named format presets defined in the `[formats]` table.

---

## Example Workflow

1. **First run** -- `td` creates `config.toml` with sensible defaults:

   ```bash
   td now
   # Config file created at ~/.config/tardis/config.toml
   ```

2. **Customize defaults** -- set your preferred format and timezone:

   ```bash
   td config edit
   ```

   ```toml
   format = "%d/%m/%Y %H:%M"
   timezone = "America/Sao_Paulo"

   [formats]
   br   = "%d/%m/%Y"
   iso  = "%Y-%m-%dT%H:%M:%S%:z"
   time = "%H:%M:%S"
   ```

3. **Override per-invocation** -- CLI flags always win:

   ```bash
   td now -f "%Y" -t UTC
   # Uses %Y format and UTC timezone regardless of config
   ```

4. **Override via environment** -- useful in scripts:

   ```bash
   export TARDIS_FORMAT="%Y-%m-%d"
   export TARDIS_TIMEZONE="UTC"
   td now
   # Uses env var values (overriding config, but not CLI flags)
   ```
