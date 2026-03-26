% TD(1) TARDIS Manual
% TARDIS Contributors
% 2026

# NAME

td - translate natural-language date expressions into formatted output

# SYNOPSIS

**td** [*OPTIONS*] [*EXPRESSION*]

**td** *SUBCOMMAND* [*ARGS*]

# DESCRIPTION

**td** (TARDIS -- Translates And Renders Dates Into Strings) converts
human-readable date and time expressions into machine-usable output.  It
accepts natural-language phrases such as "tomorrow", "next friday at 3pm",
or "2 days ago" and renders them in configurable formats.

When invoked without an expression in a terminal, **td** defaults to "now".
When STDIN is a pipe, expressions are read one per line (batch mode).

Epoch timestamps are accepted with the **@** prefix (e.g. **@1719244800**).
Smart precision auto-detects seconds, milliseconds, microseconds, and
nanoseconds.

# OPTIONS

**-f**, **-\-format** *FMT*
:   Output format (strftime pattern or preset name).  Special values
    **epoch** and **unix** emit a Unix timestamp in seconds.
    See **docs/FORMAT-SPECIFIERS.md** for the full reference.

**-t**, **-\-timezone** *TZ*
:   IANA/Olson timezone to apply (e.g. "UTC", "America/Sao_Paulo").
    If omitted, uses the system local timezone.

**-\-now** *DATETIME*
:   Override the current time.  Format: RFC 3339
    (e.g. 2025-06-24T09:00:00Z).  Useful for deterministic output in
    scripts and tests.

**-j**, **-\-json**
:   Output as a JSON object with fields: *input*, *output*, *epoch*,
    *timezone*, *format*.

**-n**, **-\-no-newline**
:   Suppress the trailing newline.

**-v**, **-\-verbose**
:   Print verbose diagnostics to stderr (config, parse steps, timing).

**-\-skip-errors**
:   In batch mode, skip lines that fail to parse instead of aborting.
    Errors are printed to stderr; stdout gets an empty line to preserve
    alignment.  Exit code is 1 if any line failed.

**-\-version**
:   Print version information and exit.

**-h**, **-\-help**
:   Print help information.  Use **-\-help** for long-form help.

# SUBCOMMANDS

**diff**
:   Compute the difference between two dates.  See **td-diff**(1).

**convert**
:   Convert a date between formats.  See **td-convert**(1).

**tz**
:   Convert a datetime to a different timezone.  See **td-tz**(1).

**info**
:   Display calendar metadata for a date.  See **td-info**(1).

**range**
:   Expand a date expression into a start/end range.  See **td-range**(1).

**config**
:   Manage the configuration file.  See **td-config**(1).

**completions**
:   Generate shell completions.  See **td-completions**(1).

# EXAMPLES

Basic usage -- parse "tomorrow":

    td tomorrow --now 2025-06-24T09:00:00Z

Custom output format:

    td "next friday" -f "%Y-%m-%d" --now 2025-06-24T09:00:00Z

Apply a specific timezone:

    td "now" -t UTC --now 2025-06-24T09:00:00Z

Parse an epoch timestamp:

    td @1719244800 --now 2025-06-24T09:00:00Z

JSON output:

    td tomorrow --json --now 2025-06-24T09:00:00Z

Batch mode (one expression per line from pipe):

    printf "tomorrow\nnext week\n" | td -f "%Y-%m-%d" --now 2025-06-24T09:00:00Z

Deterministic output with --now:

    td "in 3 days" --now 2025-01-01T00:00:00Z -f "%Y-%m-%d" -t UTC

Date arithmetic:

    td "tomorrow + 3 hours" --now 2025-06-24T09:00:00Z -t UTC

Boundary expression (end of day):

    td eod --now 2025-06-24T09:00:00Z -t UTC

Operator-prefixed offset:

    td -- +3h --now 2025-06-24T09:00:00Z -t UTC

# ENVIRONMENT

**TARDIS_FORMAT**
:   Default output format or preset name.  Overridden by **-f**.

**TARDIS_TIMEZONE**
:   Default IANA timezone.  Overridden by **-t**.

**XDG_CONFIG_HOME**
:   Override the configuration directory base path.

**EDITOR**
:   Editor used by **td config edit**.

**NO_COLOR**
:   When set (any value), disable ANSI color output.

# FILES

*$XDG_CONFIG_HOME/tardis/config.toml*

:   Configuration file.  Created automatically on first run.
    Platform defaults when XDG_CONFIG_HOME is unset:

    - Linux: *~/.config/tardis/config.toml*
    - macOS: *~/Library/Application Support/tardis/config.toml*
    - Windows: *%APPDATA%\\tardis\\config.toml*

# EXIT STATUS

**0**
:   Success.

**64** (USAGE)
:   User input error -- invalid expression, unsupported timezone, bad
    format, or missing argument.

**74** (IOERR)
:   I/O error -- failed to read stdin or write output.

**78** (CONFIG)
:   Configuration error -- corrupt or unreadable config file.

# SEE ALSO

**td-diff**(1), **td-convert**(1), **td-tz**(1), **td-info**(1),
**td-range**(1), **td-config**(1), **td-completions**(1)

Project: *https://github.com/hvpaiva/tardis-cli*
