//! UTC timestamps, RFC 3339 format, and the words for a length of time, no
//! external dependency.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current time as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_rfc3339(secs)
}

/// How long a duration reads as: `10d`, `5h`, `12m`, or `under a minute`.
///
/// One vocabulary for both TUIs: raul's waits and hugo's holds say the same
/// words for the same length. The largest whole unit is the one named, and a
/// length under a minute says so rather than counting zero minutes. A
/// negative length reads as no time at all.
pub fn age_text(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds < 60 {
        return "under a minute".to_string();
    }
    let days = seconds / 86_400;
    if days > 0 {
        return format!("{days}d");
    }
    let hours = seconds / 3_600;
    if hours > 0 {
        return format!("{hours}h");
    }
    format!("{}m", seconds / 60)
}

/// Format a unix timestamp as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_rfc3339(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// The seconds of an RFC 3339 timestamp.
///
/// The zone is read: `Z` is UTC, `+02:00` is two hours ahead of it. The
/// seconds are optional, and so is the fraction after them. The
/// separators are checked, not assumed: a stamp that is not one is not
/// a stamp.
pub fn parse_rfc3339(text: &str) -> Option<i64> {
    let text = text.trim();
    let bytes = text.as_bytes();
    if bytes.len() < 17 {
        return None;
    }
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
    {
        return None;
    }
    let year = digits(text.get(0..4)?)?;
    let month = digits(text.get(5..7)?)?;
    let day = digits(text.get(8..10)?)?;
    let hour = digits(text.get(11..13)?)?;
    let minute = digits(text.get(14..16)?)?;
    let mut rest = text.get(16..)?;
    let mut second = 0;
    if rest.starts_with(':') {
        second = digits(rest.get(1..3)?)?;
        rest = rest.get(3..)?;
    }
    // A fraction of a second is read and dropped: a wait is in seconds.
    if let Some(fraction) = rest.strip_prefix('.') {
        let length = fraction
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .count();
        if length == 0 {
            return None;
        }
        rest = fraction.get(length..)?;
    }
    let offset = zone_offset(rest)?;
    if !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || minute > 59
        || !second_is_valid(second, year, month, day, hour, minute, offset)
    {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second - offset)
}

/// Whether a second is one the civil clock has.
///
/// A leap second is `:60`, and it exists only at 23:59:60 UTC on 30 June
/// or 31 December. The stamp is converted to UTC, date rollover included,
/// before the decision: an offset changes the local reading, not the
/// instant.
fn second_is_valid(
    second: i64,
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    offset: i64,
) -> bool {
    if second < 60 {
        return true;
    }
    if second > 60 {
        return false;
    }
    // The instant a second earlier: the leap second follows 23:59:59 UTC.
    let utc = days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + 59 - offset;
    let days = utc.div_euclid(86_400);
    let rem = utc.rem_euclid(86_400);
    let (_, utc_month, utc_day) = civil_from_days(days);
    rem / 3_600 == 23
        && (rem % 3_600) / 60 == 59
        && matches!((utc_month, utc_day), (6, 30) | (12, 31))
}

/// The days in a month of the civil calendar.
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// The seconds of the zone of a stamp: `Z` is zero, `+02:00` is two hours
/// ahead of UTC.
fn zone_offset(zone: &str) -> Option<i64> {
    let bytes = zone.as_bytes();
    match bytes.first() {
        Some(b'Z') | Some(b'z') => {
            if bytes.len() == 1 {
                Some(0)
            } else {
                None
            }
        }
        Some(b'+') | Some(b'-') => {
            if bytes.len() != 6 || bytes[3] != b':' {
                return None;
            }
            let hours = digits(zone.get(1..3)?)?;
            let minutes = digits(zone.get(4..6)?)?;
            if hours > 23 || minutes > 59 {
                return None;
            }
            let offset = hours * 3_600 + minutes * 60;
            Some(if bytes[0] == b'-' { -offset } else { offset })
        }
        _ => None,
    }
}

