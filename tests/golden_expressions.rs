//! Golden snapshot tests for the TARDIS CLI parser.
//!
//! These tests capture the **exact** output of every supported expression type
//! against a fixed reference time. They form the comprehensive safety net for
//! the parser: if any snapshot changes unexpectedly, a regression was introduced.
//!
//! Reference anchor: `--now 2025-06-15T12:00:00Z` (a Sunday, UTC noon).

use assert_cmd::Command;
use assert_fs::TempDir;
use insta::assert_snapshot;

const NOW: &str = "2025-06-15T12:00:00Z";
const FMT: &str = "%Y-%m-%dT%H:%M:%S";
const TZ: &str = "UTC";

fn td_golden(expression: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let output = Command::cargo_bin("td")
        .unwrap()
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("LANG", "en_US.UTF-8")
        .env_remove("LC_TIME")
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

/// Run `td` with arbitrary arguments for subcommand testing.
///
/// Unlike [`td_golden`], this helper does **not** inject `--format` since
/// subcommands define their own output formatting.  It still pins `--now` and
/// `--timezone` via the per-subcommand flags passed inside `args`.
fn td_golden_args(args: &[&str]) -> String {
    let tmp = TempDir::new().unwrap();
    let output = Command::cargo_bin("td")
        .unwrap()
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("LANG", "en_US.UTF-8")
        .env_remove("LC_TIME")
        .env_remove("NO_COLOR")
        .args(args)
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
fn golden_relative_ereyesterday() {
    assert_snapshot!(td_golden("ereyesterday"), @"2025-06-13T00:00:00");
}

#[test]
fn golden_relative_day_after_tomorrow() {
    assert_snapshot!(td_golden("day after tomorrow"), @"ERROR: Invalid date format: could not parse 'day after tomorrow' as a date expression");
}

#[test]
fn golden_relative_day_before_yesterday() {
    assert_snapshot!(td_golden("day before yesterday"), @"ERROR: Invalid date format: could not parse 'day before yesterday' as a date expression");
}

#[test]
fn golden_relative_empty_defaults_to_now() {
    assert_snapshot!(td_golden(""), @"2025-06-15T12:00:00");
}

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
    assert_snapshot!(td_golden("last saturday"), @"2025-06-14T00:00:00");
}

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

#[test]
fn golden_time_tomorrow_3pm() {
    assert_snapshot!(td_golden("tomorrow 3pm"), @"2025-06-16T15:00:00");
}

#[test]
fn golden_time_today_12am() {
    assert_snapshot!(td_golden("today 12am"), @"2025-06-15T00:00:00");
}

#[test]
fn golden_past_1_hour_ago() {
    assert_snapshot!(td_golden("1 hour ago"), @"2025-06-15T11:00:00");
}

#[test]
fn golden_past_2_hours_ago() {
    assert_snapshot!(td_golden("2 hours ago"), @"2025-06-15T10:00:00");
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
fn golden_past_a_day_ago() {
    assert_snapshot!(td_golden("a day ago"), @"2025-06-14T12:00:00");
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

#[test]
fn golden_abbrev_3h_ago() {
    assert_snapshot!(td_golden("3h ago"), @"2025-06-15T09:00:00");
}

#[test]
fn golden_abbrev_1d_ago() {
    assert_snapshot!(td_golden("1d ago"), @"2025-06-14T12:00:00");
}

#[test]
fn golden_abbrev_2w_ago() {
    assert_snapshot!(td_golden("2w ago"), @"2025-06-01T12:00:00");
}

#[test]
fn golden_abbrev_in_5min() {
    assert_snapshot!(td_golden("in 5min"), @"2025-06-15T12:05:00");
}

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

#[test]
fn golden_operator_plus_3_hours() {
    assert_snapshot!(td_golden("+3 hours"), @"2025-06-15T15:00:00");
}

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

#[test]
fn golden_epoch_smart_precision() {
    assert_snapshot!(td_golden("@99999999999999999"), @"5138-11-16T09:46:39");
}

#[test]
fn golden_epoch_milliseconds() {
    assert_snapshot!(td_golden("@1735689600000"), @"2025-01-01T00:00:00");
}

#[test]
fn golden_epoch_microseconds() {
    assert_snapshot!(td_golden("@1735689600000000"), @"2025-01-01T00:00:00");
}

#[test]
fn golden_boundary_sod() {
    assert_snapshot!(td_golden("sod"), @"2025-06-15T00:00:00");
}

#[test]
fn golden_boundary_eod() {
    assert_snapshot!(td_golden("eod"), @"2025-06-15T23:59:59");
}

#[test]
fn golden_boundary_sow() {
    assert_snapshot!(td_golden("sow"), @"2025-06-09T00:00:00");
}

#[test]
fn golden_boundary_eow() {
    assert_snapshot!(td_golden("eow"), @"2025-06-15T23:59:59");
}

#[test]
fn golden_boundary_soww() {
    assert_snapshot!(td_golden("soww"), @"2025-06-09T00:00:00");
}

#[test]
fn golden_boundary_eoww() {
    assert_snapshot!(td_golden("eoww"), @"2025-06-13T23:59:59");
}

#[test]
fn golden_boundary_som() {
    assert_snapshot!(td_golden("som"), @"2025-06-01T00:00:00");
}

#[test]
fn golden_boundary_eom() {
    assert_snapshot!(td_golden("eom"), @"2025-06-30T23:59:59");
}

#[test]
fn golden_boundary_soq() {
    assert_snapshot!(td_golden("soq"), @"2025-04-01T00:00:00");
}

#[test]
fn golden_boundary_eoq() {
    assert_snapshot!(td_golden("eoq"), @"2025-06-30T23:59:59");
}

#[test]
fn golden_boundary_soy() {
    assert_snapshot!(td_golden("soy"), @"2025-01-01T00:00:00");
}

#[test]
fn golden_boundary_eoy() {
    assert_snapshot!(td_golden("eoy"), @"2025-12-31T23:59:59");
}

#[test]
fn golden_boundary_sopd() {
    assert_snapshot!(td_golden("sopd"), @"2025-06-14T00:00:00");
}

#[test]
fn golden_boundary_eopd() {
    assert_snapshot!(td_golden("eopd"), @"2025-06-14T23:59:59");
}

#[test]
fn golden_boundary_sond() {
    assert_snapshot!(td_golden("sond"), @"2025-06-16T00:00:00");
}

#[test]
fn golden_boundary_eond() {
    assert_snapshot!(td_golden("eond"), @"2025-06-16T23:59:59");
}

#[test]
fn golden_boundary_sopw() {
    assert_snapshot!(td_golden("sopw"), @"2025-06-02T00:00:00");
}

#[test]
fn golden_boundary_eopw() {
    assert_snapshot!(td_golden("eopw"), @"2025-06-08T23:59:59");
}

#[test]
fn golden_boundary_sonw() {
    assert_snapshot!(td_golden("sonw"), @"2025-06-16T00:00:00");
}

#[test]
fn golden_boundary_eonw() {
    assert_snapshot!(td_golden("eonw"), @"2025-06-22T23:59:59");
}

#[test]
fn golden_boundary_sopm() {
    assert_snapshot!(td_golden("sopm"), @"2025-05-01T00:00:00");
}

#[test]
fn golden_boundary_eopm() {
    assert_snapshot!(td_golden("eopm"), @"2025-05-31T23:59:59");
}

#[test]
fn golden_boundary_sonm() {
    assert_snapshot!(td_golden("sonm"), @"2025-07-01T00:00:00");
}

#[test]
fn golden_boundary_eonm() {
    assert_snapshot!(td_golden("eonm"), @"2025-07-31T23:59:59");
}

#[test]
fn golden_boundary_sopq() {
    assert_snapshot!(td_golden("sopq"), @"2025-01-01T00:00:00");
}

#[test]
fn golden_boundary_eopq() {
    assert_snapshot!(td_golden("eopq"), @"2025-03-31T23:59:59");
}

#[test]
fn golden_boundary_sonq() {
    assert_snapshot!(td_golden("sonq"), @"2025-07-01T00:00:00");
}

#[test]
fn golden_boundary_eonq() {
    assert_snapshot!(td_golden("eonq"), @"2025-09-30T23:59:59");
}

#[test]
fn golden_boundary_sopy() {
    assert_snapshot!(td_golden("sopy"), @"2024-01-01T00:00:00");
}

#[test]
fn golden_boundary_eopy() {
    assert_snapshot!(td_golden("eopy"), @"2024-12-31T23:59:59");
}

#[test]
fn golden_boundary_sony() {
    assert_snapshot!(td_golden("sony"), @"2026-01-01T00:00:00");
}

#[test]
fn golden_boundary_eony() {
    assert_snapshot!(td_golden("eony"), @"2026-12-31T23:59:59");
}

#[test]
fn golden_range_last_week() {
    assert_snapshot!(td_golden("last week"), @"2025-06-02T00:00:00");
}

#[test]
fn golden_range_this_month() {
    assert_snapshot!(td_golden("this month"), @"2025-06-01T00:00:00");
}

#[test]
fn golden_range_next_year() {
    assert_snapshot!(td_golden("next year"), @"2026-01-01T00:00:00");
}

#[test]
fn golden_range_q3_2025() {
    assert_snapshot!(td_golden("Q3 2025"), @"2025-07-01T00:00:00");
}

#[test]
fn golden_arithmetic_tomorrow_plus_3_hours() {
    assert_snapshot!(td_golden("tomorrow + 3 hours"), @"2025-06-16T03:00:00");
}

#[test]
fn golden_arithmetic_chained() {
    assert_snapshot!(td_golden("now + 1 day + 3 hours - 30 minutes"), @"2025-06-16T14:30:00");
}

#[test]
fn golden_arithmetic_next_friday_minus_1_week() {
    assert_snapshot!(td_golden("next friday - 1 week"), @"2025-06-13T00:00:00");
}

#[test]
fn golden_arithmetic_eod_plus_1_hour() {
    assert_snapshot!(td_golden("eod + 1 hour"), @"2025-06-16T00:59:59");
}

#[test]
fn golden_arithmetic_sow_minus_1_day() {
    assert_snapshot!(td_golden("sow - 1 day"), @"2025-06-08T00:00:00");
}

#[test]
fn golden_verbal_3_hours_after_tomorrow() {
    assert_snapshot!(td_golden("3 hours after tomorrow"), @"2025-06-16T03:00:00");
}

#[test]
fn golden_verbal_2_days_before_next_friday() {
    assert_snapshot!(td_golden("2 days before next friday"), @"2025-06-18T00:00:00");
}

#[test]
fn golden_error_question_marks() {
    assert_snapshot!(td_golden("???"), @r"
    ERROR: Invalid date format: could not parse '???' as a date expression

    Did you mean 'a'?
    ");
}

#[test]
fn golden_error_not_a_date() {
    assert_snapshot!(td_golden("not a date"), @r"
    ERROR: Invalid date format: could not parse 'not a date' as a date expression

    Did you mean 'nov'?
    ");
}

#[test]
fn golden_error_invalid_epoch() {
    assert_snapshot!(td_golden("@abc"), @"ERROR: Invalid date format: could not parse '@abc' as a date expression");
}

#[test]
fn golden_error_gibberish_long() {
    let long_input = "a".repeat(120);
    let result = td_golden(&long_input);
    assert!(result.starts_with("ERROR: Invalid date format: could not parse '"));
}

#[test]
fn golden_error_special_chars() {
    assert_snapshot!(td_golden("!@#$%^&*()"), @r"
    ERROR: Invalid date format: could not parse '!@#$%^&*()' as a date expression

    Did you mean 'a'?
    ");
}

#[test]
fn golden_error_sql_injection_attempt() {
    assert_snapshot!(td_golden("'; DROP TABLE dates; --"), @r"
    ERROR: Invalid date format: could not parse ''; DROP TABLE dates; --' as a date expression

    Did you mean 'a'?
    ");
}

/// -----------------------------------------------------------------------
/// Subcommand: `td diff`
/// -----------------------------------------------------------------------

#[test]
fn golden_diff_months_human() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "2025-01-01", "2025-06-15", "--output", "human",
        ]),
        @"5mo 14d"
    );
}

#[test]
fn golden_diff_months_seconds() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "2025-01-01", "2025-06-15", "--output", "seconds",
        ]),
        @"14256000"
    );
}

