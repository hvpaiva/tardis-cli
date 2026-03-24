//! Golden snapshot tests for the TARDIS CLI parser.
//!
//! These tests capture the **exact** output of every supported expression type
//! against a fixed reference time. They form the safety net that must remain
//! green throughout the parser migration: if any snapshot changes unexpectedly,
//! the migration introduced a behavioral regression.
//!
//! Reference anchor: `--now 2025-06-15T12:00:00Z` (a Sunday, UTC noon).

use assert_cmd::Command;
use assert_fs::TempDir;
use insta::assert_snapshot;

const NOW: &str = "2025-06-15T12:00:00Z";
const FMT: &str = "%Y-%m-%dT%H:%M:%S";
const TZ: &str = "UTC";

/// Run the `td` binary with a fixed `--now`, `--format`, and `--timezone`,
/// returning trimmed stdout on success or `"ERROR: {stderr}"` on failure.
fn td_golden(expression: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let output = Command::cargo_bin("td")
        .unwrap()
        .env("XDG_CONFIG_HOME", tmp.path())
        .args(["--now", NOW, "--format", FMT, "--timezone", TZ])
        .arg(expression)
        .output()
        .unwrap();

    if output.status.success() {
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    } else {
        format!(
            "ERROR: {}",
            String::from_utf8(output.stderr).unwrap().trim()
        )
    }
}

// ============================================================
// Relative dates (8 tests)
// ============================================================

#[test]
fn golden_relative_now() {
    assert_snapshot!(td_golden("now"), @"2025-06-15T12:00:00");
}

#[test]
fn golden_relative_today() {
    assert_snapshot!(td_golden("today"), @"2025-06-15T00:00:00");
}

#[test]
fn golden_relative_tomorrow() {
    assert_snapshot!(td_golden("tomorrow"), @"2025-06-16T00:00:00");
}

#[test]
fn golden_relative_yesterday() {
    assert_snapshot!(td_golden("yesterday"), @"2025-06-14T00:00:00");
}

#[test]
fn golden_relative_overmorrow() {
    assert_snapshot!(td_golden("overmorrow"), @"2025-06-17T00:00:00");
}

#[test]
fn golden_relative_day_after_tomorrow() {
    // "day after tomorrow" is NOT supported by human-date-parser
    assert_snapshot!(td_golden("day after tomorrow"), @"ERROR: Invalid date format: failed to parse human date 'day after tomorrow': Could not match input to any known format");
}

#[test]
fn golden_relative_day_before_yesterday() {
    // "day before yesterday" is NOT supported by human-date-parser
    assert_snapshot!(td_golden("day before yesterday"), @"ERROR: Invalid date format: failed to parse human date 'day before yesterday': Could not match input to any known format");
}

#[test]
fn golden_relative_empty_defaults_to_now() {
    // Empty input defaults to "now"
    assert_snapshot!(td_golden(""), @"2025-06-15T12:00:00");
}

// ============================================================
// Day references (14 tests)
// ============================================================

#[test]
fn golden_dayref_next_monday() {
    assert_snapshot!(td_golden("next monday"), @"2025-06-16T00:00:00");
}

#[test]
fn golden_dayref_next_tuesday() {
    assert_snapshot!(td_golden("next tuesday"), @"2025-06-17T00:00:00");
}

#[test]
fn golden_dayref_next_wednesday() {
    assert_snapshot!(td_golden("next wednesday"), @"2025-06-18T00:00:00");
}

#[test]
fn golden_dayref_next_thursday() {
    assert_snapshot!(td_golden("next thursday"), @"2025-06-19T00:00:00");
}

#[test]
fn golden_dayref_next_friday() {
    assert_snapshot!(td_golden("next friday"), @"2025-06-20T00:00:00");
}

#[test]
fn golden_dayref_next_saturday() {
    assert_snapshot!(td_golden("next saturday"), @"2025-06-21T00:00:00");
}

#[test]
fn golden_dayref_next_sunday() {
    assert_snapshot!(td_golden("next sunday"), @"2025-06-22T00:00:00");
}

#[test]
fn golden_dayref_last_monday() {
    assert_snapshot!(td_golden("last monday"), @"2025-06-09T00:00:00");
}

#[test]
fn golden_dayref_last_friday() {
    assert_snapshot!(td_golden("last friday"), @"2025-06-13T00:00:00");
}

#[test]
fn golden_dayref_last_sunday() {
    assert_snapshot!(td_golden("last sunday"), @"2025-06-08T00:00:00");
}

#[test]
fn golden_dayref_last_tuesday() {
    assert_snapshot!(td_golden("last tuesday"), @"2025-06-10T00:00:00");
}

#[test]
fn golden_dayref_last_wednesday() {
    assert_snapshot!(td_golden("last wednesday"), @"2025-06-11T00:00:00");
}

#[test]
fn golden_dayref_last_thursday() {
    assert_snapshot!(td_golden("last thursday"), @"2025-06-12T00:00:00");
}

