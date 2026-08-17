//! HL7 DT/TM/DTM (a.k.a. TS) handling: strict parsing plus friendly rendering.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

#[derive(Debug, Clone, Copy)]
pub struct Timestamp {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub offset_minutes: Option<i32>,
    pub precision: Precision,
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days since 1970-01-01 (Howard Hinnant's civil-date algorithm).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let mp = ((m as i64) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + (d as i64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era as i64) * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// Today's date in UTC. Interface traffic is timestamped anywhere on earth, so
/// callers should allow a day of slack before calling a date "in the future".
pub fn today() -> (i32, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    civil_from_days(secs / 86_400)
}

impl Timestamp {
    /// Whole days between this timestamp and `(y, m, d)`; positive when this
    /// timestamp is later.
    pub fn days_after(&self, y: i32, m: u32, d: u32) -> i64 {
        days_from_civil(self.year, self.month.max(1), self.day.max(1)) - days_from_civil(y, m, d)
    }

    pub fn date_string(&self) -> String {
        match self.precision {
            Precision::Year => format!("{:04}", self.year),
            Precision::Month => format!("{:04}-{:02}", self.year, self.month),
            _ => format!("{:04}-{:02}-{:02}", self.year, self.month, self.day),
        }
    }

    /// `1985-03-12 14:32:05 +0100`, trimmed to the precision actually supplied.
    pub fn display(&self) -> String {
        let mut out = self.date_string();
        match self.precision {
            Precision::Hour => out.push_str(&format!(" {:02}:00", self.hour)),
            Precision::Minute => out.push_str(&format!(" {:02}:{:02}", self.hour, self.minute)),
            Precision::Second => out.push_str(&format!(
                " {:02}:{:02}:{:02}",
                self.hour, self.minute, self.second
            )),
            _ => {}
        }
        if let Some(off) = self.offset_minutes {
            let sign = if off < 0 { '-' } else { '+' };
            out.push_str(&format!(
                " {}{:02}{:02}",
                sign,
                off.abs() / 60,
                off.abs() % 60
            ));
        }
        out
    }

    /// Completed years between this date and `(y, m, d)`.
    pub fn years_until(&self, y: i32, m: u32, d: u32) -> i32 {
        let mut age = y - self.year;
        if (m, d) < (self.month.max(1), self.day.max(1)) {
            age -= 1;
        }
        age
    }
}

/// Parses `YYYY[MM[DD[HH[MM[SS[.S[S[S[S]]]]]]]]][+/-ZZZZ]`.
pub fn parse_ts(raw: &str) -> Result<Timestamp, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("empty value".into());
    }
    let (body, offset) = split_offset(raw)?;
    let (digits, fraction) = match body.split_once('.') {
        Some((d, f)) => (d, Some(f)),
        None => (body, None),
    };
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("{:?} contains non-numeric characters", raw));
    }
    if let Some(f) = fraction {
        if f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()) || f.len() > 4 {
            return Err(format!("{:?} has an invalid fractional second", raw));
        }
        if digits.len() < 14 {
            return Err("fractional seconds require a full YYYYMMDDHHMMSS value".into());
        }
    }

    let precision = match digits.len() {
        4 => Precision::Year,
        6 => Precision::Month,
        8 => Precision::Day,
        10 => Precision::Hour,
        12 => Precision::Minute,
        14 => Precision::Second,
        n => {
            return Err(format!(
                "{} digits is not a valid HL7 date/time length (expected 4, 6, 8, 10, 12 or 14)",
                n
            ))
        }
    };

    let num = |start: usize, len: usize| -> u32 {
        digits[start..start + len].parse::<u32>().unwrap_or(0)
    };
    let year = num(0, 4) as i32;
    let month = if digits.len() >= 6 { num(4, 2) } else { 1 };
    let day = if digits.len() >= 8 { num(6, 2) } else { 1 };
    let hour = if digits.len() >= 10 { num(8, 2) } else { 0 };
    let minute = if digits.len() >= 12 { num(10, 2) } else { 0 };
    let second = if digits.len() >= 14 { num(12, 2) } else { 0 };

    if year < 1 {
        return Err("year 0000 is not a valid date".into());
    }
    if precision >= Precision::Month && !(1..=12).contains(&month) {
        return Err(format!("month {:02} is out of range", month));
    }
    if precision >= Precision::Day && (day < 1 || day > days_in_month(year, month)) {
        return Err(format!(
            "day {:02} does not exist in {:04}-{:02}",
            day, year, month
        ));
    }
    // Hour 24 is accepted by some senders as midnight; HL7 does not permit it.
    if hour > 23 {
        return Err(format!("hour {:02} is out of range (00-23)", hour));
    }
    if minute > 59 {
        return Err(format!("minute {:02} is out of range (00-59)", minute));
    }
    if second > 59 {
        return Err(format!("second {:02} is out of range (00-59)", second));
    }

    Ok(Timestamp {
        year,
        month,
        day,
        hour,
        minute,
        second,
        offset_minutes: offset,
        precision,
    })
}

/// Parses a DT (date-only) value and rejects any time portion.
pub fn parse_date(raw: &str) -> Result<Timestamp, String> {
    let ts = parse_ts(raw)?;
    if ts.precision > Precision::Day {
        return Err("a date-only (DT) field must not carry a time".into());
    }
    if ts.offset_minutes.is_some() {
        return Err("a date-only (DT) field must not carry a timezone offset".into());
    }
    Ok(ts)
}

