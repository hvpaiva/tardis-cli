% TD-INFO(1) TARDIS Manual
% TARDIS Contributors
% 2026

# NAME

td-info - display calendar metadata for a date

# SYNOPSIS

**td info** [*DATE*] [*OPTIONS*]

# DESCRIPTION

**td info** shows detailed calendar metadata for the given date expression.
When no expression is provided, it defaults to "now".

The output includes fields such as: year, month, day, weekday, week number,
quarter, day of year, Julian Day Number, Unix epoch, timezone, and whether
the date falls in a leap year or during daylight saving time.

# OPTIONS

**-j**, **-\-json**
:   Output as a JSON object with all metadata fields.

**-n**, **-\-no-newline**
:   Suppress the trailing newline.

**-\-now** *DATETIME*
:   Override the current time (RFC 3339).

**-t**, **-\-timezone** *TZ*
:   Timezone for resolution (IANA/Olson ID).

**-v**, **-\-verbose**
:   Print verbose diagnostics to stderr.

**-h**, **-\-help**
:   Print help information.

# EXAMPLES

Info for the current date:

    td info --now 2025-06-24T09:00:00Z -t UTC

Info for a specific date:

    td info "2025-12-25" --now 2025-06-24T09:00:00Z -t UTC

Info for a natural-language expression:

    td info "next friday" --now 2025-06-24T09:00:00Z -t UTC

JSON output with all metadata fields:

    td info "2025-01-01" --json --now 2025-01-01T00:00:00Z -t UTC

Info with a specific timezone:

    td info "now" -t America/Sao_Paulo --now 2025-06-24T09:00:00Z

Info for a relative expression:

    td info "3 days ago" --now 2025-06-24T09:00:00Z -t UTC

# SEE ALSO

**td**(1), **td-diff**(1), **td-convert**(1), **td-tz**(1),
**td-range**(1), **td-config**(1), **td-completions**(1)
