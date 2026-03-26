% TD-COMPLETIONS(1) TARDIS Manual
% TARDIS Contributors
% 2026

# NAME

td-completions - generate shell completion scripts for td

# SYNOPSIS

**td completions** *SHELL*

# DESCRIPTION

**td completions** generates shell completion scripts for the specified
shell and writes them to stdout.  Redirect the output to the appropriate
file for your shell to enable tab completions for all **td** commands,
subcommands, and options.

# SHELLS

The following shells are supported:

- **bash**
- **zsh**
- **fish**
- **elvish**
- **powershell**

# EXAMPLES

Generate and install Bash completions:

    td completions bash > ~/.local/share/bash-completion/completions/td

Generate and install Zsh completions:

    td completions zsh > "${fpath[1]}/_td"

Generate and install Fish completions:

    td completions fish > ~/.config/fish/completions/td.fish

Generate Elvish completions:

    td completions elvish > ~/.config/elvish/lib/td.elv

Generate PowerShell completions:

    td completions powershell > td.ps1

# SEE ALSO

**td**(1), **td-diff**(1), **td-convert**(1), **td-tz**(1),
**td-info**(1), **td-range**(1), **td-config**(1)