/// Parses a TM (time-only) value: `HH[MM[SS[.S...]]][+/-ZZZZ]`.
pub fn parse_time(raw: &str) -> Result<(), String> {
    let raw = raw.trim();
    let (body, _) = split_offset(raw)?;
    let digits = body.split('.').next().unwrap_or("");
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("{:?} contains non-numeric characters", raw));
    }
    if ![2usize, 4, 6].contains(&digits.len()) {
        return Err(format!("{:?} is not a valid HH[MM[SS]] time", raw));
    }
    let part = |i: usize| digits[i..i + 2].parse::<u32>().unwrap_or(99);
    if part(0) > 23 {
        return Err("hour is out of range (00-23)".into());
    }
    if digits.len() >= 4 && part(2) > 59 {
        return Err("minute is out of range (00-59)".into());
    }
    if digits.len() >= 6 && part(4) > 59 {
        return Err("second is out of range (00-59)".into());
    }
    Ok(())
}

fn split_offset(raw: &str) -> Result<(&str, Option<i32>), String> {
    let idx = raw.rfind(['+', '-']).filter(|i| *i > 0);
    let Some(idx) = idx else {
        return Ok((raw, None));
    };
    let (body, off) = raw.split_at(idx);
    let sign = if off.starts_with('-') { -1 } else { 1 };
    let digits = &off[1..];
    if digits.len() != 4 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("timezone offset {:?} must be +/-HHMM", off));
    }
    let hours: i32 = digits[0..2].parse().unwrap_or(99);
    let mins: i32 = digits[2..4].parse().unwrap_or(99);
    if hours > 14 || mins > 59 {
        return Err(format!("timezone offset {:?} is out of range", off));
    }
    Ok((body, Some(sign * (hours * 60 + mins))))
}

impl PartialOrd for Precision {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.rank().cmp(&other.rank()))
    }
}

impl Precision {
    fn rank(self) -> u8 {
        match self {
            Precision::Year => 0,
            Precision::Month => 1,
            Precision::Day => 2,
            Precision::Hour => 3,
            Precision::Minute => 4,
            Precision::Second => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_every_valid_precision() {
        for (raw, precision) in [
            ("2024", Precision::Year),
            ("202401", Precision::Month),
            ("20240115", Precision::Day),
            ("2024011514", Precision::Hour),
            ("202401151432", Precision::Minute),
            ("20240115143200", Precision::Second),
        ] {
            assert_eq!(parse_ts(raw).unwrap().precision, precision, "{}", raw);
        }
        assert!(
            parse_ts("20240115143200.5").is_ok(),
            "fractional seconds are legal"
        );
    }

    #[test]
    fn rejects_impossible_dates_and_times() {
        for raw in [
            "19850332",
            "20240015",
            "20240230",
            "2024011525",
            "202401151460",
            "2024011514320",
            "20240a15",
            "",
            "20240115.5",
        ] {
            assert!(parse_ts(raw).is_err(), "{:?} should be rejected", raw);
        }
    }

    #[test]
    fn knows_leap_years() {
        assert!(parse_ts("20240229").is_ok());
        assert!(parse_ts("20230229").is_err());
        assert!(parse_ts("20000229").is_ok());
        assert!(parse_ts("19000229").is_err());
    }

    #[test]
    fn reads_timezone_offsets() {
        let ts = parse_ts("20240115143200-0500").unwrap();
        assert_eq!(ts.offset_minutes, Some(-300));
        assert_eq!(ts.display(), "2024-01-15 14:32:00 -0500");
        assert_eq!(
            parse_ts("20240115143200+0530").unwrap().offset_minutes,
            Some(330)
        );
        assert!(parse_ts("20240115143200+530").is_err());
        assert!(parse_ts("20240115143200+2500").is_err());
    }

    #[test]
    fn date_only_fields_reject_times() {
        assert!(parse_date("19850312").is_ok());
        assert!(parse_date("198503121200").is_err());
        assert!(parse_date("19850312-0500").is_err());
    }

    #[test]
    fn time_only_fields() {
        assert!(parse_time("14").is_ok());
        assert!(parse_time("1432").is_ok());
        assert!(parse_time("143200").is_ok());
        assert!(parse_time("143200.25-0500").is_ok());
        assert!(parse_time("2500").is_err());
        assert!(parse_time("14320").is_err());
    }

    #[test]
    fn civil_day_arithmetic_round_trips() {
        for (y, m, d) in [(1970, 1, 1), (1985, 3, 12), (2024, 2, 29), (2100, 12, 31)] {
            assert_eq!(civil_from_days(days_from_civil(y, m, d)), (y, m, d));
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn computes_age_and_relative_days() {
        let dob = parse_ts("19850312").unwrap();
        assert_eq!(dob.years_until(2024, 3, 12), 39);
        assert_eq!(dob.years_until(2024, 3, 11), 38, "birthday not yet reached");
        assert_eq!(dob.days_after(1985, 3, 10), 2);
        assert_eq!(dob.days_after(1985, 3, 14), -2);
    }

    #[test]
    fn display_trims_to_the_precision_supplied() {
        assert_eq!(parse_ts("2024").unwrap().display(), "2024");
        assert_eq!(parse_ts("202401").unwrap().display(), "2024-01");
        assert_eq!(parse_ts("20240115").unwrap().display(), "2024-01-15");
        assert_eq!(
            parse_ts("202401151432").unwrap().display(),
            "2024-01-15 14:32"
        );
    }
}
