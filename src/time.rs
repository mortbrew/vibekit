//! Times written the way logs and stamps write them: UTC, ISO 8601.

use std::time::{SystemTime, UNIX_EPOCH};

/// `2026-10-25T14:03:05.123Z`.
pub fn iso_utc(at: SystemTime) -> String {
    let d = at.duration_since(UNIX_EPOCH).unwrap_or_default();
    let (days, rest) = (d.as_secs() / 86_400, d.as_secs() % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60,
        d.subsec_millis()
    )
}

/// Today, `2026-10-29` (UTC).
pub fn today() -> String {
    iso_utc(SystemTime::now())[..10].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn dates_are_written_in_utc() {
        let at = |s: u64, ms: u64| SystemTime::UNIX_EPOCH + Duration::from_millis(s * 1000 + ms);
        assert_eq!(iso_utc(at(0, 0)), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_utc(at(951_782_400, 7)), "2000-02-29T00:00:00.007Z");
        assert_eq!(iso_utc(at(1_709_251_199, 999)), "2024-02-29T23:59:59.999Z");
    }

    #[test]
    fn today_is_the_first_ten_characters_of_now() {
        let today = today();
        assert_eq!(today.len(), 10, "{today}");
        assert_eq!(&today[4..5], "-");
        assert_eq!(&today[7..8], "-");
        assert!(today.starts_with(&iso_utc(SystemTime::now())[..4]), "{today}");
    }
}
