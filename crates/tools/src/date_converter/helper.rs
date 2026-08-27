use std::str::FromStr;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone, Utc};

pub const UNIX_EPOCH_TICKS: i128 = 621_355_968_000_000_000;
const NANOS_PER_TICK: i128 = 100;
const NANOS_PER_SECOND: i128 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimestampFormat {
    Ticks,
    #[default]
    Seconds,
    Milliseconds,
}

impl TimestampFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Ticks" => Some(Self::Ticks),
            "Seconds" => Some(Self::Seconds),
            "Milliseconds" => Some(Self::Milliseconds),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ticks => "Ticks",
            Self::Seconds => "Seconds",
            Self::Milliseconds => "Milliseconds",
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DateConvertError {
    #[error("非法日期或时间戳")]
    InvalidInput,
    #[error("未知时区")]
    UnknownTimezone,
    #[error("非法纪元")]
    InvalidEpoch,
}

#[derive(Clone, Debug)]
enum Zone {
    Local,
    Named(chrono_tz::Tz),
}

pub fn timestamp_to_datetime(
    timestamp: &str,
    format: TimestampFormat,
    timezone: Option<&str>,
    epoch: Option<&str>,
) -> Result<String, DateConvertError> {
    let utc = parse_timestamp_to_utc(timestamp, format, epoch)?;
    let zone = resolve_zone(timezone)?;
    Ok(format_in_zone(&utc, &zone))
}

pub fn datetime_to_timestamp(
    datetime: &str,
    format: TimestampFormat,
    timezone: Option<&str>,
    epoch: Option<&str>,
) -> Result<String, DateConvertError> {
    let zone = resolve_zone(timezone)?;
    let utc = parse_datetime_to_utc(datetime, &zone)?;
    utc_to_timestamp_string(&utc, format, epoch)
}

pub fn looks_like_date(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if looks_like_unix_timestamp(t) {
        return true;
    }
    parse_datetime_to_utc(t, &Zone::Named(chrono_tz::UTC)).is_ok()
}

fn looks_like_unix_timestamp(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if !digits.chars().all(|c| c.is_ascii_digit()) || digits.is_empty() {
        return false;
    }
    matches!(digits.len(), 10 | 13)
}

fn resolve_zone(timezone: Option<&str>) -> Result<Zone, DateConvertError> {
    match timezone.map(str::trim).filter(|s| !s.is_empty()) {
        None | Some("local") | Some("本机") => Ok(Zone::Local),
        Some(name) => chrono_tz::Tz::from_str(name)
            .map(Zone::Named)
            .map_err(|_| DateConvertError::UnknownTimezone),
    }
}

fn parse_epoch(epoch: Option<&str>) -> Result<Option<DateTime<Utc>>, DateConvertError> {
    match epoch.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => {
            if let Ok(secs) = s.parse::<i64>() {
                return DateTime::from_timestamp(secs, 0)
                    .ok_or(DateConvertError::InvalidEpoch)
                    .map(Some);
            }
            parse_datetime_to_utc(s, &Zone::Named(chrono_tz::UTC))
                .map(Some)
                .map_err(|_| DateConvertError::InvalidEpoch)
        }
    }
}

fn parse_timestamp_to_utc(
    timestamp: &str,
    format: TimestampFormat,
    epoch: Option<&str>,
) -> Result<DateTime<Utc>, DateConvertError> {
    let raw = timestamp.trim();
    if raw.is_empty() {
        return Err(DateConvertError::InvalidInput);
    }
    let value: i128 = raw.parse().map_err(|_| DateConvertError::InvalidInput)?;
    let epoch_dt = parse_epoch(epoch)?;

    let unix_nanos = match (format, epoch_dt) {
        (TimestampFormat::Seconds, None) => value
            .checked_mul(NANOS_PER_SECOND)
            .ok_or(DateConvertError::InvalidInput)?,
        (TimestampFormat::Milliseconds, None) => value
            .checked_mul(1_000_000)
            .ok_or(DateConvertError::InvalidInput)?,
        (TimestampFormat::Ticks, None) => value
            .checked_sub(UNIX_EPOCH_TICKS)
            .and_then(|ticks| ticks.checked_mul(NANOS_PER_TICK))
            .ok_or(DateConvertError::InvalidInput)?,
        (TimestampFormat::Seconds, Some(epoch)) => {
            let epoch_nanos = epoch_nanos(epoch)?;
            value
                .checked_mul(NANOS_PER_SECOND)
                .and_then(|n| epoch_nanos.checked_add(n))
                .ok_or(DateConvertError::InvalidInput)?
        }
        (TimestampFormat::Milliseconds, Some(epoch)) => {
            let epoch_nanos = epoch_nanos(epoch)?;
            value
                .checked_mul(1_000_000)
                .and_then(|n| epoch_nanos.checked_add(n))
                .ok_or(DateConvertError::InvalidInput)?
        }
        (TimestampFormat::Ticks, Some(epoch)) => {
            let epoch_nanos = epoch_nanos(epoch)?;
            value
                .checked_mul(NANOS_PER_TICK)
                .and_then(|n| epoch_nanos.checked_add(n))
                .ok_or(DateConvertError::InvalidInput)?
        }
    };

    nanos_to_utc(unix_nanos)
}

