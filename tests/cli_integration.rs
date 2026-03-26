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

    // Default mode is human: shows human-readable duration only
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
        .stdout(predicate::str::contains("mo")); // human-readable contains month abbreviation
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

    // Default mode is human: same date produces empty span display
    td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-01-01",
            "--output",
            "seconds",
            "--now",
            "2025-06-15T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("0"));
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

#[test]
fn test_convert_bare_epoch_seconds() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["convert", "1750003200", "--to", "iso8601", "-t", "UTC"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-15"));
}

#[test]
fn test_convert_bare_epoch_milliseconds() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["convert", "1735689600000", "--to", "iso8601", "-t", "UTC"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01-01"));
}

#[test]
fn test_convert_bare_epoch_negative() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["convert", "--to", "iso8601", "-t", "UTC", "--", "-86400"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1969-12-31"));
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
fn test_last_week_returns_single_date() {
    // "last week" should output a single date (today - 7 days), not a range
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "last week",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-01-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout("2025-01-08\n"); // 15 - 7 = 8
}

#[test]
fn test_last_month_returns_single_date() {
    // "last month" should output a single date (today - 1 month)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "last month",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-03-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout("2025-02-15\n"); // March 15 - 1 month = Feb 15
}

#[test]
fn test_last_year_returns_single_date() {
    // "last year" should output a single date (today - 1 year)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "last year",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout("2024-06-15\n"); // 2025 - 1 year = 2024
}

// Per D-01/D-02: default command now returns single line (start of period).
// Range output moved to `td range` subcommand (D-04).

#[test]
fn test_this_month_returns_single_instant() {
    // Default command: "this month" returns start-of-period (D-01, D-02)
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
    assert_eq!(
        lines.len(),
        1,
        "Default command should return 1 line (start of period)"
    );
    assert_eq!(lines[0], "2025-06-01");
}

#[test]
fn test_this_week_returns_single_instant() {
    // Default command: "this week" returns start-of-period Monday (D-01, D-02)
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "this week",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-01-15T00:00:00Z", // Wednesday
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "Default command should return 1 line, got: {lines:?}"
    );
    assert_eq!(lines[0], "2025-01-13"); // Monday
}

#[test]
fn test_next_week_returns_single_instant() {
    // Default command: "next week" returns start of next week Monday (D-01, D-02)
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "next week",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-01-15T00:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "Default command should return 1 line, got: {lines:?}"
    );
    assert_eq!(lines[0], "2025-01-20"); // Next Monday
}

#[test]
fn test_range_subcommand_this_week() {
    // `td range "this week"` returns two lines (D-04)
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "this week",
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
    assert_eq!(lines.len(), 2, "Range subcommand should return 2 lines");
    assert_eq!(lines[0], "2025-06-16"); // Monday
    assert_eq!(lines[1], "2025-06-22"); // Sunday
}

#[test]
fn test_range_subcommand_json_output() {
    // `td range "this week" --json` returns {start, end} object (D-04)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "range",
            "this week",
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
fn test_q3_2025_returns_single_instant() {
    // Default command: "Q3 2025" returns start of Q3 (D-01, D-02)
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
    assert_eq!(lines.len(), 1, "Default command should return 1 line");
    assert_eq!(lines[0], "2025-07-01"); // Start of Q3
}

#[test]
fn test_range_subcommand_tomorrow_day_granularity() {
    // `td range "tomorrow"` returns day granularity (D-05)
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "tomorrow",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-18T12:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2, "Range subcommand should return 2 lines");
    assert_eq!(lines[0], "2025-06-19T00:00:00");
    assert_eq!(lines[1], "2025-06-19T23:59:59");
}

#[test]
fn test_range_subcommand_now_is_instant() {
    // `td range "now"` returns same value twice (D-06)
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "now",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-18T12:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2, "Range subcommand should return 2 lines");
    assert_eq!(lines[0], lines[1], "now should produce identical start/end");
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

// ============================================================
// No-space arithmetic tests — ensures lexer emits Plus/Dash
// instead of absorbing operator into a signed number (PARS-04)
// ============================================================

#[test]
fn test_arithmetic_no_space_tomorrow_plus_3h() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tomorrow+3h",
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
fn test_arithmetic_no_space_tomorrow_minus_2h() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tomorrow-2h",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-15T22:00:00"));
}

