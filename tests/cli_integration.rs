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

    td_cmd(&tmp)
        .args(["@99999999999999999", "--format", "%Y", "--timezone", "UTC"])
        .assert()
        .success()
        .stdout(predicate::str::contains("5138"));
}

#[test]
fn epoch_roundtrip() {
    let tmp = TempDir::new().unwrap();

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

#[test]
fn ambiguous_dst_resolves_compatible() {
    let tmp = TempDir::new().unwrap();

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

#[test]
fn timezone_conversion_across_date_boundary() {
    let tmp = TempDir::new().unwrap();

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
        .stdout(predicate::str::contains("mo"));
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
        .stdout(predicate::str::contains("09:00"));
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
        .success();
}

#[test]
fn test_last_week_returns_period_start() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "last week",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-01-15T14:30:00Z",
        ])
        .assert()
        .success()
        .stdout("2025-01-06T00:00:00\n");
}

#[test]
fn test_last_month_returns_period_start() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "last month",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-03-15T14:30:00Z",
        ])
        .assert()
        .success()
        .stdout("2025-02-01T00:00:00\n");
}

#[test]
fn test_last_year_returns_period_start() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "last year",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
            "-t",
            "UTC",
            "--now",
            "2025-06-15T14:30:00Z",
        ])
        .assert()
        .success()
        .stdout("2024-01-01T00:00:00\n");
}

