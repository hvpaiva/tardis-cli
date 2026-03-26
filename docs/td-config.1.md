% TD-CONFIG(1) TARDIS Manual
% TARDIS Contributors
% 2026

# NAME

td-config - manage the TARDIS configuration file

# SYNOPSIS

**td config** *SUBCOMMAND*

# DESCRIPTION

**td config** provides subcommands to inspect and manage the TARDIS
configuration file.  The config file is TOML-formatted and is created
automatically on first run with commented examples for every field.

# SUBCOMMANDS

**path**
:   Print the full path to the configuration file.

**show**
:   Display the effective configuration (format, timezone, and all defined
    format presets).

**edit**
:   Open the configuration file in the editor specified by the **EDITOR**
    environment variable (falls back to **vi** if unset).

**presets**
:   List all user-defined format presets (name and strftime pattern).

# EXAMPLES

Show the path to the config file:

    td config path

Display effective configuration:

    td config show

Edit the configuration file:

    td config edit

List all format presets:

    td config presets

Open with a specific editor:

    EDITOR=nano td config edit

# ENVIRONMENT

**EDITOR**
:   Editor program used by **td config edit**.  Defaults to **vi**.

**XDG_CONFIG_HOME**
:   Override the configuration directory base path.

# FILES

*$XDG_CONFIG_HOME/tardis/config.toml*

:   Configuration file.  Platform defaults when XDG_CONFIG_HOME is unset:

    - Linux: *~/.config/tardis/config.toml*
    - macOS: *~/Library/Application Support/tardis/config.toml*
    - Windows: *%APPDATA%\\tardis\\config.toml*

# SEE ALSO

**td**(1), **td-diff**(1), **td-convert**(1), **td-tz**(1),
**td-info**(1), **td-range**(1), **td-completions**(1)