#[test]
fn test_arithmetic_no_space_now_plus_1d() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "now+1d",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T12:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-16T12:00:00"));
}

#[test]
fn test_arithmetic_no_space_chained() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "now+1d+3h-30min",
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

#[test]
fn test_arithmetic_no_space_yesterday_plus_1w() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "yesterday+1w",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-21"));
}

// Ensure "in N unit" still works (direction-based offset)
#[test]
fn test_direction_offset_still_works() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "in 5 hours",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T12:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-15T17:00:00"));
}

// Ensure ISO dates still parse correctly (dash between numbers)
#[test]
fn test_iso_date_dash_still_works() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "2025-06-15",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-15"));
}

// Ensure negative epoch still works
#[test]
fn test_negative_epoch_still_works() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "@-86400",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("1969-12-31"));
}

// Abbreviated units with spaces (regression guard)
#[test]
fn test_abbreviated_units_with_spaces() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tomorrow + 3hr",
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

    td_cmd(&tmp)
        .args([
            "tomorrow + 2wk",
            "-f",
            "%Y-%m-%d",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-30"));
}

// ============================================================
// Phase 8: Expression gaps, TW boundaries, range subcommand
// ============================================================
//
// All tests use --now 2025-03-26T12:00:00Z (a Wednesday) and -t UTC
// for determinism. Each gap pattern from the CONTEXT.md inventory
// (#1-#11) has at least one integration test.

// ── Gap #1 and #2: NhMM compound duration inference ───────────

#[test]
fn test_nhmm_compound_now_plus_13h30() {
    // Gap #1: "now+13h30" = now + 13 hours 30 minutes
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "now+13h30",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 01:30"));
}

#[test]
fn test_nhmm_with_spaces() {
    // Gap #2: "now + 13h 30" = same
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "now + 13h 30",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 01:30"));
}

#[test]
fn test_tomorrow_plus_1h30() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tomorrow+1h30",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 01:30"));
}

// ── Gap #3: N:MM as duration in arithmetic ────────────────────

#[test]
fn test_colon_duration_now_plus_13_30() {
    // Gap #3: "now+13:30" = now + 13 hours 30 minutes
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "now+13:30",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 01:30"));
}

#[test]
fn test_colon_duration_with_spaces() {
    // "now + 13:30" = same
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "now + 13:30",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 01:30"));
}

// ── Gap #4 and #5: Operator-prefixed offsets ──────────────────

#[test]
fn test_operator_prefix_plus_1h() {
    // Gap #4: "+1h" = now + 1 hour
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "+1h",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("13:00"));
}

#[test]
fn test_operator_prefix_plus_3_hours() {
    // Gap #4: "+3 hours" = now + 3 hours
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "+3 hours",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("15:00"));
}

#[test]
fn test_operator_prefix_minus_1d() {
    // Gap #4: "-1d" = now - 1 day (requires -- to avoid clap flag parsing)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
            "--",
            "-1d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-25"));
}

#[test]
fn test_operator_prefix_compound_plus_1d3h() {
    // Gap #4: "+1d3h" = now + 1 day 3 hours
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "+1d3h",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 15:00"));
}

#[test]
fn test_operator_prefix_compound_plus_1h30min() {
    // Gap #4: "+1h30min" = now + 1 hour 30 minutes
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "+1h30min",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("13:30"));
}

#[test]
fn test_operator_prefix_with_space() {
    // Gap #5: "+ 3h" = now + 3 hours (space after +)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "+ 3h",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("15:00"));
}