/// A number written in digits, and nothing else.
fn digits(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// The days since the epoch of a date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_of_year = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_of_year + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_epoch() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn an_age_names_its_largest_whole_unit() {
        for (seconds, words) in [
            (0, "under a minute"),
            (1, "under a minute"),
            (59, "under a minute"),
            (60, "1m"),
            (3_599, "59m"),
            (3_600, "1h"),
            (86_399, "23h"),
            (86_400, "1d"),
            (10 * 86_400, "10d"),
        ] {
            assert_eq!(age_text(seconds), words, "{seconds} seconds");
        }
    }

    #[test]
    fn a_negative_age_is_no_time_at_all() {
        for seconds in [-1, -3_600, -86_400] {
            assert_eq!(age_text(seconds), "under a minute", "{seconds} seconds");
        }
    }

    #[test]
    fn formats_known_date() {
        assert_eq!(format_rfc3339(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn parses_a_stamp() {
        assert_eq!(
            parse_rfc3339("2026-11-07T13:00:00Z"),
            Some(1_794_056_400),
            "the stamp of a moment"
        );
    }

    #[test]
    fn parses_a_stamp_without_seconds() {
        assert_eq!(
            parse_rfc3339("2026-11-07T13:00Z"),
            parse_rfc3339("2026-11-07T13:00:00Z")
        );
    }

    #[test]
    fn reads_the_zone() {
        // Two hours ahead of UTC is two hours earlier in UTC.
        assert_eq!(
            parse_rfc3339("2026-11-07T13:00:00+02:00"),
            parse_rfc3339("2026-11-07T11:00:00Z")
        );
        assert_eq!(
            parse_rfc3339("2026-11-07T13:00:00-05:00"),
            parse_rfc3339("2026-11-07T18:00:00Z")
        );
    }

    #[test]
    fn reads_a_fraction_of_a_second() {
        assert_eq!(
            parse_rfc3339("2026-11-07T13:00:00.250Z"),
            parse_rfc3339("2026-11-07T13:00:00Z")
        );
    }

    #[test]
    fn refuses_what_is_not_a_stamp() {
        assert_eq!(parse_rfc3339("not a stamp"), None);
        assert_eq!(parse_rfc3339(""), None);
        // The separators are checked.
        assert_eq!(parse_rfc3339("2026-11-07X13:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026/11/07T13:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-11-07T13-00-00Z"), None);
        // The zone is checked.
        assert_eq!(parse_rfc3339("2026-11-07T13:00:00"), None);
        assert_eq!(parse_rfc3339("2026-11-07T13:00:00+2:00"), None);
        // The numbers are checked.
        assert_eq!(parse_rfc3339("2026-13-07T13:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-11-07T25:00:00Z"), None);
    }

    #[test]
    fn refuses_a_date_the_calendar_does_not_have() {
        assert_eq!(parse_rfc3339("2026-02-30T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-02-29T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-04-31T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-06-31T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-00-10T00:00:00Z"), None);
        assert_eq!(parse_rfc3339("2026-11-00T00:00:00Z"), None);
        // A leap year keeps its extra day.
        assert!(parse_rfc3339("2024-02-29T00:00:00Z").is_some());
        assert!(parse_rfc3339("2000-02-29T00:00:00Z").is_some());
        assert_eq!(parse_rfc3339("1900-02-29T00:00:00Z"), None);
    }

    #[test]
    fn reads_a_leap_second_only_at_the_utc_end_of_june_or_december() {
        // RFC 3339 writes a leap second as `:60` at 23:59:60 UTC on 30 June
        // or 31 December. A stamp in another zone names the same instant at
        // a different local reading.
        assert!(parse_rfc3339("2026-12-31T23:59:60Z").is_some());
        assert!(parse_rfc3339("2026-06-30T23:59:60Z").is_some());
        // An offset moves the reading, not the instant: this is the
        // 2026-12-31T23:59:60Z leap second.
        assert!(parse_rfc3339("2026-12-31T15:59:60-08:00").is_some());
        // The local date rolls over; the instant is still 31 December UTC.
        assert!(parse_rfc3339("2027-01-01T01:59:60+02:00").is_some());
        // The other way: 21:59:60Z, which is not a leap second.
        assert_eq!(parse_rfc3339("2026-12-31T23:59:60+02:00"), None);
        // `:60` anywhere else is not a leap second.
        assert_eq!(parse_rfc3339("2026-11-07T23:59:60Z"), None);
        assert_eq!(parse_rfc3339("2026-06-29T23:59:60Z"), None);
        assert_eq!(parse_rfc3339("2026-07-01T23:59:60Z"), None);
        assert_eq!(parse_rfc3339("2026-12-31T23:58:60Z"), None);
        assert_eq!(parse_rfc3339("2026-12-31T13:00:60Z"), None);
        assert_eq!(parse_rfc3339("2026-12-31T13:00:61Z"), None);
    }
}