#[test]
fn golden_diff_months_iso() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "2025-01-01", "2025-06-15", "--output", "iso",
        ]),
        @"P5M14D"
    );
}

#[test]
fn golden_diff_same_day_hours() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "2025-06-15 12:00", "2025-06-15 15:30",
        ]),
        @"3h 30m"
    );
}

#[test]
fn golden_diff_same_day_seconds() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "2025-06-15 12:00", "2025-06-15 15:30", "--output", "seconds",
        ]),
        @"12600"
    );
}

#[test]
fn golden_diff_same_day_iso() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "2025-06-15 12:00", "2025-06-15 15:30", "--output", "iso",
        ]),
        @"PT3H30M"
    );
}

#[test]
fn golden_diff_relative_expressions() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "tomorrow", "next friday",
        ]),
        @"4d"
    );
}

#[test]
fn golden_diff_relative_iso() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "tomorrow", "next friday", "--output", "iso",
        ]),
        @"P4D"
    );
}

#[test]
fn golden_diff_reversed_negative() {
    assert_snapshot!(
        td_golden_args(&[
            "diff", "--now", NOW, "--timezone", TZ,
            "2025-06-15", "2025-01-01",
        ]),
        @"5mo 14d ago"
    );
}

/// -----------------------------------------------------------------------
/// Subcommand: `td range`
/// -----------------------------------------------------------------------

