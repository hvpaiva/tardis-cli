include!("src/cli_defs.rs");

fn main() {
    use clap::CommandFactory;

    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR not set"));

    let mut cmd = Cli::command();

    // Generate shell completions for all supported shells.
    for &shell in clap_complete::Shell::value_variants() {
        clap_complete::generate_to(shell, &mut cmd, "td", &out_dir)
            .expect("failed to generate completions");
    }

    println!("cargo:rerun-if-changed=src/cli_defs.rs");
}
