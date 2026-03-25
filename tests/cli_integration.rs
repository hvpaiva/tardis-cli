use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

fn td_cmd(temp_cfg: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("td").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp_cfg.path());
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env_remove("LC_TIME");
    cmd
}

fn write_config(temp_cfg: &TempDir, contents: &str) {
    let cfg_dir = temp_cfg.child("tardis");
    cfg_dir.create_dir_all().unwrap();
    cfg_dir.child("config.toml").write_str(contents).unwrap();
}

#[test]
fn reads_from_piped_stdin_and_trims() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "--now",
            "2024-01-01T10:00:00Z",
            "--format",
            "%Y-%m-%dT%H:%M:%S",
            "--timezone",
            "UTC",
        ])
        .write_stdin("  today \n")
        .assert()
        .success()
        .stdout(predicate::str::contains("2024-01-01T00:00:00"));
}

#[test]
fn uses_format_as_is_if_no_symbols() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["--now", "2024-01-01T00:00:00Z", "--format", "around"])
        .write_stdin("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("around"));
}

#[test]
fn should_consider_timestamp() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "--now",
            "2024-01-01T00:00:00Z",
            "--timezone",
            "UTC",
            "--format",
            "%Y-%m-%dT%H:%M:%S%:z",
        ])
        .write_stdin("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("2024-01-01T00:00:00+00:00\n"));
}

#[test]
fn uses_format_from_env_when_not_cli() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .env("TARDIS_FORMAT", "%Y")
        .args(["today", "--now", "2023-12-25T00:00:00Z"])
        .assert()
        .success()
        .stdout("2023\n");
}

#[test]
fn uses_format_from_config_when_no_cli_or_env() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%H:%M"
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .args(["now", "--now", "2024-06-24T15:00:00Z"])
        .assert()
        .success()
        .stdout("15:00\n");
}

#[test]
fn cli_argument_overrides_stdin() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "next friday",
            "--now",
            "2024-01-01T00:00:00Z",
            "--format",
            "%Y",
        ])
        .write_stdin("ignored\n")
        .assert()
        .success()
        .stdout("2024\n");
}

#[test]
fn timezone_from_env_when_not_cli() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .env("TARDIS_TIMEZONE", "UTC")
        .args(["today", "--now", "2024-06-24T00:00:00Z", "--format", "%Z"])
        .assert()
        .success()
        .stdout("UTC\n");
}

#[test]
fn invalid_timezone_from_env_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .env("TARDIS_TIMEZONE", "Mars/Olympus")
        .args(["today", "--now", "2024-06-24T00:00:00Z"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported timezone: invalid timezone ID: Mars/Olympus",
        ));
}

#[test]
fn invalid_format_in_config_should_fail() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "bad %Q"
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .args(["now", "--now", "2024-06-24T00:00:00Z"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported format: invalid format string:",
        ));
}

#[test]
fn creates_default_config_when_missing() {
    let tmp = TempDir::new().unwrap();
    let cfg_path = tmp.child("tardis/config.toml");

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2024-01-01T00:00:00Z",
            "--format",
            "%Y",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout("2024\n");

    assert!(cfg_path.path().exists(), "config.toml was not auto-created");
}

#[test]
fn uses_preset_from_config() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%H:%M"
            timezone = "UTC"

            [formats]
            iso = "%Y-%m-%d"
        "#,
    );

    td_cmd(&tmp)
        .args(["now", "--now", "2025-01-02T00:00:00Z", "--format", "iso"])
        .assert()
        .success()
        .stdout("2025-01-02\n");
}

#[test]
fn convert_timezone_when_needed() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y-%m-%dT%H:%M:%S%:z"
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .args(["now", "--now", "2024-06-24T15:00:00-03:00"])
        .assert()
        .success()
        .stdout("2024-06-24T18:00:00+00:00\n");
}

#[test]
fn cli_overrides_env_and_config() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y"
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .env("TARDIS_FORMAT", "%d")
        .args(["today", "--now", "2024-01-01T00:00:00Z", "--format", "%m"])
        .assert()
        .success()
        .stdout("01\n");
}