#[test]
fn golden_range_this_week() {
    assert_snapshot!(
        td_golden_args(&[
            "range", "--now", NOW, "--timezone", TZ,
            "this week", "-f", "%Y-%m-%d",
        ]),
        @r"
        2025-06-09
        2025-06-15
        "
    );
}

#[test]
fn golden_range_this_month_fmt() {
    assert_snapshot!(
        td_golden_args(&[
            "range", "--now", NOW, "--timezone", TZ,
            "this month", "-f", "%Y-%m-%d",
        ]),
        @r"
        2025-06-01
        2025-06-30
        "
    );
}

#[test]
fn golden_range_tomorrow_datetime() {
    assert_snapshot!(
        td_golden_args(&[
            "range", "--now", NOW, "--timezone", TZ,
            "tomorrow", "-f", "%Y-%m-%dT%H:%M:%S",
        ]),
        @r"
        2025-06-16T00:00:00
        2025-06-16T23:59:59
        "
    );
}

#[test]
fn golden_range_quarter() {
    assert_snapshot!(
        td_golden_args(&[
            "range", "--now", NOW, "--timezone", TZ,
            "Q3 2025", "-f", "%Y-%m-%d",
        ]),
        @r"
        2025-07-01
        2025-09-30
        "
    );
}

#[test]
fn golden_range_subcmd_last_week() {
    assert_snapshot!(
        td_golden_args(&[
            "range", "--now", NOW, "--timezone", TZ,
            "last week", "-f", "%Y-%m-%d",
        ]),
        @r"
        2025-06-02
        2025-06-08
        "
    );
}