#[test]
fn test_operator_prefix_in_3h_equivalent() {
    // "+3h" and "in 3h" should produce the same result
    let tmp = TempDir::new().unwrap();

    let out_plus = td_cmd(&tmp)
        .args(["+3h", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .output()
        .expect("process");
    let out_in = td_cmd(&tmp)
        .args(["in 3h", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .output()
        .expect("process");
    assert_eq!(out_plus.stdout, out_in.stdout);
}

// ── Gap #6, #7, #8: Nh as time suffix with day context ───────

#[test]
fn test_today_18h_time_suffix() {
    // Gap #6: "today 18h" = "today 18:00"
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today 18h",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("18:00"));
}

#[test]
fn test_tomorrow_15h_time_suffix() {
    // Gap #6: "tomorrow 15h" = "tomorrow 15:00"
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tomorrow 15h",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 15:00"));
}

#[test]
fn test_today_18_hours_time_suffix() {
    // Gap #7: "today 18 hours" = "today 18:00"
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today 18 hours",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("18:00"));
}

#[test]
fn test_today_at_18h_time_suffix() {
    // Gap #8: "today at 18h" = "today at 18:00"
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today at 18h",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("18:00"));
}

#[test]
fn test_nh_suffix_equals_colon_time() {
    // "today 18h" and "today 18:00" must produce identical output
    let tmp = TempDir::new().unwrap();

    let out_h = td_cmd(&tmp)
        .args(["today 18h", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .output()
        .expect("process");
    let out_colon = td_cmd(&tmp)
        .args(["today 18:00", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .output()
        .expect("process");
    assert_eq!(out_h.stdout, out_colon.stdout);
}

// ── Gap #9: Time suffix + arithmetic ──────────────────────────

#[test]
fn test_today_18h_plus_2h() {
    // Gap #9: "today 18h + 2h" = today 20:00
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "today 18h + 2h",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("20:00"));
}

#[test]
fn test_tomorrow_8h_minus_30min() {
    // Gap #9: "tomorrow 8h - 30min" = tomorrow 07:30
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tomorrow 8h - 30min",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 07:30"));
}

// ── Gap #10: TW boundary keywords ────────────────────────────

#[test]
fn test_tw_eod() {
    // Gap #10: "eod" = today 23:59:59
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "eod",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-26 23:59:59"));
}

#[test]
fn test_tw_sod() {
    // "sod" = today 00:00:00
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "sod",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-26 00:00:00"));
}

#[test]
fn test_tw_sow() {
    // "sow" = Monday 00:00:00 (2025-03-26 is Wednesday -> Monday is 2025-03-24)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "sow",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-24 00:00:00"));
}

#[test]
fn test_tw_eow() {
    // "eow" = Sunday 23:59:59 (2025-03-30)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "eow",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-30 23:59:59"));
}

#[test]
fn test_tw_som() {
    // "som" = 2025-03-01 00:00:00
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "som",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-01"));
}

#[test]
fn test_tw_eom() {
    // "eom" = 2025-03-31 23:59:59
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "eom",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-31 23:59:59"));
}

#[test]
fn test_tw_soy() {
    // "soy" = 2025-01-01 00:00:00
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "soy",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-01-01"));
}

#[test]
fn test_tw_eoy() {
    // "eoy" = 2025-12-31 23:59:59
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "eoy",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-12-31 23:59:59"));
}

#[test]
fn test_tw_soww_eoww() {
    // "soww" = Monday, "eoww" = Friday 23:59:59
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "soww",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-24"));

    td_cmd(&tmp)
        .args([
            "eoww",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-28 23:59:59"));
}

#[test]
fn test_tw_soq_eoq() {
    // "soq" = Q1 start = 2025-01-01, "eoq" = Q1 end = 2025-03-31
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "soq",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-01-01"));

    td_cmd(&tmp)
        .args([
            "eoq",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-31 23:59:59"));
}

#[test]
fn test_tw_sopd_eopd() {
    // Previous day boundaries (yesterday)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "sopd",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-25 00:00:00"));

    td_cmd(&tmp)
        .args([
            "eopd",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-25 23:59:59"));
}

#[test]
fn test_tw_sond_eond() {
    // Next day boundaries (tomorrow)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "sond",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 00:00:00"));

    td_cmd(&tmp)
        .args([
            "eond",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 23:59:59"));
}

// ── Gap #11: TW keywords + arithmetic ─────────────────────────

#[test]
fn test_tw_eod_plus_1h() {
    // Gap #11: "eod + 1h" = today 23:59:59 + 1 hour
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "eod + 1h",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-27 00:59"));
}

#[test]
fn test_tw_sow_minus_1d() {
    // Gap #11: "sow - 1d" = Monday - 1 day = previous Sunday
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "sow - 1d",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-23")); // Sunday
}

#[test]
fn test_tw_eom_plus_3d() {
    // "eom + 3 days" = March 31 23:59:59 + 3 days
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "eom + 3 days",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-04-03"));
}

// ── Range subcommand tests ────────────────────────────────────

#[test]
fn test_range_subcommand_this_week_phase8() {
    // "td range 'this week'" = two lines: Monday..Sunday
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "this week",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("2025-03-24\n2025-03-30"),
        "Expected Monday..Sunday, got: {stdout}"
    );
}

#[test]
fn test_range_subcommand_tomorrow_phase8() {
    // Day granularity: 00:00:00..23:59:59
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "tomorrow",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("2025-03-27 00:00:00\n2025-03-27 23:59:59"),
        "Expected day granularity, got: {stdout}"
    );
}

#[test]
fn test_range_subcommand_tomorrow_at_18_30() {
    // Minute granularity: 18:30:00..18:30:59
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "tomorrow at 18:30",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("2025-03-27 18:30:00\n2025-03-27 18:30:59"),
        "Expected minute granularity, got: {stdout}"
    );
}

#[test]
fn test_range_subcommand_now_instant() {
    // D-06: "range now" = instant duplicated
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args(["range", "now", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 2, "range now should produce 2 lines");
    assert_eq!(lines[0], lines[1], "range now should duplicate the instant");
}

#[test]
fn test_range_subcommand_json() {
    // Range JSON output has start, end, start_epoch, end_epoch
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "range",
            "tomorrow",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"start\""))
        .stdout(predicate::str::contains("\"end\""))
        .stdout(predicate::str::contains("\"start_epoch\""))
        .stdout(predicate::str::contains("\"end_epoch\""));
}

#[test]
fn test_range_subcommand_no_newline() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "range",
            "tomorrow",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-n",
        ])
        .assert()
        .success();
    // Just verify it doesn't crash with -n flag
}

// ── Range delimiter flag ────────────

#[test]
fn test_range_subcommand_custom_delimiter_dotdot() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "tomorrow",
            "-d",
            "..",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("2025-03-27..2025-03-27"),
        "Expected dotdot delimiter, got: {stdout}"
    );
}

#[test]
fn test_range_subcommand_custom_delimiter_space() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "this week",
            "--delimiter",
            " to ",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("2025-03-24 to 2025-03-30"),
        "Expected ' to ' delimiter, got: {stdout}"
    );
}

