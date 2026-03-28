# Expression Reference

Complete reference of all date/time expressions supported by `td`.

All examples run under a test harness that sets `TARDIS_NOW=2025-01-15T10:30:00Z`
and `TZ=UTC` for deterministic, reproducible output.

---

## Named Dates

Simple keywords that resolve to a specific point in time.

| Expression   | Description                |
|--------------|----------------------------|
| `now`        | Current date and time      |
| `today`      | Start of the current day   |
| `tomorrow`   | Start of the next day      |
| `yesterday`  | Start of the previous day  |
| `overmorrow` | Start of the day after tomorrow |
| `ereyesterday` | Start of the day before yesterday |

```console
$ td now
2025-01-15T10:30:00

$ td today
2025-01-15T00:00:00

$ td tomorrow
2025-01-16T00:00:00

$ td yesterday
2025-01-14T00:00:00

$ td overmorrow
2025-01-17T00:00:00

$ td ereyesterday
2025-01-13T00:00:00

```

## Weekday References

Navigate to a specific weekday relative to the current date. Full names
and three-letter abbreviations are both accepted (e.g., `monday` or `mon`).

| Expression         | Description                          |
|--------------------|--------------------------------------|
| `next <weekday>`   | Next occurrence of the named weekday |
| `last <weekday>`   | Previous occurrence of the weekday   |
| `this <weekday>`   | Current week's occurrence            |
| `<weekday>`        | Bare weekday (same as `next`)        |

```console
$ td "next monday"
2025-01-20T00:00:00

$ td "last friday"
2025-01-10T00:00:00

$ td "this wednesday"
2025-01-15T00:00:00

$ td friday
2025-01-17T00:00:00

```

## Relative Offsets

Express a duration relative to the current moment. Supports all seven
temporal units (year, month, week, day, hour, minute, second) with
singular, plural, and abbreviated forms.

| Expression           | Direction | Description                     |
|----------------------|-----------|---------------------------------|
| `in N <unit>`        | Future    | N units from now                |
| `N <unit> ago`       | Past      | N units before now              |
| `a <unit> ago`       | Past      | One unit before now             |
| `last week`          | Past      | Start of previous week (Monday 00:00) |
| `last month`         | Past      | Start of previous month (1st 00:00)   |
| `last year`          | Past      | Start of previous year (Jan 1 00:00)  |
| `next year`          | Future    | Start of next year (range)      |

**Accepted unit forms:**

| Unit    | Accepted spellings                            |
|---------|-----------------------------------------------|
| Year    | `year`, `years`, `y`, `yr`, `yrs`             |
| Month   | `month`, `months`, `mo`, `mos`                |
| Week    | `week`, `weeks`, `w`, `wk`, `wks`             |
| Day     | `day`, `days`, `d`                             |
| Hour    | `hour`, `hours`, `h`, `hr`, `hrs`             |
| Minute  | `minute`, `minutes`, `min`, `mins`            |
| Second  | `second`, `seconds`, `sec`, `secs`            |

```console
$ td "in 3 days"
2025-01-18T10:30:00

$ td "2 hours ago"
2025-01-15T08:30:00

$ td "a week ago"
2025-01-08T10:30:00

$ td "last month"
2024-12-01T00:00:00

```

## Arithmetic

Combine a base expression with `+` or `-` followed by a duration.
Chains left-to-right: `base + A - B` applies A first, then subtracts B.

```console
$ td "tomorrow + 3 hours"
2025-01-16T03:00:00

$ td "next friday - 1 day"
2025-01-16T00:00:00

$ td "eod + 1h"
2025-01-16T00:59:59

$ td "next friday at 15:00 + 2 hours"
2025-01-17T17:00:00

$ td "now + 1h30"
2025-01-15T12:00:00

```

## Operator-Prefixed Offsets

Start an expression with `+` or `-` to apply a duration to the implicit
"now" reference. Use `--` before the expression to prevent the shell
from interpreting `-` as a flag.

```console
$ td -- "-1d"
2025-01-14T10:30:00

```

**Note:** Place flags before `--` so they are still parsed. The `+`
prefix expressions conflict with shell argument parsing when combined
with other flags. Use the arithmetic form instead: `td "now + 3h"`.

## Absolute Dates