#[test]
fn golden_range_next_month() {
    assert_snapshot!(
        td_golden_args(&[
            "range", "--now", NOW, "--timezone", TZ,
            "next month", "-f", "%Y-%m-%d",
        ]),
        @r"
        2025-07-01
        2025-07-31
        "
    );
}

#[test]
fn golden_range_yesterday() {
    assert_snapshot!(
        td_golden_args(&[
            "range", "--now", NOW, "--timezone", TZ,
            "yesterday", "-f", "%Y-%m-%dT%H:%M:%S",
        ]),
        @r"
        2025-06-14T00:00:00
        2025-06-14T23:59:59
        "
    );
}

/// -----------------------------------------------------------------------
/// Subcommand: `td convert`
/// -----------------------------------------------------------------------

#[test]
fn golden_convert_to_date_format() {
    assert_snapshot!(
        td_golden_args(&[
            "convert", "--now", NOW, "--timezone", TZ,
            "2025-06-15 12:00:00", "--to", "%d/%m/%Y",
        ]),
        @"15/06/2025"
    );
}

#[test]
fn golden_convert_to_unix() {
    assert_snapshot!(
        td_golden_args(&[
            "convert", "--now", NOW, "--timezone", TZ,
            "2025-06-15 12:00:00", "--to", "unix",
        ]),
        @"1749988800"
    );
}