#[test]
fn golden_dayref_last_saturday() {
    // 2025-06-15 is Sunday, so last Saturday is 2025-06-14
    assert_snapshot!(td_golden("last saturday"), @"2025-06-14T00:00:00");
}

// ============================================================
// "this" day references (5 tests)
// ============================================================

#[test]
fn golden_dayref_this_monday() {
    assert_snapshot!(td_golden("this monday"), @"2025-06-16T00:00:00");
}

#[test]
fn golden_dayref_this_friday() {
    assert_snapshot!(td_golden("this friday"), @"2025-06-20T00:00:00");
}

#[test]
fn golden_dayref_this_wednesday() {
    assert_snapshot!(td_golden("this wednesday"), @"2025-06-18T00:00:00");
}

#[test]
fn golden_dayref_this_saturday() {
    assert_snapshot!(td_golden("this saturday"), @"2025-06-21T00:00:00");
}

#[test]
fn golden_dayref_this_sunday() {
    assert_snapshot!(td_golden("this sunday"), @"2025-06-15T00:00:00");
}

// ============================================================
// Time suffixes (8 tests)
// ============================================================

#[test]
fn golden_time_today_1830() {
    assert_snapshot!(td_golden("today 18:30"), @"2025-06-15T18:30:00");
}

#[test]
fn golden_time_today_0900() {
    assert_snapshot!(td_golden("today 09:00"), @"2025-06-15T09:00:00");
}

#[test]
fn golden_time_today_0000() {
    assert_snapshot!(td_golden("today 00:00"), @"2025-06-15T00:00:00");
}

#[test]
fn golden_time_today_1230() {
    assert_snapshot!(td_golden("today 12:30"), @"2025-06-15T12:30:00");
}

#[test]
fn golden_time_tomorrow_1500() {
    assert_snapshot!(td_golden("tomorrow 15:00"), @"2025-06-16T15:00:00");
}

#[test]
fn golden_time_tomorrow_0000() {
    assert_snapshot!(td_golden("tomorrow 00:00"), @"2025-06-16T00:00:00");
}

#[test]
fn golden_time_next_friday_1700() {
    assert_snapshot!(td_golden("next friday 17:00"), @"2025-06-20T17:00:00");
}

#[test]
fn golden_time_yesterday_2359() {
    assert_snapshot!(td_golden("yesterday 23:59"), @"2025-06-14T23:59:00");
}

// ============================================================
// Duration past (10 tests)
// ============================================================

#[test]
fn golden_past_1_hour_ago() {
    assert_snapshot!(td_golden("1 hour ago"), @"2025-06-15T11:00:00");
}

#[test]
fn golden_past_3_hours_ago() {
    assert_snapshot!(td_golden("3 hours ago"), @"2025-06-15T09:00:00");
}

#[test]
fn golden_past_10_hours_ago() {
    assert_snapshot!(td_golden("10 hours ago"), @"2025-06-15T02:00:00");
}

#[test]
fn golden_past_1_day_ago() {
    assert_snapshot!(td_golden("1 day ago"), @"2025-06-14T12:00:00");
}

#[test]
fn golden_past_3_days_ago() {
    assert_snapshot!(td_golden("3 days ago"), @"2025-06-12T12:00:00");
}

#[test]
fn golden_past_5_days_ago() {
    assert_snapshot!(td_golden("5 days ago"), @"2025-06-10T12:00:00");
}

#[test]
fn golden_past_a_week_ago() {
    assert_snapshot!(td_golden("a week ago"), @"2025-06-08T12:00:00");
}

#[test]
fn golden_past_2_weeks_ago() {
    assert_snapshot!(td_golden("2 weeks ago"), @"2025-06-01T12:00:00");
}

#[test]
fn golden_past_a_month_ago() {
    assert_snapshot!(td_golden("a month ago"), @"2025-05-15T12:00:00");
}

#[test]
fn golden_past_a_year_ago() {
    assert_snapshot!(td_golden("a year ago"), @"2024-06-15T12:00:00");
}

// ============================================================
// Duration past (minutes)  (3 tests)
// ============================================================

#[test]
fn golden_past_5_minutes_ago() {
    assert_snapshot!(td_golden("5 minutes ago"), @"2025-06-15T11:55:00");
}

#[test]
fn golden_past_30_minutes_ago() {
    assert_snapshot!(td_golden("30 minutes ago"), @"2025-06-15T11:30:00");
}

#[test]
fn golden_past_an_hour_ago() {
    assert_snapshot!(td_golden("an hour ago"), @"2025-06-15T11:00:00");
}

// ============================================================
// Duration future (10 tests)
// ============================================================

#[test]
fn golden_future_in_1_hour() {
    assert_snapshot!(td_golden("in 1 hour"), @"2025-06-15T13:00:00");
}

#[test]
fn golden_future_in_3_hours() {
    assert_snapshot!(td_golden("in 3 hours"), @"2025-06-15T15:00:00");
}

#[test]
fn golden_future_in_5_hours() {
    assert_snapshot!(td_golden("in 5 hours"), @"2025-06-15T17:00:00");
}