#[test]
fn test_this_month_returns_single_instant() {
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
    let tmp = TempDir::new().unwrap();

    let output = td_cmd(&tmp)
        .args([
            "this week",
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
    assert_eq!(lines[0], "2025-01-13");
}

#[test]
fn test_next_week_returns_single_instant() {
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
    assert_eq!(lines[0], "2025-01-20");
}

#[test]
fn test_range_subcommand_this_week() {
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
    assert_eq!(lines[0], "2025-06-16");
    assert_eq!(lines[1], "2025-06-22");
}

#[test]
fn test_range_subcommand_json_output() {
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
    assert_eq!(lines[0], "2025-07-01");
}

#[test]
fn test_range_subcommand_tomorrow_day_granularity() {
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

#[test]
fn test_nhmm_compound_now_plus_13h30() {
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

#[test]
fn test_colon_duration_now_plus_13_30() {
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

#[test]
fn test_operator_prefix_plus_1h() {
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

#[test]
fn test_today_18h_time_suffix() {
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

#[test]
fn test_today_18h_plus_2h() {
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

#[test]
fn test_ereyesterday() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args([
            "ereyesterday",
            "--now",
            "2025-01-15T10:30:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%dT%H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-01-13T00:00:00"));
}

#[test]
fn test_boundary_eod() {
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
fn test_boundary_sod() {
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
fn test_boundary_sow() {
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
fn test_boundary_eow() {
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
fn test_boundary_som() {
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
fn test_boundary_eom() {
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
fn test_boundary_soy() {
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
fn test_boundary_eoy() {
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
fn test_boundary_soww_eoww() {
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
fn test_boundary_soq_eoq() {
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
fn test_boundary_sopd_eopd() {
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
fn test_boundary_sond_eond() {
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

#[test]
fn test_boundary_sopw() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "sopw",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-17 00:00:00"));
}

#[test]
fn test_boundary_eopw() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "eopw",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-23 23:59:59"));
}

#[test]
fn test_boundary_sopm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "sopm",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-02-01 00:00:00"));
}

#[test]
fn test_boundary_eopm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "eopm",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-02-28 23:59:59"));
}

#[test]
fn test_boundary_sopq() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "sopq",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2024-10-01 00:00:00"));
}

#[test]
fn test_boundary_eopq() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "eopq",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2024-12-31 23:59:59"));
}

#[test]
fn test_boundary_sopy() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "sopy",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2024-01-01 00:00:00"));
}

#[test]
fn test_boundary_eopy() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "eopy",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2024-12-31 23:59:59"));
}

#[test]
fn test_boundary_sonw() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "sonw",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-03-31 00:00:00"));
}

#[test]
fn test_boundary_eonw() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "eonw",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-04-06 23:59:59"));
}

#[test]
fn test_boundary_sonm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "sonm",
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
fn test_boundary_eonm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "eonm",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-04-30 23:59:59"));
}

#[test]
fn test_boundary_sonq() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "sonq",
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
fn test_boundary_eonq() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "eonq",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-06-30 23:59:59"));
}

#[test]
fn test_boundary_sony() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "sony",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2026-01-01 00:00:00"));
}

#[test]
fn test_boundary_eony() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "eony",
            "--now",
            "2025-03-26T12:00:00Z",
            "-t",
            "UTC",
            "-f",
            "%Y-%m-%d %H:%M:%S",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2026-12-31 23:59:59"));
}

#[test]
fn test_boundary_eod_plus_1h() {
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
fn test_boundary_sow_minus_1d() {
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
        .stdout(predicate::str::starts_with("2025-03-23"));
}

#[test]
fn test_boundary_eom_plus_3d() {
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

#[test]
fn test_range_subcommand_tomorrow() {
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
}

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

#[test]
fn test_default_this_week_single_instant() {
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

#[test]
fn test_bare_duration_3h_still_errors() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["3h", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_bare_duration_2_hours_still_errors() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["2 hours", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_bare_duration_1_day_still_errors() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["1 day", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_operator_without_unit_plus_1_errors() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["+1", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_operator_without_unit_minus_1_errors() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["--now", "2025-03-26T12:00:00Z", "-t", "UTC", "--", "-1"])
        .assert()
        .failure();
}

#[test]
fn test_bare_18h_no_day_context_errors() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["18h", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_bare_30min_still_errors() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["30min", "--now", "2025-03-26T12:00:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn test_epoch_positive_still_works() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["@+1735689600", "-t", "UTC", "-f", "%Y-%m-%d"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("2025-01-01"));
}

#[test]
fn test_epoch_negative_still_works() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["@-86400", "-t", "UTC", "-f", "%Y-%m-%d"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("1969-12-31"));
}

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
    assert!(
        !stdout.contains("seconds"),
        "human mode should not contain 'seconds' line, got: {stdout}"
    );
    assert!(
        !stdout.lines().any(|l| l.starts_with('P')),
        "human mode should not contain ISO line, got: {stdout}"
    );
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
    assert!(
        stdout.starts_with('P'),
        "iso mode should start with 'P', got: {stdout}"
    );
    assert!(
        !stdout.contains('\n'),
        "iso mode should be single line, got: {stdout}"
    );
}

#[test]
fn diff_output_default_is_human() {
    let tmp = TempDir::new().unwrap();

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
    assert!(
        !stdout.contains('\n'),
        "piped JSON should be single line, got: {stdout}"
    );
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
    assert!(
        !stdout.contains('\n'),
        "piped diff JSON should be single line, got: {stdout}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("piped diff JSON should be valid: {e}, got: {stdout}"));
    assert!(parsed.is_object(), "diff JSON should be an object");
}

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

#[test]
fn standalone_time_rejected_3pm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["3pm", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn standalone_time_rejected_3_30pm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["3:30pm", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn standalone_time_rejected_12am() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["12am", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn standalone_time_rejected_12pm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["12pm", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn am_pm_compound() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "next friday at 3pm",
            "--now",
            "2025-01-15T10:30:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01-17T15:00:00"));
}

#[test]
fn am_pm_tomorrow_3pm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "tomorrow at 3pm",
            "--now",
            "2025-01-15T10:30:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01-16T15:00:00"));
}

#[test]
fn standalone_time_rejected_3_30_45pm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["3:30:45pm", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn standalone_time_rejected_11_59pm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["11:59pm", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn standalone_time_rejected_3am() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["3am", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn standalone_time_rejected_3_space_pm() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["3 pm", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn standalone_time_rejected_15_00() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["15:00", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn standalone_time_rejected_15h() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args(["15h", "--now", "2025-01-15T10:30:00Z", "-t", "UTC"])
        .assert()
        .failure();
}

#[test]
fn same_time_tomorrow() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "tomorrow at same time",
            "--now",
            "2025-01-15T10:30:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01-16T10:30:00"));
}

#[test]
fn same_time_next_friday() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "next friday at same time",
            "--now",
            "2025-01-15T10:30:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01-17T10:30:00"));
}

#[test]
fn same_time_yesterday() {
    let tmp = TempDir::new().unwrap();
    td_cmd(&tmp)
        .args([
            "yesterday at same time",
            "--now",
            "2025-01-15T10:30:00Z",
            "-t",
            "UTC",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01-14T10:30:00"));
}
