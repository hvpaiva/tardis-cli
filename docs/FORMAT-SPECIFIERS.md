# Format Specifiers

Complete reference of all strftime/strptime format specifiers supported
by `td`.

Used with `td -f "<pattern>"` or `td convert --to "<pattern>"`.

All examples run under a test harness that sets `TARDIS_NOW=2025-01-15T10:30:00Z`
and `TZ=UTC` for deterministic output.

---

## Date Specifiers

| Specifier | Description                        | Example Output |
|-----------|------------------------------------|----------------|
| `%Y`      | Four-digit year                    | `2025`         |
| `%C`      | Century (first two digits of year) | `20`           |
| `%y`      | Two-digit year (00-99)             | `25`           |
| `%m`      | Month (01-12)                      | `01`           |
| `%b`      | Abbreviated month name             | `Jan`          |
| `%h`      | Abbreviated month name (alias)     | `Jan`          |
| `%B`      | Full month name                    | `January`      |
| `%d`      | Day of month (01-31)               | `15`           |
| `%e`      | Day of month, space-padded ( 1-31) | `15`           |
| `%j`      | Day of year (001-366)              | `015`          |
| `%u`      | Day of week (1=Monday, 7=Sunday)   | `3`            |
| `%w`      | Day of week (0=Sunday, 6=Saturday) | `3`            |
| `%a`      | Abbreviated weekday name           | `Wed`          |
| `%A`      | Full weekday name                  | `Wednesday`    |
| `%U`      | Week number (Sunday start, 00-53)  | `02`           |
| `%W`      | Week number (Monday start, 00-53)  | `02`           |
| `%G`      | ISO 8601 week-based year           | `2025`         |
| `%g`      | ISO 8601 week-based year (2 digit) | `25`           |
| `%V`      | ISO 8601 week number (01-53)       | `03`           |

## Time Specifiers

| Specifier | Description                              | Example Output |
|-----------|------------------------------------------|----------------|
| `%H`      | Hour, 24-hour clock (00-23)              | `10`           |
| `%k`      | Hour, 24-hour clock, space-padded        | `10`           |
| `%I`      | Hour, 12-hour clock (01-12)              | `10`           |
| `%l`      | Hour, 12-hour clock, space-padded        | `10`           |
| `%M`      | Minute (00-59)                           | `30`           |
| `%S`      | Second (00-60)                           | `00`           |
| `%N`      | Nanoseconds (000000000-999999999)        | `000000000`    |
| `%f`      | Nanoseconds (alias for `%N` in parsing)  | `0`            |
| `%p`      | AM/PM (uppercase)                        | `AM`           |
| `%P`      | am/pm (lowercase)                        | `am`           |
| `%r`      | 12-hour time (`%I:%M:%S %p`)             | `10:30:00 AM`  |
| `%R`      | 24-hour time (`%H:%M`)                   | `10:30`        |
| `%T`      | 24-hour time (`%H:%M:%S`)               | `10:30:00`     |
| `%X`      | Time representation (`%H:%M:%S`)         | `10:30:00`     |
| `%s`      | Unix epoch (seconds since 1970-01-01)    | `1736937000`   |

## Timezone Specifiers

| Specifier | Description                  | Example Output |
|-----------|------------------------------|----------------|
| `%Z`      | Timezone abbreviation        | `UTC`          |
| `%z`      | UTC offset (+HHMM)           | `+0000`        |
| `%:z`     | UTC offset (+HH:MM)          | `+00:00`       |
| `%::z`    | UTC offset (+HH:MM:SS)       | `+00:00:00`    |

## Combined / Shortcut Specifiers

| Specifier | Description              | Example Output               |
|-----------|--------------------------|------------------------------|
| `%D`      | Date (`%m/%d/%y`)        | `01/15/25`                   |
| `%F`      | ISO 8601 date (`%Y-%m-%d`) | `2025-01-15`              |
| `%x`      | Date representation      | `2025 M01 15`                |
| `%c`      | Date and time            | `2025 M01 15, Wed 10:30:00`  |

## Padding Modifiers

Place a modifier between `%` and the specifier letter to control padding.

| Modifier | Description          | Example          | Output  |
|----------|----------------------|------------------|---------|
| `%-`     | No padding           | `%-d` on day 5   | `5`     |
| `%0`     | Zero padding         | `%0d` on day 5   | `05`    |
| `%_`     | Space padding        | `%_d` on day 5   | ` 5`    |

```console
$ td now -f "%-d"
15

$ td now -f "%_d"
15

```

## Literal Characters

| Specifier | Description     |
|-----------|-----------------|
| `%%`      | Literal `%`     |
| `%n`      | Newline         |
| `%t`      | Tab             |

## Built-in Format Names

These named formats can be used with `td convert --to <name>` and
`td range -f <name>` instead of spelling out the full strftime pattern.
The `epoch` and `unix` names also work with `td -f`.

| Name               | Pattern                           | Example Output                     |
|--------------------|-----------------------------------|------------------------------------|
| `epoch` / `unix`   | Unix timestamp (seconds)          | `1736937000`                       |
| `iso8601` / `iso`  | `%Y-%m-%dT%H:%M:%S%:z`           | `2025-01-15T10:30:00+00:00`        |
| `rfc3339`          | `%Y-%m-%dT%H:%M:%S%:z`           | `2025-01-15T10:30:00+00:00`        |
| `rfc2822`          | `%a, %d %b %Y %H:%M:%S %z`      | `Wed, 15 Jan 2025 10:30:00 +0000`  |

```console
$ td now -f epoch
1736937000

$ td convert "2025-03-15T14:30:45Z" --to iso8601
2025-03-15T14:30:45+00:00

$ td convert "2025-03-15T14:30:45Z" --to rfc2822
Sat, 15 Mar 2025 14:30:45 +0000

$ td convert "2025-03-15T14:30:45Z" --to rfc3339
2025-03-15T14:30:45+00:00

```

**Note:** `iso8601`, `rfc3339`, and `rfc2822` names resolve in `convert`,
`range`, and `tz` subcommands. The default `td` command with `-f` supports
`epoch`/`unix` as special names and treats all other values as strftime
patterns or config preset names.

## Custom Examples

```console
$ td now -f "%Y-%m-%d"
2025-01-15

$ td now -f "%A, %B %d"
Wednesday, January 15

$ td now -f "%H:%M"
10:30

$ td now -f "%d/%m/%Y"
15/01/2025

```

---

## User-Defined Presets

In addition to the built-in names, you can define your own named formats
in the configuration file under `[formats]`. See the
[Configuration](CONFIGURATION.md) reference.

```toml
[formats]
br    = "%d/%m/%Y"
us    = "%m/%d/%Y"
time  = "%H:%M:%S"
```

Then use them by name:

```sh
$ td now -f br
15/03/2025
```
