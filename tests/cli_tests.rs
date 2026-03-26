#[test]
fn trycmd_docs() {
    trycmd::TestCases::new()
        .case("docs/EXPRESSIONS.md")
        .case("docs/SUBCOMMANDS.md")
        .case("docs/CONFIGURATION.md")
        .case("docs/FORMAT-SPECIFIERS.md");
}

#[test]
fn trycmd_readme() {
    trycmd::TestCases::new().case("README.md");
}

/// Validate man page sources have required pandoc structure.
///
/// Checks that every .1.md file in docs/ contains the mandatory pandoc
/// header and man page sections. This catches structural drift without
/// requiring pandoc to be installed.
#[test]
fn man_page_structure() {
    let required_sections = ["# NAME", "# SYNOPSIS", "# DESCRIPTION"];
    let man_pages = [
        "docs/td.1.md",
        "docs/td-diff.1.md",
        "docs/td-convert.1.md",
        "docs/td-tz.1.md",
        "docs/td-info.1.md",
        "docs/td-range.1.md",
        "docs/td-config.1.md",
        "docs/td-completions.1.md",
    ];

    for path in &man_pages {
        let content =
            std::fs::read_to_string(path).unwrap_or_else(|_| panic!("{path} should exist"));

        // Pandoc title block: first line must be % TITLE(SECTION) ...
        assert!(
            content.starts_with("% "),
            "{path}: missing pandoc title block (must start with '% ')"
        );

        for section in &required_sections {
            assert!(
                content.contains(section),
                "{path}: missing required section '{section}'"
            );
        }
    }
}

/// Validate that generated roff man pages exist and are non-empty.
#[test]
fn man_page_roff_snapshots() {
    let roff_pages = [
        "docs/man/td.1",
        "docs/man/td-diff.1",
        "docs/man/td-convert.1",
        "docs/man/td-tz.1",
        "docs/man/td-info.1",
        "docs/man/td-range.1",
        "docs/man/td-config.1",
        "docs/man/td-completions.1",
    ];

    for path in &roff_pages {
        let content =
            std::fs::read_to_string(path).unwrap_or_else(|_| panic!("{path} should exist"));

        assert!(
            !content.is_empty(),
            "{path}: roff man page should not be empty"
        );

        // Basic roff structure: should start with a comment or macro
        assert!(
            content.starts_with(".\\\"") || content.starts_with(".TH"),
            "{path}: does not look like valid roff (expected .\\\" or .TH header)"
        );
    }
}