#[test]
fn test_range_subcommand_delimiter_default_is_newline() {
    // Without -d flag, the output should be newline-separated (backward compat)
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "tomorrow",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("2025-03-27\n2025-03-27"),
        "Expected newline delimiter by default, got: {stdout}"
    );
}

#[test]
fn test_range_subcommand_delimiter_in_json() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "range",
            "tomorrow",
            "-d",
            "..",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"delimiter\":\"..\""));
}

#[test]
fn test_range_subcommand_delimiter_with_no_newline() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "range",
            "tomorrow",
            "-d",
            "..",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
            "-n",
        ])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Expected success");
    assert!(
        !stdout.ends_with('\n'),
        "Expected no trailing newline with -n flag"
    );
    assert!(
        stdout.contains(".."),
        "Expected dotdot delimiter, got: {stdout}"
    );
}

// ── Default command single-instant behavior (D-01) ────────────

#[test]
fn test_default_this_week_single_instant() {
    // D-01: "this week" in default command = Monday 00:00:00 (single line)
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "this week",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .output()
        .expect("process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "default command should return single line for 'this week'"
    );
    assert!(
        lines[0].starts_with("2025-03-24 00:00:00"),
        "should be Monday midnight"
    );
}

#[test]
fn test_default_next_month_single_instant() {
    // D-02: "next month" in default = April 1 00:00:00
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "next month",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-04-01 00:00:00"));
}

#[test]
fn test_default_next_year_single_instant() {
    // D-02: "next year" in default = Jan 1 2026 00:00:00
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "next year",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2026-01-01"));
}

// ── Regression guards (must still error) ──────────────────────