#[test]
fn empty_env_fallbacks_to_config() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y"
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .env("TARDIS_FORMAT", "")
        .args(["today", "--now", "2024-01-01T00:00:00Z"])
        .assert()
        .success()
        .stdout("2024\n");
}

#[test]
fn empty_pipe_defaults_to_now() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["--format", "%Y", "--timezone", "UTC"])
        .write_stdin("")
        .assert()
        .success();
}

#[test]
fn wrong_pipe_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .write_stdin("A")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid date format: could not parse 'A' as a date expression",
        ));
}

#[test]
fn no_stdin_defaults_to_now() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["--format", "%Y", "--timezone", "UTC"])
        .assert()
        .success();
}

#[test]
fn empty_arg_defaults_to_now() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["", "--format", "%Y", "--timezone", "UTC"])
        .assert()
        .success();
}

#[test]
fn fails_when_wrong_input_interactive() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .arg("A")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid date format: could not parse 'A' as a date expression",
        ));
}

#[test]
fn invalid_now_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["today", "--now", "not-a-date"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid 'now' argument:"))
        .stderr(predicate::str::contains(
            "expect RFC 3339, ex.: 2025-06-24T12:00:00Z",
        ));
}

#[test]
fn invalid_format_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["today", "--format", "not-a-date %Q"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported format: invalid format string:",
        ));
}

#[test]
fn empty_format_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["today", "--format", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Missing required argument: no output format specified\n",
        ));
}

#[test]
fn empty_format_in_config_should_fail() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = ""
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .args(["today"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Missing required argument: no output format specified\n",
        ));
}

#[test]
fn unknown_timezone_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["today", "--timezone", "Mars/Olympus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported timezone: invalid timezone ID: Mars/Olympus\n",
        ));
}

// --- Epoch support ---

#[test]
fn epoch_input_with_at_syntax() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["@1735689600", "--format", "%Y-%m-%d", "--timezone", "UTC"])
        .assert()
        .success()
        .stdout("2025-01-01\n");
}

#[test]
fn epoch_output_format() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "epoch",
            "--timezone",
            "UTC",
        ])
        .assert()
        .success()
        .stdout("1735689600\n");
}

#[test]
fn unix_output_format_alias() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "unix",
            "--timezone",
            "UTC",
        ])
        .assert()
        .success()
        .stdout("1735689600\n");
}

// --- JSON output ---

#[test]
fn json_output() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "%Y-%m-%d",
            "--timezone",
            "UTC",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"output\":\"2025-01-01\""))
        .stdout(predicate::str::contains("\"epoch\":1735689600"));
}

// --- No-newline flag ---

#[test]
fn no_newline_flag() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "%Y",
            "--timezone",
            "UTC",
            "-n",
        ])
        .assert()
        .success()
        .stdout("2025");
}

// --- Batch mode ---

#[test]
fn batch_mode_multiple_lines() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "%Y-%m-%d",
            "--timezone",
            "UTC",
        ])
        .write_stdin("today\ntomorrow\n")
        .assert()
        .success()
        .stdout("2025-01-01\n2025-01-02\n");
}

// --- Config subcommand ---

#[test]
fn config_path_subcommand() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tardis/config.toml"));
}

#[test]
fn config_show_subcommand() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y"
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("format"))
        .stdout(predicate::str::contains("timezone"));
}

#[test]
fn config_presets_subcommand() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y"
            timezone = "UTC"

            [formats]
            iso = "%Y-%m-%d"
            br = "%d/%m/%Y"
        "#,
    );

    td_cmd(&tmp)
        .args(["config", "presets"])
        .assert()
        .success()
        .stdout(predicate::str::contains("iso"))
        .stdout(predicate::str::contains("br"));
}

// --- Shell completions ---

#[test]
fn completions_bash() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn completions_zsh() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("compdef"));
}

#[test]
fn completions_fish() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn completions_elvish() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["completions", "elvish"])
        .assert()
        .success();
}

#[test]
fn completions_powershell() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["completions", "powershell"])
        .assert()
        .success();
}

// --- Man page generation ---

