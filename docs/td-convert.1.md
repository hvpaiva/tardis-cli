% TD-CONVERT(1) TARDIS Manual
% TARDIS Contributors
% 2026

# NAME

td-convert - convert a date between formats

# SYNOPSIS

**td convert** *INPUT* **-\-to** *FORMAT* [*OPTIONS*]

# DESCRIPTION

**td convert** takes a date expression or formatted date string and
re-renders it in the target format specified by **-\-to**.

When **-\-from** is provided, the input is parsed according to that
strptime pattern.  When omitted, **td** attempts to auto-detect the input
format by trying RFC 3339, ISO 8601, natural-language parsing, and epoch
timestamps (with or without the **@** prefix).

Built-in format names accepted by **-\-to** (and **-\-from**):

- **iso8601** -- ISO 8601 format
- **rfc3339** -- RFC 3339 format
- **rfc2822** -- RFC 2822 format
- **epoch** / **unix** -- Unix timestamp (seconds since epoch)

Any strftime pattern or preset name from the config file is also accepted.

# OPTIONS

**-\-from** *FORMAT*
:   Input format (strptime pattern or preset name).  Auto-detected if
    omitted.

**-\-to** *FORMAT*
:   Output format (required).  Accepts strftime patterns, preset names,
    or built-in names: iso8601, rfc3339, rfc2822, epoch, unix.

**-j**, **-\-json**
:   Output as a JSON object.

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

Convert a natural-language expression to epoch:

    td convert "2025-06-24" --to epoch -t UTC

Convert from ISO 8601 to a custom format:

    td convert "2025-06-24T09:00:00Z" --to "%d/%m/%Y %H:%M" -t UTC

Auto-detect input and convert to RFC 2822:

    td convert "next friday" --to rfc2822 -t UTC

Convert a bare epoch timestamp:

    td convert 1719244800 --to "%Y-%m-%d" -t UTC

Convert with explicit input format:

    td convert "24/06/2025" --from "%d/%m/%Y" --to iso8601 -t UTC

JSON output:

    td convert "tomorrow" --to epoch --json -t UTC

# SEE ALSO

**td**(1), **td-diff**(1), **td-tz**(1), **td-info**(1),
**td-range**(1), **td-config**(1), **td-completions**(1)