#[test]
fn golden_convert_to_iso() {
    assert_snapshot!(
        td_golden_args(&[
            "convert", "--now", NOW, "--timezone", TZ,
            "2025-06-15 12:00:00", "--to", "iso",
        ]),
        @"2025-06-15T12:00:00+00:00"
    );
}

#[test]
fn golden_convert_to_rfc2822() {
    assert_snapshot!(
        td_golden_args(&[
            "convert", "--now", NOW, "--timezone", TZ,
            "2025-06-15 12:00:00", "--to", "rfc2822",
        ]),
        @"Sun, 15 Jun 2025 12:00:00 +0000"
    );
}

#[test]
fn golden_convert_relative_to_long_date() {
    assert_snapshot!(
        td_golden_args(&[
            "convert", "--now", NOW, "--timezone", TZ,
            "tomorrow", "--to", "%A, %B %d, %Y",
        ]),
        @"Monday, June 16, 2025"
    );
}

#[test]
fn golden_convert_epoch_to_datetime() {
    assert_snapshot!(
        td_golden_args(&[
            "convert", "--now", NOW, "--timezone", TZ,
            "1749988800", "--to", "%Y-%m-%d %H:%M:%S",
        ]),
        @"2025-06-15 12:00:00"
    );
}

/// -----------------------------------------------------------------------
/// Subcommand: `td tz`
/// -----------------------------------------------------------------------

#[test]
fn golden_tz_utc_to_sao_paulo() {
    assert_snapshot!(
        td_golden_args(&[
            "tz", "--now", NOW, "--from", TZ,
            "2025-06-15 12:00", "--to", "America/Sao_Paulo",
        ]),
        @"2025-06-15T09:00:00-03:00"
    );
}

#[test]
fn golden_tz_utc_to_tokyo() {
    assert_snapshot!(
        td_golden_args(&[
            "tz", "--now", NOW, "--from", TZ,
            "2025-06-15 12:00", "--to", "Asia/Tokyo",
        ]),
        @"2025-06-15T21:00:00+09:00"
    );
}

#[test]
fn golden_tz_utc_to_london() {
    assert_snapshot!(
        td_golden_args(&[
            "tz", "--now", NOW, "--from", TZ,
            "2025-06-15 12:00", "--to", "Europe/London",
        ]),
        @"2025-06-15T13:00:00+01:00"
    );
}

#[test]
fn golden_tz_new_york_to_tokyo() {
    assert_snapshot!(
        td_golden_args(&[
            "tz", "--now", NOW, "--from", "America/New_York",
            "2025-06-15 08:00", "--to", "Asia/Tokyo",
        ]),
        @"2025-06-15T21:00:00+09:00"
    );
}

/// -----------------------------------------------------------------------
/// Subcommand: `td info`
/// -----------------------------------------------------------------------

#[test]
fn golden_info_mid_year() {
    assert_snapshot!(
        td_golden_args(&[
            "info", "--now", NOW, "--timezone", TZ,
            "2025-06-15",
        ]),
        @r"
        Date         Sunday, June 15, 2025
          Time         00:00:00 UTC
          Week         W24, 2025
          Quarter      Q2
          Day of Year  166/365
          Leap Year    No
          Unix Epoch   1749945600
          Julian Day   2460841.50
        "
    );
}

#[test]
fn golden_info_new_year() {
    assert_snapshot!(
        td_golden_args(&[
            "info", "--now", NOW, "--timezone", TZ,
            "2025-01-01",
        ]),
        @r"
        Date         Wednesday, January  1, 2025
          Time         00:00:00 UTC
          Week         W01, 2025
          Quarter      Q1
          Day of Year  1/365
          Leap Year    No
          Unix Epoch   1735689600
          Julian Day   2460676.50
        "
    );
}

#[test]
fn golden_info_christmas() {
    assert_snapshot!(
        td_golden_args(&[
            "info", "--now", NOW, "--timezone", TZ,
            "2025-12-25",
        ]),
        @r"
        Date         Thursday, December 25, 2025
          Time         00:00:00 UTC
          Week         W52, 2025
          Quarter      Q4
          Day of Year  359/365
          Leap Year    No
          Unix Epoch   1766620800
          Julian Day   2461034.50
        "
    );
}