#[test]
fn generate_man_page() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp).arg("--generate-man").output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Basic roff structure
    assert!(output.status.success(), "man page generation failed");
    assert!(stdout.contains(".SH NAME"), "missing NAME section");

    // Reference quality sections (D-04)
    assert!(stdout.contains(".SH EXAMPLES"), "missing EXAMPLES section");
    assert!(
        stdout.contains(".SH ENVIRONMENT"),
        "missing ENVIRONMENT section"
    );
    assert!(stdout.contains(".SH FILES"), "missing FILES section");
    assert!(
        stdout.contains("EXIT STATUS") || stdout.contains("EXIT_STATUS"),
        "missing EXIT STATUS section"
    );

    // Content verification
    assert!(
        stdout.contains("TARDIS_FORMAT"),
        "ENVIRONMENT should mention TARDIS_FORMAT"
    );
    assert!(
        stdout.contains("TARDIS_TIMEZONE"),
        "ENVIRONMENT should mention TARDIS_TIMEZONE"
    );
    assert!(
        stdout.contains("config.toml"),
        "FILES should mention config.toml"
    );
}

// --- --version and --help smoke tests ---

#[test]
fn version_flag() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("td"));
}

#[test]
fn help_flag() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("TARDIS"))
        .stdout(predicate::str::contains("--format"));
}

// --- Epoch edge cases ---

#[test]
fn invalid_epoch_not_a_number() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["@notanumber", "--format", "%Y", "--timezone", "UTC"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "could not parse '@notanumber' as a date expression",
        ));
}

#[test]
fn epoch_smart_precision_large_value() {
    let tmp = TempDir::new().unwrap();

    // 17-digit epoch auto-detected as microseconds by the custom parser (~year 5138)
    td_cmd(&tmp)
        .args(["@99999999999999999", "--format", "%Y", "--timezone", "UTC"])
        .assert()
        .success()
        .stdout(predicate::str::contains("5138"));
}

#[test]
fn epoch_roundtrip() {
    let tmp = TempDir::new().unwrap();

    // @epoch -> formatted
    td_cmd(&tmp)
        .args(["@0", "--format", "%Y-%m-%dT%H:%M:%SZ", "--timezone", "UTC"])
        .assert()
        .success()
        .stdout("1970-01-01T00:00:00Z\n");
}

#[test]
fn epoch_negative_timestamp() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["@-86400", "--format", "%Y-%m-%d", "--timezone", "UTC"])
        .assert()
        .success()
        .stdout("1969-12-31\n");
}

// --- JSON edge cases ---

#[test]
fn json_with_no_newline() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "%Y",
            "--timezone",
            "UTC",
            "--json",
            "-n",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should NOT end with newline
    assert!(!stdout.ends_with('\n'));
    assert!(stdout.contains("\"output\":\"2025\""));
    assert!(stdout.contains("\"epoch\":"));
}

#[test]
fn json_with_epoch_format() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "epoch",
            "--timezone",
            "UTC",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"output\":\"1735689600\""))
        .stdout(predicate::str::contains("\"epoch\":1735689600"));
}

#[test]
fn json_with_preset() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y"
            timezone = "UTC"

            [formats]
            br = "%d/%m/%Y"
        "#,
    );

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "br",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"output\":\"01/01/2025\""));
}

// --- Batch edge cases ---

#[test]
fn batch_with_blank_lines() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "%Y-%m-%d",
            "--timezone",
            "UTC",
        ])
        .write_stdin("today\n\ntomorrow\n")
        .assert()
        .success()
        .stdout("2025-01-01\n2025-01-02\n");
}

#[test]
fn batch_single_line_not_batch() {
    let tmp = TempDir::new().unwrap();

    // A single line should produce a single result, not be treated as batch
    td_cmd(&tmp)
        .args([
            "--now",
            "2025-01-01T00:00:00Z",
            "--format",
            "%Y",
            "--timezone",
            "UTC",
        ])
        .write_stdin("today\n")
        .assert()
        .success()
        .stdout("2025\n");
}

// --- Config edge cases ---