#[test]
fn test_bare_duration_3h_still_errors() {
    // D-08: "3h" without operator = error
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["3h", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_bare_duration_2_hours_still_errors() {
    // D-08: "2 hours" without operator = error
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["2 hours", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_bare_duration_1_day_still_errors() {
    // D-08: "1 day" without operator = error
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["1 day", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_operator_without_unit_plus_1_errors() {
    // D-09: "+1" without unit = error
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["+1", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_operator_without_unit_minus_1_errors() {
    // D-09: "-1" without unit = error (requires -- to avoid clap flag parsing)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["--now", "2025-03-26T12:00:00Z", "-t", "UTC", "--", "-1"])
        .assert()
        .failure();
}

#[test]
fn test_bare_18h_no_day_context_errors() {
    // D-10: "18h" without day context = error
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["18h", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_bare_30min_still_errors() {
    // D-08: "30min" without operator = error
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["30min", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

// ── Epoch regression guards (lexer sign fix) ──────────────────

#[test]
fn test_epoch_positive_still_works() {
    // Regression: "@+1735689600" must still work after lexer fix
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["@+1735689600", "-t", "UTC", "-f", "%Y-%m-%d"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-01-01"));
}

#[test]
fn test_epoch_negative_still_works() {
    // Regression: "@-86400" must still work (negative epoch)
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["@-86400", "-t", "UTC", "-f", "%Y-%m-%d"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("1969-12-31"));
}

// ============================================================
// td diff --output mode tests (D-01, 10-02)
// ============================================================

#[test]
fn diff_output_human() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-03-15",
            "--output",
            "human",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Human output should NOT contain "seconds" as a standalone value line
    assert!(
        !stdout.contains("seconds"),
        "human mode should not contain 'seconds' line, got: {stdout}"
    );
    // Should not contain ISO 8601 duration starting with P on its own line
    assert!(
        !stdout.lines().any(|l| l.starts_with('P')),
        "human mode should not contain ISO line, got: {stdout}"
    );
    // Should contain human-readable text (e.g. "mo" for months or "d" for days)
    assert!(
        stdout.contains("mo") || stdout.contains('d') || stdout.contains('y'),
        "human mode should contain readable duration words, got: {stdout}"
    );
}

#[test]
fn diff_output_seconds() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-03-15",
            "--output",
            "seconds",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Should be a bare integer (digits only, possibly with leading minus)
    assert!(
        stdout.trim().parse::<i64>().is_ok(),
        "seconds mode should output a bare integer, got: {stdout}"
    );
}

#[test]
fn diff_output_iso() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-03-15",
            "--output",
            "iso",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // ISO 8601 duration starts with P
    assert!(
        stdout.starts_with('P'),
        "iso mode should start with 'P', got: {stdout}"
    );
    // Should NOT contain multiple lines (only the ISO duration)
    assert!(
        !stdout.contains('\n'),
        "iso mode should be single line, got: {stdout}"
    );
}

#[test]
fn diff_output_default_is_human() {
    let tmp = TempDir::new().unwrap();

    // Without --output flag, default should behave like --output human
    let output_default = td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-03-15",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();

    let output_human = td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-03-15",
            "--output",
            "human",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();

    let default_stdout = String::from_utf8_lossy(&output_default.stdout);
    let human_stdout = String::from_utf8_lossy(&output_human.stdout);
    assert_eq!(
        default_stdout, human_stdout,
        "default and human modes should produce identical output"
    );
}

#[test]
fn json_piped_is_compact() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "now",
            "--json",
            "--now",
            "2025-01-01T12:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // When piped (assert_cmd captures stdout, which is not a TTY),
    // JSON should be compact: single line with no indentation
    assert!(
        !stdout.contains('\n'),
        "piped JSON should be single line, got: {stdout}"
    );
    // Validate it's actual JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("piped JSON should be valid: {e}, got: {stdout}"));
    assert!(parsed.is_object(), "JSON should be an object");
}

#[test]
fn diff_json_piped_is_compact() {
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-03-15",
            "--json",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // When piped, diff JSON should be compact single line
    assert!(
        !stdout.contains('\n'),
        "piped diff JSON should be single line, got: {stdout}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("piped diff JSON should be valid: {e}, got: {stdout}"));
    assert!(parsed.is_object(), "diff JSON should be an object");
}

// --- Verbose flag on subcommands ---

#[test]
fn verbose_diff() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "diff",
            "2025-01-01",
            "2025-06-01",
            "-v",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("[parse]"));
}

#[test]
fn verbose_convert() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "convert",
            "2025-01-01",
            "--to",
            "epoch",
            "-v",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("[parse]"));
}

#[test]
fn verbose_tz() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "tz",
            "2025-01-01 12:00",
            "--to",
            "America/New_York",
            "-v",
            "--now",
            "2025-01-01T00:00:00Z",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("[parse]"));
}

#[test]
fn verbose_info() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "info",
            "2025-01-01",
            "-v",
            "--now",
            "2025-01-01T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("[parse]"));
}

#[test]
fn verbose_range() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "range",
            "this week",
            "-v",
            "--now",
            "2025-01-06T00:00:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("[parse]"));
}