fn utc_to_timestamp_string(
    utc: &DateTime<Utc>,
    format: TimestampFormat,
    epoch: Option<&str>,
) -> Result<String, DateConvertError> {
    let epoch_dt = parse_epoch(epoch)?;
    let unix_nanos = epoch_nanos(*utc)?;
    let value = match (format, epoch_dt) {
        (TimestampFormat::Seconds, None) => utc.timestamp() as i128,
        (TimestampFormat::Milliseconds, None) => utc.timestamp_millis() as i128,
        (TimestampFormat::Ticks, None) => {
            unix_nanos / NANOS_PER_TICK + UNIX_EPOCH_TICKS
        }
        (TimestampFormat::Seconds, Some(epoch)) => {
            (unix_nanos - epoch_nanos(epoch)?) / NANOS_PER_SECOND
        }
        (TimestampFormat::Milliseconds, Some(epoch)) => {
            (unix_nanos - epoch_nanos(epoch)?) / 1_000_000
        }
        (TimestampFormat::Ticks, Some(epoch)) => {
            (unix_nanos - epoch_nanos(epoch)?) / NANOS_PER_TICK
        }
    };
    Ok(value.to_string())
}

fn epoch_nanos(dt: DateTime<Utc>) -> Result<i128, DateConvertError> {
    dt.timestamp_nanos_opt()
        .map(|n| n as i128)
        .ok_or(DateConvertError::InvalidInput)
}

fn nanos_to_utc(unix_nanos: i128) -> Result<DateTime<Utc>, DateConvertError> {
    let secs = unix_nanos.div_euclid(NANOS_PER_SECOND);
    let nsecs = unix_nanos.rem_euclid(NANOS_PER_SECOND) as u32;
    let secs = i64::try_from(secs).map_err(|_| DateConvertError::InvalidInput)?;
    DateTime::from_timestamp(secs, nsecs).ok_or(DateConvertError::InvalidInput)
}

fn format_in_zone(utc: &DateTime<Utc>, zone: &Zone) -> String {
    match zone {
        Zone::Local => utc
            .with_timezone(&Local)
            .to_rfc3339_opts(SecondsFormat::Secs, false),
        Zone::Named(tz) => utc
            .with_timezone(tz)
            .to_rfc3339_opts(SecondsFormat::Secs, false),
    }
}

fn parse_datetime_to_utc(input: &str, zone: &Zone) -> Result<DateTime<Utc>, DateConvertError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DateConvertError::InvalidInput);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Ok(dt.with_timezone(&Utc));
    }
    const NAIVE_PATTERNS: [&str; 4] = [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
    ];
    for pattern in NAIVE_PATTERNS {
        if let Ok(naive) = NaiveDateTime::parse_from_str(input, pattern) {
            return naive_to_utc(naive, zone);
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        let naive = date
            .and_hms_opt(0, 0, 0)
            .ok_or(DateConvertError::InvalidInput)?;
        return naive_to_utc(naive, zone);
    }
    Err(DateConvertError::InvalidInput)
}

fn naive_to_utc(naive: NaiveDateTime, zone: &Zone) -> Result<DateTime<Utc>, DateConvertError> {
    match zone {
        Zone::Local => Local
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or(DateConvertError::InvalidInput),
        Zone::Named(tz) => tz
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or(DateConvertError::InvalidInput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_zero_seconds_is_1970_in_utc() {
        let got =
            timestamp_to_datetime("0", TimestampFormat::Seconds, Some("UTC"), None).unwrap();
        assert_eq!(got, "1970-01-01T00:00:00+00:00");
        assert!(got.contains("1970-01-01"));
    }

    #[test]
    fn roundtrip_seconds() {
        let ts = datetime_to_timestamp(
            "1970-01-01T00:00:00+00:00",
            TimestampFormat::Seconds,
            Some("UTC"),
            None,
        )
        .unwrap();
        assert_eq!(ts, "0");
        let back =
            timestamp_to_datetime(&ts, TimestampFormat::Seconds, Some("UTC"), None).unwrap();
        assert_eq!(back, "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn unix_epoch_ticks_is_1970_in_utc() {
        let got = timestamp_to_datetime(
            "621355968000000000",
            TimestampFormat::Ticks,
            Some("UTC"),
            None,
        )
        .unwrap();
        assert_eq!(got, "1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn invalid_input_is_err_without_echo() {
        let input = "not-a-date-xyz";
        let err =
            datetime_to_timestamp(input, TimestampFormat::Seconds, Some("UTC"), None).unwrap_err();
        assert_eq!(err, DateConvertError::InvalidInput);
        let message = err.to_string();
        assert!(!message.contains(input), "error must not include user input");
        assert!(!message.contains("xyz"));
    }
}