Specify an exact calendar date. If no year is given, the current year is
assumed. ISO 8601 format (`YYYY-MM-DD`) and natural-language forms are
both accepted.

```console
$ td "2025-01-15"
2025-01-15T00:00:00

$ td "15 March 2025"
2025-03-15T00:00:00

```

## Absolute Date-Times

Append a time suffix to any date expression. The `at` keyword is
optional. Hours can use `Nh` notation or `HH:MM[:SS]` format.

```console
$ td "tomorrow at 15:00"
2025-01-16T15:00:00

$ td "today 18h"
2025-01-15T18:00:00

$ td "next monday 9:30"
2025-01-20T09:30:00

```

## Time Suffixes

Time expressions require a date context. All three notations are equivalent
and can follow any date keyword or `at`:

- `15:30` — colon notation (HH:MM)
- `15h30` — hour notation (Nh[MM])
- `3:30pm` — 12-hour clock with AM/PM

```console
$ td "tomorrow 15:30"
2025-01-16T15:30:00

$ td "tomorrow 15h30"
2025-01-16T15:30:00

$ td "tomorrow 3:30pm"
2025-01-16T15:30:00

$ td "today at 9:00"
2025-01-15T09:00:00

$ td "today 9h"
2025-01-15T09:00:00

$ td "today 9am"
2025-01-15T09:00:00

```

## AM/PM Time

Use 12-hour clock notation with `am` or `pm`. Supports bare hours (`3pm`),
hours with minutes (`3:30pm`), and full `HH:MM:SS` (`3:30:45pm`). A space
before `am`/`pm` is optional. Requires a date context.

```console
$ td "tomorrow at 3pm"
2025-01-16T15:00:00

$ td "tomorrow at 3:30pm"
2025-01-16T15:30:00

$ td "tomorrow 12am"
2025-01-16T00:00:00

$ td "tomorrow 12pm"
2025-01-16T12:00:00

$ td "next friday at 3pm"
2025-01-17T15:00:00

```

## Same Time

Use `at same time` after a date expression to preserve the current time
of day from the `--now` reference (or the system clock when `--now` is not
set).

```console
$ td "tomorrow at same time"
2025-01-16T10:30:00

$ td "next friday at same time"
2025-01-17T10:30:00

```

## Epoch Timestamps

Prefix a Unix timestamp with `@`. Precision is auto-detected from the
magnitude of the number, or can be specified explicitly with a suffix.

| Suffix | Precision    | Example                    |
|--------|--------------|----------------------------|
| (none) | Auto-detect  | `@1735689600`              |
| `s`    | Seconds      | `@1735689600s`             |
| `ms`   | Milliseconds | `@1735689600000ms`         |
| `us`   | Microseconds | `@1735689600000000us`      |
| `ns`   | Nanoseconds  | `@1735689600000000000ns`   |

```console
$ td "@1735689600"
2025-01-01T00:00:00

$ td "@1735689600s"
2025-01-01T00:00:00

$ td "@1735689600000ms"
2025-01-01T00:00:00

$ td "@1735689600000000us"
2025-01-01T00:00:00

$ td "@1735689600000000000ns"
2025-01-01T00:00:00

```

**Auto-detection thresholds:** values above ~1e12 are treated as
milliseconds, above ~1e15 as microseconds, and above ~1e18 as
nanoseconds.

## Periods (Range Expressions)

Period expressions describe a span of time. When used with the default
`td` command, they resolve to the **start** of the period. Use
`td range` to get both start and end.

| Expression    | Default `td`            | `td range` start / end             |
|---------------|-------------------------|------------------------------------|
| `this week`   | Start of current week   | Monday 00:00 / Sunday 23:59:59     |
| `this month`  | Start of current month  | 1st 00:00 / last day 23:59:59      |
| `this year`   | Start of current year   | Jan 1 00:00 / Dec 31 23:59:59      |
| `next week`   | Start of next week      | Next Mon 00:00 / Next Sun 23:59:59 |
| `next month`  | Start of next month     | 1st 00:00 / last day 23:59:59      |
| `next year`   | Start of next year      | Jan 1 00:00 / Dec 31 23:59:59      |

```console
$ td "this week"
2025-01-13T00:00:00

$ td "this month"
2025-01-01T00:00:00

$ td "this year"
2025-01-01T00:00:00

```

