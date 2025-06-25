use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

fn td_cmd(temp_cfg: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("td").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp_cfg.path());
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
            "Unsupported format: an error occurred when formatting an argument",
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
fn empty_pipe_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid date format: no input provided in stdin; pass an argument or pipe data\n",
        ));
}

#[test]
fn wrong_pipe_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .write_stdin("A")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid date format: failed to parse human date \'A\': Could not match input to any known format\n",
        ));
}

#[test]
fn fails_when_no_stdin() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid date format: no input provided in stdin; pass an argument or pipe data\n",
        ));
}

#[test]
fn fails_when_no_input_interactive() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .arg("")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid date format: no input provided; pass an argument or pipe data\n",
        ));
}

#[test]
fn fails_when_wrong_input_interactive() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .arg("A")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Invalid date format: failed to parse human date \'A\': Could not match input to any known format\n",
        ));
}

#[test]
fn invalid_now_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["today", "--now", "not-a-date"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid 'now' argument: input contains invalid characters (expect RFC 3339, ex.: 2025-06-24T12:00:00Z)"));
}

#[test]
fn invalid_format_should_fail() {
    let tmp = TempDir::new().unwrap();

    td_cmd(&tmp)
        .args(["today", "--format", "not-a-date %Q"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unsupported format: an error occurred when formatting an argument\n",
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