#[test]
fn config_show_with_presets() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y"
            timezone = "UTC"

            [formats]
            iso = "%Y-%m-%d"
        "#,
    );

    td_cmd(&tmp)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[formats]"))
        .stdout(predicate::str::contains("iso"));
}

#[test]
fn config_presets_empty() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y"
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .args(["config", "presets"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No presets defined"));
}

#[test]
fn config_edit_with_nonexistent_editor() {
    let tmp = TempDir::new().unwrap();
    write_config(
        &tmp,
        r#"
            format = "%Y"
            timezone = "UTC"
        "#,
    );

    td_cmd(&tmp)
        .env("EDITOR", "/nonexistent/editor")
        .args(["config", "edit"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to open editor"));
}

// --- Error messages quality ---

#[test]
fn ambiguous_dst_resolves_compatible() {
    let tmp = TempDir::new().unwrap();

    // 2025-11-02 01:30 is ambiguous in America/New_York (DST fall-back).
    // The custom parser resolves ambiguous times using jiff's compatible()
    // semantics, picking the earlier (pre-transition) interpretation.
    td_cmd(&tmp)
        .args([
            "2025-11-02 01:30",
            "--timezone",
            "America/New_York",
            "--format",
            "%Y-%m-%d %H:%M",
            "--now",
            "2025-11-01T12:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-11-02 01:30"));
}

#[test]
fn invalid_now_format_error_message() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["today", "--now", "2025-13-01"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid 'now' argument"))
        .stderr(predicate::str::contains("expect RFC 3339"));
}

// --- Timezone edge cases ---

#[test]
fn timezone_conversion_across_date_boundary() {
    let tmp = TempDir::new().unwrap();

    // 2025-01-01 23:00 UTC should be 2025-01-02 in Tokyo (+09:00)
    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-01T23:00:00Z",
            "--timezone",
            "Asia/Tokyo",
            "--format",
            "%Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout("2025-01-02\n");
}

// --- Format edge cases ---

#[test]
fn format_with_literal_text() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-06-24T00:00:00Z",
            "--timezone",
            "UTC",
            "--format",
            "Date: %Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout("Date: 2025-06-24\n");
}

#[test]
fn format_percent_only() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today",
            "--now",
            "2025-01-15T00:00:00Z",
            "--timezone",
            "UTC",
            "--format",
            "%Y%m%d",
        ])
        .assert()
        .success()
        .stdout("20250115\n");
}

// ============================================================
// td diff integration tests (SUBCMD-01, D-01)
// ============================================================

#[test]
fn test_diff_basic_output() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-03-24",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("seconds"))
        .stdout(predicate::str::contains("P")); // ISO 8601 duration starts with P
}

#[test]
fn test_diff_json_output() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-03-24",
            "--json",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"human\""))
        .stdout(predicate::str::contains("\"seconds\""))
        .stdout(predicate::str::contains("\"iso8601\""));
}

#[test]
fn test_diff_no_newline() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-01-02",
            "-n",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.ends_with('\n'));
}

#[test]
fn test_diff_same_date_zero() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-01-01",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 seconds"));
}

// ============================================================
// td convert integration tests (SUBCMD-02, D-02)
// ============================================================

#[test]
fn test_convert_to_epoch() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "convert",
            "2025-01-01",
            "--to",
            "epoch",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("1735689600"));
}

#[test]
fn test_convert_to_iso() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["convert", "@1735689600", "--to", "iso8601", "-t", "UTC"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01-01"));
}

#[test]
fn test_convert_json() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "convert",
            "2025-01-01",
            "--to",
            "epoch",
            "--json",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"output\""))
        .stdout(predicate::str::contains("\"to_format\""));
}

// ============================================================
// td tz integration tests (SUBCMD-03, D-03)
// ============================================================

#[test]
fn test_tz_utc_to_sao_paulo() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tz",
            "2025-01-01 12:00",
            "--from",
            "UTC",
            "--to",
            "America/Sao_Paulo",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("09:00")); // UTC-3
}

#[test]
fn test_tz_json() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tz",
            "2025-01-01 12:00",
            "--from",
            "UTC",
            "--to",
            "America/Sao_Paulo",
            "--json",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"converted\""))
        .stdout(predicate::str::contains("\"from_timezone\""));
}