With `td range`:

```console
$ td range "this week"
2025-01-13T00:00:00
2025-01-19T23:59:59

$ td range "next year"
2026-01-01T00:00:00
2026-12-31T23:59:59

```

**Note:** `last week`, `last month`, and `last year` resolve to the
start of the previous period (Monday 00:00 for weeks, 1st 00:00 for
months, Jan 1 00:00 for years) — consistent with `this` and `next`.
For relative offsets, use `-1w`, `-1mo`, or `1 week ago`.

## Boundary Keywords (TaskWarrior-Style)

Shortcuts for the start or end of common periods. Inspired by TaskWarrior
date expressions.

### Current Period

| Keyword | Description               |
|---------|---------------------------|
| `sod`   | Start of day (00:00:00)   |
| `eod`   | End of day (23:59:59)     |
| `sow`   | Start of week (Monday)    |
| `eow`   | End of week (Sunday)      |
| `soww`  | Start of work week (Mon)  |
| `eoww`  | End of work week (Fri)    |
| `som`   | Start of month            |
| `eom`   | End of month              |
| `soq`   | Start of quarter          |
| `eoq`   | End of quarter            |
| `soy`   | Start of year             |
| `eoy`   | End of year               |

### Previous Period

| Keyword | Description                     |
|---------|---------------------------------|
| `sopd`  | Start of previous day           |
| `eopd`  | End of previous day             |
| `sopw`  | Start of previous week          |
| `eopw`  | End of previous week            |
| `sopm`  | Start of previous month         |
| `eopm`  | End of previous month           |
| `sopq`  | Start of previous quarter       |
| `eopq`  | End of previous quarter         |
| `sopy`  | Start of previous year          |
| `eopy`  | End of previous year            |

### Next Period

| Keyword | Description                     |
|---------|---------------------------------|
| `sond`  | Start of next day               |
| `eond`  | End of next day                 |
| `sonw`  | Start of next week              |
| `eonw`  | End of next week                |
| `sonm`  | Start of next month             |
| `eonm`  | End of next month               |
| `sonq`  | Start of next quarter           |
| `eonq`  | End of next quarter             |
| `sony`  | Start of next year              |
| `eony`  | End of next year                |

```console
$ td eod
2025-01-15T23:59:59

$ td sod
2025-01-15T00:00:00

$ td sow
2025-01-13T00:00:00

$ td eom
2025-01-31T23:59:59

$ td soy
2025-01-01T00:00:00

$ td eoy
2025-12-31T23:59:59

$ td sopw
2025-01-06T00:00:00

$ td eonm
2025-02-28T23:59:59

```

## Compound Durations

Combine multiple units in a single offset. The keyword `and` is optional
between components; commas are also accepted as separators.

```console
$ td "in 1h30min"
2025-01-15T12:00:00

$ td "in 2d3h"
2025-01-17T13:30:00

```

The `NhMM` shorthand (e.g., `1h30`) infers the trailing number as
minutes when no unit suffix follows.

## Verbal Arithmetic

Use `after` or `before` to apply a duration relative to a named
expression.

```console
$ td "3 hours after tomorrow"
2025-01-16T03:00:00

$ td "2 days before friday"
2025-01-15T00:00:00

```

---

## Composability

Expressions compose freely. Arithmetic tails (`+ duration`, `- duration`)
can be appended to any primary expression, including boundary keywords,
absolute dates, weekday references, and epoch timestamps.

```console
$ td "eod + 1h"
2025-01-16T00:59:59

$ td "next friday at 15:00 + 2 hours"
2025-01-17T17:00:00

$ td "tomorrow + 3 hours"
2025-01-16T03:00:00

$ td "@1735689600 + 1d"
2025-01-02T00:00:00

```

Boundary keywords combine with arithmetic to express precise offsets:
"one hour after end of day" is simply `eod + 1h`.

---

## Input Methods

Expressions can be provided in three ways:

1. **Positional argument:** `td "next friday"`
2. **Standard input (pipe):** `echo "next friday" | td`
3. **Batch mode:** pipe multiple expressions, one per line. Use
   `--skip-errors` to continue past failures.

When no input is given in an interactive terminal, `td` defaults to
`now`.
