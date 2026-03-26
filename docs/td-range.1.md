% TD-RANGE(1) TARDIS Manual
% TARDIS Contributors
% 2026

# NAME

td-range - expand a date expression into a start/end range

# SYNOPSIS

**td range** *EXPRESSION* [*OPTIONS*]

# DESCRIPTION

**td range** interprets a date expression as a time period and expands it
into a start and end datetime.  The output contains two lines: the start
datetime followed by the end datetime, separated by the delimiter (newline
by default).

**Granularity expansion:** the range end is determined by the smallest
unspecified time unit.  For example:

- "this week" expands to Monday 00:00 through Sunday 23:59:59.
- "today" expands to 00:00:00 through 23:59:59 of the current day.
- "this month" expands to the first day through the last day of the month.
- "tomorrow at 3pm" expands from 15:00:00 through 15:59:59 (hour
  granularity, since minutes and seconds were unspecified).

When the same expression is used with the default **td** command (without
the **range** subcommand), only the start-of-period instant is returned.

# OPTIONS

**-f**, **-\-format** *FMT*
:   Output format for both start and end (strftime pattern or preset name).
    See **docs/FORMAT-SPECIFIERS.md** for the full reference.

**-t**, **-\-timezone** *TZ*
:   IANA/Olson timezone to apply (e.g. "UTC", "America/Sao_Paulo").

**-\-now** *DATETIME*
:   Override the current time (RFC 3339).

**-d**, **-\-delimiter** *DELIM*
:   Delimiter between start and end in plain-text output.  Defaults to a
    newline character.  Common values: " / ", " -- ", ",".

**-j**, **-\-json**
:   Output as a JSON object with *start*, *end*, and *delimiter* fields.

**-n**, **-\-no-newline**
:   Suppress the trailing newline.

**-v**, **-\-verbose**
:   Print verbose diagnostics to stderr.

**-h**, **-\-help**
:   Print help information.

# EXAMPLES

Expand "this week" to start/end:

    td range "this week" --now 2025-06-24T09:00:00Z -t UTC

Expand "this month" with a custom format:

    td range "this month" -f "%Y-%m-%d" --now 2025-06-24T09:00:00Z -t UTC

Expand "today" to a full day range:

    td range "today" --now 2025-06-24T09:00:00Z -t UTC

Hour-level granularity (tomorrow at 3pm):

    td range "tomorrow at 3pm" --now 2025-06-24T09:00:00Z -t UTC

Custom delimiter:

    td range "this week" -d " / " -f "%Y-%m-%d" --now 2025-06-24T09:00:00Z -t UTC

JSON output:

    td range "this month" --json --now 2025-06-24T09:00:00Z -t UTC

With explicit format:

    td range "this week" -f "%d/%m/%Y %H:%M" --now 2025-06-24T09:00:00Z -t UTC

# SEE ALSO

**td**(1), **td-diff**(1), **td-convert**(1), **td-tz**(1),
**td-info**(1), **td-config**(1), **td-completions**(1)