#[test]
fn test_tz_invalid_timezone() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["tz", "now", "--to", "Invalid/Timezone"])
        .assert()
        .failure();
}

#[test]
fn test_tz_no_newline() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "tz",
            "2025-01-01 12:00",
            "--from",
            "UTC",
            "--to",
            "America/Sao_Paulo",
            "-n",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.ends_with('\n'));
}

// ============================================================
// td info integration tests (SUBCMD-04, D-04)
// ============================================================

#[test]
fn test_info_basic() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .env("NO_COLOR", "1")
        .args([
            "info",
            "2025-03-24",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Date"))
        .stdout(predicate::str::contains("Week"))
        .stdout(predicate::str::contains("Quarter"))
        .stdout(predicate::str::contains("Day of Year"))
        .stdout(predicate::str::contains("Leap Year"))
        .stdout(predicate::str::contains("Unix Epoch"))
        .stdout(predicate::str::contains("Julian Day"));
}

#[test]
fn test_info_json() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "info",
            "2025-03-24",
            "--json",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"quarter\""))
        .stdout(predicate::str::contains("\"leap_year\""))
        .stdout(predicate::str::contains("\"julian_day\""))
        .stdout(predicate::str::contains("\"unix_epoch\""));
}

#[test]
fn test_info_leap_year() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "info",
            "2024-06-15",
            "--json",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"leap_year\":true"));
}

#[test]
fn test_info_default_now() {
    // No input should default to "now"
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .env("NO_COLOR", "1")
        .args(["info"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Date"));
}

#[test]
fn test_info_no_newline() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .env("NO_COLOR", "1")
        .args([
            "info",
            "2025-03-24",
            "-n",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.ends_with('\n'));
    assert!(stdout.contains("Date"));
}

// ============================================================
// --skip-errors integration tests (UX-01, D-10)
// ============================================================

#[test]
fn test_skip_errors_continues() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "--skip-errors",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .write_stdin("tomorrow\n$$$invalid\nyesterday\n")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should have 3 lines in stdout (2 valid + 1 empty for error)
    let stdout_lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        stdout_lines.len(),
        3,
        "Expected 3 lines, got: {:?}",
        stdout_lines
    );
    assert!(stdout_lines[1].is_empty(), "Error line should be empty");
    assert!(!stderr.is_empty(), "Error should be on stderr");
    assert_ne!(output.status.code(), Some(0), "Exit code should be 1");
}

#[test]
fn test_skip_errors_all_valid() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "--skip-errors",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .write_stdin("tomorrow\nyesterday\n")
        .assert()
        .success(); // Exit code 0 when all valid
}

// ============================================================
// Range expression output tests (D-09, PARS-05)
// ============================================================

#[test]
fn test_range_last_week_two_lines() {
    // "last week" should output two lines: start (Monday) and end (Sunday)
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "last week",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-18T00:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Range should output exactly 2 lines, got: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "2025-06-09",
        "Start should be Monday of last week"
    );
    assert_eq!(lines[1], "2025-06-15", "End should be Sunday of last week");
}

#[test]
fn test_range_this_month_two_lines() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "this month",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-18T00:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "2025-06-01");
    assert_eq!(lines[1], "2025-06-30");
}

#[test]
fn test_range_json_output() {
    // Range with --json should return {start, end} object
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "last week",
            "--json",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-18T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"start\""))
        .stdout(predicate::str::contains("\"end\""));
}

#[test]
fn test_range_q3_2025() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "Q3 2025",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-18T00:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "2025-07-01");
    assert_eq!(lines[1], "2025-09-30");
}

// ============================================================
// Arithmetic expression integration tests (PARS-04)
// ============================================================

#[test]
fn test_arithmetic_tomorrow_plus_3_hours() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tomorrow + 3 hours",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-16T03:00:00"));
}

#[test]
fn test_verbal_arithmetic_3_hours_after_tomorrow() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "3 hours after tomorrow",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-16T03:00:00"));
}

#[test]
fn test_chained_arithmetic() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "now + 1 day + 3 hours - 30 minutes",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T12:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-16T14:30:00"));
}
