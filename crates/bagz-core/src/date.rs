use std::time::{SystemTime, UNIX_EPOCH};

/// Gregorian `YYYY-MM-DD` for a Unix timestamp.
///
/// This is Howard Hinnant's `civil_from_days` algorithm. It is only used so a
/// swipe can stamp "today" without pulling in a date crate. Days before the
/// Unix epoch are not required.
pub fn utc_date(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let mut year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }
    format!("{year:04}-{month:02}-{day:02}")
}

/// Today's UTC date as `YYYY-MM-DD`.
pub fn utc_today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    utc_date(secs)
}