#[test]
fn golden_future_in_10_minutes() {
    assert_snapshot!(td_golden("in 10 minutes"), @"2025-06-15T12:10:00");
}

#[test]
fn golden_future_in_30_minutes() {
    assert_snapshot!(td_golden("in 30 minutes"), @"2025-06-15T12:30:00");
}

#[test]
fn golden_future_in_2_days() {
    assert_snapshot!(td_golden("in 2 days"), @"2025-06-17T12:00:00");
}

#[test]
fn golden_future_in_3_days() {
    assert_snapshot!(td_golden("in 3 days"), @"2025-06-18T12:00:00");
}

#[test]
fn golden_future_in_5_days() {
    assert_snapshot!(td_golden("in 5 days"), @"2025-06-20T12:00:00");
}

#[test]
fn golden_future_in_1_week() {
    assert_snapshot!(td_golden("in 1 week"), @"2025-06-22T12:00:00");
}

#[test]
fn golden_future_in_2_weeks() {
    assert_snapshot!(td_golden("in 2 weeks"), @"2025-06-29T12:00:00");
}

#[test]
fn golden_future_in_1_month() {
    assert_snapshot!(td_golden("in 1 month"), @"2025-07-15T12:00:00");
}

// ============================================================
// Absolute dates (5 tests)
// ============================================================

#[test]
fn golden_absolute_datetime() {
    assert_snapshot!(td_golden("2022-11-07 13:25:30"), @"2022-11-07T13:25:30");
}

#[test]
fn golden_absolute_date_only() {
    assert_snapshot!(td_golden("2025-01-01"), @"2025-01-01T00:00:00");
}

#[test]
fn golden_absolute_end_of_year() {
    assert_snapshot!(td_golden("2030-12-31 23:59:59"), @"2030-12-31T23:59:59");
}

#[test]
fn golden_absolute_same_as_now() {
    assert_snapshot!(td_golden("2025-06-15 12:00:00"), @"2025-06-15T12:00:00");
}

#[test]
fn golden_absolute_y2k() {
    assert_snapshot!(td_golden("2000-01-01 00:00:00"), @"2000-01-01T00:00:00");
}

#[test]
fn golden_absolute_partial_date() {
    assert_snapshot!(td_golden("2025-03-15"), @"2025-03-15T00:00:00");
}

#[test]
fn golden_absolute_past_date() {
    assert_snapshot!(td_golden("1999-12-31 23:59:59"), @"1999-12-31T23:59:59");
}

// ============================================================
// Epoch input (4 tests)
// ============================================================

#[test]
fn golden_epoch_standard() {
    assert_snapshot!(td_golden("@1735689600"), @"2025-01-01T00:00:00");
}

#[test]
fn golden_epoch_zero() {
    assert_snapshot!(td_golden("@0"), @"1970-01-01T00:00:00");
}

#[test]
fn golden_epoch_negative() {
    assert_snapshot!(td_golden("@-86400"), @"1969-12-31T00:00:00");
}

#[test]
fn golden_epoch_billion() {
    assert_snapshot!(td_golden("@1000000000"), @"2001-09-09T01:46:40");
}

// ============================================================
// Edge cases / failures (7 tests)
// ============================================================

#[test]
fn golden_error_question_marks() {
    assert_snapshot!(td_golden("???"), @"ERROR: Invalid date format: failed to parse human date '???': Could not match input to any known format");
}

#[test]
fn golden_error_not_a_date() {
    assert_snapshot!(td_golden("not a date"), @"ERROR: Invalid date format: failed to parse human date 'not a date': Could not match input to any known format");
}

#[test]
fn golden_error_invalid_epoch() {
    assert_snapshot!(td_golden("@abc"), @"ERROR: Invalid date format: invalid epoch timestamp: abc");
}

#[test]
fn golden_error_epoch_out_of_range() {
    assert_snapshot!(td_golden("@99999999999999999"), @"ERROR: Invalid date format: epoch timestamp out of range: 99999999999999999");
}

#[test]
fn golden_error_gibberish_long() {
    let long_input = "a".repeat(120);
    let result = td_golden(&long_input);
    assert!(result.starts_with("ERROR: Invalid date format: failed to parse human date"));
}

#[test]
fn golden_error_special_chars() {
    assert_snapshot!(td_golden("!@#$%^&*()"), @"ERROR: Invalid date format: failed to parse human date '!@#$%^&*()': Could not match input to any known format");
}

#[test]
fn golden_error_sql_injection_attempt() {
    assert_snapshot!(td_golden("'; DROP TABLE dates; --"), @"ERROR: Invalid date format: failed to parse human date ''; DROP TABLE dates; --': Could not match input to any known format");
}

// ============================================================
// Additional duration expressions (2 tests)
// ============================================================

#[test]
fn golden_past_2_hours_ago() {
    assert_snapshot!(td_golden("2 hours ago"), @"2025-06-15T10:00:00");
}

#[test]
fn golden_past_a_day_ago() {
    assert_snapshot!(td_golden("a day ago"), @"2025-06-14T12:00:00");
}
