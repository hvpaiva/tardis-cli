#[test]
fn trycmd_docs() {
    trycmd::TestCases::new()
        .case("docs/EXPRESSIONS.md")
        .case("docs/SUBCOMMANDS.md")
        .case("docs/CONFIGURATION.md");
}
