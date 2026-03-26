# Format Specifiers

Complete reference of all strftime/strptime format specifiers supported
by `td`.

Used with `td -f "<pattern>"` or `td convert --to "<pattern>"`.

All examples use `--now "2025-03-15T14:30:45Z" -t UTC` for deterministic
output.

---

## Date Specifiers

| Specifier | Description                        | Example Output |
|-----------|------------------------------------|----------------|
| `%Y`      | Four-digit year                    | `2025`         |
| `%C`      | Century (first two digits of year) | `20`           |
| `%y`      | Two-digit year (00-99)             | `25`           |
| `%m`      | Month (01-12)                      | `03`           |
| `%b`      | Abbreviated month name             | `Mar`          |
| `%h`      | Abbreviated month name (alias)     | `Mar`          |
| `%B`      | Full month name                    | `March`        |
| `%d`      | Day of month (01-31)               | `15`           |
| `%e`      | Day of month, space-padded ( 1-31) | `15`           |
| `%j`      | Day of year (001-366)              | `074`          |
| `%u`      | Day of week (1=Monday, 7=Sunday)   | `6`            |
| `%w`      | Day of week (0=Sunday, 6=Saturday) | `6`            |
| `%a`      | Abbreviated weekday name           | `Sat`          |
| `%A`      | Full weekday name                  | `Saturday`     |
| `%U`      | Week number (Sunday start, 00-53)  | `10`           |
| `%W`      | Week number (Monday start, 00-53)  | `10`           |
| `%G`      | ISO 8601 week-based year           | `2025`         |
| `%g`      | ISO 8601 week-based year (2 digit) | `25`           |
| `%V`      | ISO 8601 week number (01-53)       | `11`           |

## Time Specifiers

| Specifier | Description                              | Example Output |
|-----------|------------------------------------------|----------------|
| `%H`      | Hour, 24-hour clock (00-23)              | `14`           |
| `%k`      | Hour, 24-hour clock, space-padded        | `14`           |
| `%I`      | Hour, 12-hour clock (01-12)              | `02`           |
| `%l`      | Hour, 12-hour clock, space-padded        | ` 2`           |
| `%M`      | Minute (00-59)                           | `30`           |
| `%S`      | Second (00-60)                           | `45`           |
| `%N`      | Nanoseconds (000000000-999999999)        | `000000000`    |
| `%f`      | Nanoseconds (alias for `%N` in parsing)  | `0`            |
| `%p`      | AM/PM (uppercase)                        | `PM`           |
| `%P`      | am/pm (lowercase)                        | `pm`           |
| `%r`      | 12-hour time (`%I:%M:%S %p`)             | `2:30:45 PM`   |
| `%R`      | 24-hour time (`%H:%M`)                   | `14:30`        |
| `%T`      | 24-hour time (`%H:%M:%S`)               | `14:30:45`     |
| `%X`      | Time representation (`%H:%M:%S`)         | `14:30:45`     |
| `%s`      | Unix epoch (seconds since 1970-01-01)    | `1742049045`   |

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
| `%D`      | Date (`%m/%d/%y`)        | `03/15/25`                   |
| `%F`      | ISO 8601 date (`%Y-%m-%d`) | `2025-03-15`              |
| `%x`      | Date representation      | `2025 M03 15`                |
| `%c`      | Date and time            | `2025 M03 15, Sat 14:30:45`  |

## Padding Modifiers

Place a modifier between `%` and the specifier letter to control padding.

| Modifier | Description          | Example          | Output  |
|----------|----------------------|------------------|---------|
| `%-`     | No padding           | `%-d` on day 5   | `5`     |
| `%0`     | Zero padding         | `%0d` on day 5   | `05`    |
| `%_`     | Space padding        | `%_d` on day 5   | ` 5`    |

```console
$ td now --now "2025-03-05T14:30:45Z" -t UTC -f "%-d"
5

$ td now --now "2025-03-05T14:30:45Z" -t UTC -f "%_d"
 5

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
| `epoch` / `unix`   | Unix timestamp (seconds)          | `1742049045`                       |
| `iso8601` / `iso`  | `%Y-%m-%dT%H:%M:%S%:z`           | `2025-03-15T14:30:45+00:00`        |
| `rfc3339`          | `%Y-%m-%dT%H:%M:%S%:z`           | `2025-03-15T14:30:45+00:00`        |
| `rfc2822`          | `%a, %d %b %Y %H:%M:%S %z`      | `Sat, 15 Mar 2025 14:30:45 +0000`  |

```console
$ td now --now "2025-03-15T14:30:45Z" -t UTC -f epoch
1742049045

$ td convert "2025-03-15T14:30:45Z" --to iso8601 -t UTC
2025-03-15T14:30:45+00:00

$ td convert "2025-03-15T14:30:45Z" --to rfc2822 -t UTC
Sat, 15 Mar 2025 14:30:45 +0000

$ td convert "2025-03-15T14:30:45Z" --to rfc3339 -t UTC
2025-03-15T14:30:45+00:00

```

**Note:** `iso8601`, `rfc3339`, and `rfc2822` names resolve in `convert`,
`range`, and `tz` subcommands. The default `td` command with `-f` supports
`epoch`/`unix` as special names and treats all other values as strftime
patterns or config preset names.

## Custom Examples

```console
$ td now --now "2025-03-15T14:30:45Z" -t UTC -f "%Y-%m-%d"
2025-03-15

$ td now --now "2025-03-15T14:30:45Z" -t UTC -f "%A, %B %d"
Saturday, March 15

$ td now --now "2025-03-15T14:30:45Z" -t UTC -f "%H:%M"
14:30

$ td now --now "2025-03-15T14:30:45Z" -t UTC -f "%d/%m/%Y"
15/03/2025

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
