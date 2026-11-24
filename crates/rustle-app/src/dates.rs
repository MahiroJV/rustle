//! Friendly dates for the "Modified" column, in the computer's local time.

use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDateTime};

/// `None` (the system didn't report a time) shows as an empty cell.
pub fn format_modified(time: Option<SystemTime>) -> String {
    match time {
        Some(t) => {
            let local: DateTime<Local> = t.into();
            format_date(local.naive_local(), Local::now().naive_local())
        }
        None => String::new(),
    }
}

/// - same day:      "Today, 17:25"
/// - the day before: "Yesterday, 17:25"
/// - this year:     "Mar 23, 17:25"
/// - older:         "Mar 23, 2025"
///
/// Takes "naive" times (no time zone) so tests can use fixed dates.
pub fn format_date(t: NaiveDateTime, now: NaiveDateTime) -> String {
    let (day, today) = (t.date(), now.date());
    if day == today {
        format!("Today, {}", t.format("%H:%M"))
    } else if today.pred_opt() == Some(day) {
        format!("Yesterday, {}", t.format("%H:%M"))
    } else if t.format("%Y").to_string() == now.format("%Y").to_string() {
        t.format("%b %-d, %H:%M").to_string()
    } else {
        t.format("%b %-d, %Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, min, 0).unwrap()
    }

    #[test]
    fn friendly_dates() {
        let now = at(2026, 11, 24, 18, 0);
        assert_eq!(format_date(at(2026, 11, 24, 9, 5), now), "Today, 09:05");
        assert_eq!(format_date(at(2026, 11, 23, 17, 25), now), "Yesterday, 17:25");
        assert_eq!(format_date(at(2026, 3, 2, 8, 0), now), "Mar 2, 08:00");
        assert_eq!(format_date(at(2025, 3, 23, 8, 0), now), "Mar 23, 2025");
    }

    #[test]
    fn yesterday_across_a_new_year() {
        let now = at(2027, 1, 1, 10, 0);
        assert_eq!(format_date(at(2026, 12, 31, 23, 59), now), "Yesterday, 23:59");
    }

    #[test]
    fn missing_time_is_empty() {
        assert_eq!(format_modified(None), "");
    }
}
