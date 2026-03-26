% TD-TZ(1) TARDIS Manual
% TARDIS Contributors
% 2026

# NAME

td-tz - convert a datetime to a different timezone

# SYNOPSIS

**td tz** *INPUT* **-\-to** *TIMEZONE* [*OPTIONS*]

# DESCRIPTION

**td tz** converts a datetime expression from one timezone to another.
The **-\-to** flag specifies the target timezone using an IANA/Olson
identifier (e.g. "America/Sao_Paulo", "Europe/London", "UTC").

When **-\-from** is omitted, the source timezone is auto-detected from the
system local timezone or from the input itself (e.g. if the input contains
an offset like "+05:30").

The output is the same datetime re-expressed in the target timezone using
the default format (or the format configured in the config file / env var).

# OPTIONS

**-\-from** *TIMEZONE*
:   Source timezone (IANA/Olson ID).  Auto-detected from the system
    or input if omitted.

**-\-to** *TIMEZONE*
:   Target timezone (required).  Must be a valid IANA/Olson identifier.

**-j**, **-\-json**
:   Output as a JSON object.

**-n**, **-\-no-newline**
:   Suppress the trailing newline.

**-\-now** *DATETIME*
:   Override the current time (RFC 3339).

**-v**, **-\-verbose**
:   Print verbose diagnostics to stderr.

**-h**, **-\-help**
:   Print help information.

# EXAMPLES

Convert from system local time to UTC:

    td tz "now" --to UTC --now 2025-06-24T09:00:00Z

Convert from US Eastern to Sao Paulo:

    td tz "2025-06-24T12:00:00" --from America/New_York --to America/Sao_Paulo

Convert to Tokyo time:

    td tz "tomorrow at 9am" --to Asia/Tokyo --now 2025-06-24T09:00:00Z

With explicit source timezone:

    td tz "2025-01-15 08:30" --from Europe/London --to America/Chicago

JSON output:

    td tz "now" --to Europe/Berlin --json --now 2025-06-24T09:00:00Z

Deterministic timezone conversion:

    td tz "next monday at noon" --to Pacific/Auckland --now 2025-06-24T09:00:00Z

# SEE ALSO

**td**(1), **td-diff**(1), **td-convert**(1), **td-info**(1),
**td-range**(1), **td-config**(1), **td-completions**(1)
