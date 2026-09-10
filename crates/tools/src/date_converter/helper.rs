use std::str::FromStr;

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::OffsetComponents;

pub const UNIX_EPOCH_TICKS: i128 = 621_355_968_000_000_000;
/// .NET `DateTime.MaxValue.Ticks` (9999-12-31 23:59:59.9999999).
const MAX_TICKS: i128 = 3_155_378_975_999_999_999;
const NANOS_PER_TICK: i128 = 100;
const NANOS_PER_MILLISECOND: i128 = 1_000_000;
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

/// Calendar fields in the selected zone. Sub-second precision is not included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateParts {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub hour: i32,
    pub minute: i32,
    pub second: i32,
}

impl Default for DateParts {
    fn default() -> Self {
        Self {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

/// Local zone plus common IANA names. `"本机"` is equivalent to `local` / empty.
pub const COMMON_TIMEZONES: &[&str] = &[
    "本机",
    "UTC",
    "America/New_York",
    "America/Chicago",
    "America/Los_Angeles",
    "America/Sao_Paulo",
    "Europe/London",
    "Europe/Paris",
    "Europe/Berlin",
    "Asia/Shanghai",
    "Asia/Hong_Kong",
    "Asia/Tokyo",
    "Asia/Singapore",
    "Asia/Kolkata",
    "Australia/Sydney",
    "Pacific/Auckland",
];

/// Convert an injected clock reading into timestamp + ISO datetime.
/// Does not read the system clock.
pub fn now_values(
    now: DateTime<Utc>,
    format: TimestampFormat,
    timezone: Option<&str>,
    epoch: Option<&str>,
) -> Result<(String, String), DateConvertError> {
    let zone = resolve_zone(timezone)?;
    let timestamp = utc_to_timestamp_string(&now, format, epoch)?;
    let datetime = format_in_zone(&now, &zone);
    Ok((timestamp, datetime))
}

/// DST explanation when `instant` is in daylight time or a DST fold.
/// Uses chrono-tz `dst_offset` for named zones, not a month table.
pub fn dst_hint(
    timezone: Option<&str>,
    instant: DateTime<Utc>,
) -> Result<Option<&'static str>, DateConvertError> {
    let zone = resolve_zone(timezone)?;
    Ok(dst_hint_in_zone(&zone, instant))
}

pub fn dst_hint_at_datetime(datetime: &str, timezone: Option<&str>) -> Option<&'static str> {
    if datetime.trim().is_empty() {
        return None;
    }
    let zone = resolve_zone(timezone).ok()?;
    let utc = parse_datetime_to_utc(datetime, &zone).ok()?;
    dst_hint_in_zone(&zone, utc)
}

pub fn datetime_to_parts(
    datetime: &str,
    timezone: Option<&str>,
) -> Result<DateParts, DateConvertError> {
    let zone = resolve_zone(timezone)?;
    let utc = parse_datetime_to_utc(datetime, &zone)?;
    Ok(parts_in_zone(&utc, &zone))
}

pub fn parts_to_values(
    parts: DateParts,
    format: TimestampFormat,
    timezone: Option<&str>,
    epoch: Option<&str>,
) -> Result<(String, String), DateConvertError> {
    let month = u32::try_from(parts.month).map_err(|_| DateConvertError::InvalidInput)?;
    let day = u32::try_from(parts.day).map_err(|_| DateConvertError::InvalidInput)?;
    let hour = u32::try_from(parts.hour).map_err(|_| DateConvertError::InvalidInput)?;
    let minute = u32::try_from(parts.minute).map_err(|_| DateConvertError::InvalidInput)?;
    let second = u32::try_from(parts.second).map_err(|_| DateConvertError::InvalidInput)?;
    let naive = NaiveDate::from_ymd_opt(parts.year, month, day)
        .and_then(|date| date.and_hms_opt(hour, minute, second))
        .ok_or(DateConvertError::InvalidInput)?;
    let zone = resolve_zone(timezone)?;
    let utc = naive_to_utc(naive, &zone)?;
    let timestamp = utc_to_timestamp_string(&utc, format, epoch)?;
    let datetime = format_in_zone(&utc, &zone);
    Ok((timestamp, datetime))
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
                let dt = DateTime::from_timestamp(secs, 0).ok_or(DateConvertError::InvalidEpoch)?;
                return ensure_utc_in_range(dt)
                    .map(Some)
                    .map_err(|_| DateConvertError::InvalidEpoch);
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

    let instant_nanos = match (format, epoch_dt) {
        (TimestampFormat::Seconds, None) => value
            .checked_mul(NANOS_PER_SECOND)
            .ok_or(DateConvertError::InvalidInput)?,
        (TimestampFormat::Milliseconds, None) => value
            .checked_mul(NANOS_PER_MILLISECOND)
            .ok_or(DateConvertError::InvalidInput)?,
        (TimestampFormat::Ticks, None) => value
            .checked_sub(UNIX_EPOCH_TICKS)
            .and_then(|ticks| ticks.checked_mul(NANOS_PER_TICK))
            .ok_or(DateConvertError::InvalidInput)?,
        (TimestampFormat::Seconds, Some(epoch)) => unix_nanos(epoch)
            .checked_add(
                value
                    .checked_mul(NANOS_PER_SECOND)
                    .ok_or(DateConvertError::InvalidInput)?,
            )
            .ok_or(DateConvertError::InvalidInput)?,
        (TimestampFormat::Milliseconds, Some(epoch)) => unix_nanos(epoch)
            .checked_add(
                value
                    .checked_mul(NANOS_PER_MILLISECOND)
                    .ok_or(DateConvertError::InvalidInput)?,
            )
            .ok_or(DateConvertError::InvalidInput)?,
        (TimestampFormat::Ticks, Some(epoch)) => unix_nanos(epoch)
            .checked_add(
                value
                    .checked_mul(NANOS_PER_TICK)
                    .ok_or(DateConvertError::InvalidInput)?,
            )
            .ok_or(DateConvertError::InvalidInput)?,
    };

    nanos_to_utc(instant_nanos)
}

fn utc_to_timestamp_string(
    utc: &DateTime<Utc>,
    format: TimestampFormat,
    epoch: Option<&str>,
) -> Result<String, DateConvertError> {
    let utc = ensure_utc_in_range(*utc)?;
    let epoch_dt = parse_epoch(epoch)?;
    let nanos = match epoch_dt {
        Some(epoch) => unix_nanos(utc) - unix_nanos(epoch),
        None => unix_nanos(utc),
    };
    let value = match (format, epoch_dt) {
        (TimestampFormat::Seconds, _) => nanos / NANOS_PER_SECOND,
        (TimestampFormat::Milliseconds, _) => nanos / NANOS_PER_MILLISECOND,
        (TimestampFormat::Ticks, None) => utc_ticks(utc),
        (TimestampFormat::Ticks, Some(_)) => nanos.div_euclid(NANOS_PER_TICK),
    };
    Ok(value.to_string())
}

fn unix_nanos(dt: DateTime<Utc>) -> i128 {
    i128::from(dt.timestamp()) * NANOS_PER_SECOND + i128::from(dt.timestamp_subsec_nanos())
}

fn utc_ticks(dt: DateTime<Utc>) -> i128 {
    unix_nanos(dt).div_euclid(NANOS_PER_TICK) + UNIX_EPOCH_TICKS
}

fn ensure_utc_in_range(dt: DateTime<Utc>) -> Result<DateTime<Utc>, DateConvertError> {
    if (0..=MAX_TICKS).contains(&utc_ticks(dt)) {
        Ok(dt)
    } else {
        Err(DateConvertError::InvalidInput)
    }
}

fn nanos_to_utc(unix_nanos: i128) -> Result<DateTime<Utc>, DateConvertError> {
    let secs = unix_nanos.div_euclid(NANOS_PER_SECOND);
    let nsecs = u32::try_from(unix_nanos.rem_euclid(NANOS_PER_SECOND))
        .map_err(|_| DateConvertError::InvalidInput)?;
    let secs = i64::try_from(secs).map_err(|_| DateConvertError::InvalidInput)?;
    let utc = DateTime::from_timestamp(secs, nsecs).ok_or(DateConvertError::InvalidInput)?;
    ensure_utc_in_range(utc)
}

fn format_in_zone(utc: &DateTime<Utc>, zone: &Zone) -> String {
    match zone {
        Zone::Local => format_round_trip(&utc.with_timezone(&Local)),
        Zone::Named(tz) => format_round_trip(&utc.with_timezone(tz)),
    }
}

/// .NET round-trip (`O`) style: 7 fractional digits (100ns) and numeric offset.
fn format_round_trip<Tz: TimeZone>(dt: &DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let frac = i64::from(dt.timestamp_subsec_nanos()) / 100;
    format!(
        "{}{:07}{}",
        dt.format("%Y-%m-%dT%H:%M:%S."),
        frac,
        dt.format("%:z")
    )
}

fn parse_datetime_to_utc(input: &str, zone: &Zone) -> Result<DateTime<Utc>, DateConvertError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DateConvertError::InvalidInput);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return ensure_utc_in_range(dt.with_timezone(&Utc));
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
    let utc = match zone {
        Zone::Local => Local
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or(DateConvertError::InvalidInput)?,
        Zone::Named(tz) => tz
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or(DateConvertError::InvalidInput)?,
    };
    ensure_utc_in_range(utc)
}

fn parts_in_zone(utc: &DateTime<Utc>, zone: &Zone) -> DateParts {
    match zone {
        Zone::Local => chrono_to_parts(&utc.with_timezone(&Local)),
        Zone::Named(tz) => chrono_to_parts(&utc.with_timezone(tz)),
    }
}

fn chrono_to_parts<Tz: TimeZone>(dt: &DateTime<Tz>) -> DateParts {
    DateParts {
        year: dt.year(),
        month: dt.month() as i32,
        day: dt.day() as i32,
        hour: dt.hour() as i32,
        minute: dt.minute() as i32,
        second: dt.second() as i32,
    }
}

fn dst_hint_in_zone(zone: &Zone, instant: DateTime<Utc>) -> Option<&'static str> {
    const ACTIVE: &str = "夏令时。";
    const AMBIGUOUS: &str = "夏令时模糊时间。";
    match zone {
        Zone::Named(tz) => {
            let local = instant.with_timezone(tz);
            if local_time_is_ambiguous(tz, local.naive_local()) {
                return Some(AMBIGUOUS);
            }
            if local.offset().dst_offset().num_seconds() != 0 {
                Some(ACTIVE)
            } else {
                None
            }
        }
        Zone::Local => {
            let local = instant.with_timezone(&Local);
            if local_time_is_ambiguous(&Local, local.naive_local()) {
                return Some(AMBIGUOUS);
            }
            if local_offset_is_dst(instant) {
                Some(ACTIVE)
            } else {
                None
            }
        }
    }
}

fn local_time_is_ambiguous<Tz: TimeZone>(tz: &Tz, naive: NaiveDateTime) -> bool {
    matches!(
        tz.from_local_datetime(&naive),
        chrono::MappedLocalTime::Ambiguous(_, _)
    )
}

fn local_offset_is_dst(instant: DateTime<Utc>) -> bool {
    let current = instant.with_timezone(&Local).offset().local_minus_utc();
    let year = instant.year();
    let probe = |month: u32| {
        Utc.with_ymd_and_hms(year, month, 15, 12, 0, 0)
            .single()
            .map(|utc| utc.with_timezone(&Local).offset().local_minus_utc())
            .unwrap_or(current)
    };
    let jan = probe(1);
    let jul = probe(7);
    jan != jul && current == jan.max(jul)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent expected values from .NET DateTimeOffset / Unix epoch arithmetic,
    // not from this helper's output.
    const YEAR1_UTC: &str = "0001-01-01T00:00:00.0000000+00:00";
    const YEAR9999_UTC: &str = "9999-12-31T23:59:59.9999999+00:00";
    const UNIX_EPOCH_UTC: &str = "1970-01-01T00:00:00.0000000+00:00";
    const ONE_MS_UTC: &str = "1970-01-01T00:00:00.0010000+00:00";
    const ONE_TICK_UTC: &str = "1970-01-01T00:00:00.0000001+00:00";
    const ONE_TICK_YEAR1_UTC: &str = "0001-01-01T00:00:00.0000001+00:00";
    const MAX_TICKS: &str = "3155378975999999999";
    const UNIX_EPOCH_TICKS_STR: &str = "621355968000000000";
    const YEAR1_UNIX_SECONDS: &str = "-62135596800";
    const YEAR1_UNIX_MILLISECONDS: &str = "-62135596800000";
    const YEAR9999_UNIX_SECONDS: &str = "253402300799";
    const YEAR9999_UNIX_MILLISECONDS: &str = "253402300799999";

    fn round_trip(value: &str, format: TimestampFormat, tz: Option<&str>, epoch: Option<&str>) {
        let datetime = timestamp_to_datetime(value, format, tz, epoch).unwrap();
        let back = datetime_to_timestamp(&datetime, format, tz, epoch).unwrap();
        assert_eq!(
            back, value,
            "round-trip {format:?} tz={tz:?} epoch={epoch:?}"
        );
    }

    #[test]
    fn unix_zero_seconds_is_1970_in_utc() {
        let got = timestamp_to_datetime("0", TimestampFormat::Seconds, Some("UTC"), None).unwrap();
        assert_eq!(got, UNIX_EPOCH_UTC);
        assert!(got.contains("1970-01-01"));
    }

    #[test]
    fn roundtrip_seconds() {
        let ts = datetime_to_timestamp(UNIX_EPOCH_UTC, TimestampFormat::Seconds, Some("UTC"), None)
            .unwrap();
        assert_eq!(ts, "0");
        let back = timestamp_to_datetime(&ts, TimestampFormat::Seconds, Some("UTC"), None).unwrap();
        assert_eq!(back, UNIX_EPOCH_UTC);
    }

    #[test]
    fn unix_epoch_ticks_is_1970_in_utc() {
        let got = timestamp_to_datetime(
            UNIX_EPOCH_TICKS_STR,
            TimestampFormat::Ticks,
            Some("UTC"),
            None,
        )
        .unwrap();
        assert_eq!(got, UNIX_EPOCH_UTC);
    }

    #[test]
    fn zero_ticks_is_year_1_utc() {
        let got = timestamp_to_datetime("0", TimestampFormat::Ticks, Some("UTC"), None).unwrap();
        assert_eq!(got, YEAR1_UTC);
        assert_eq!(
            datetime_to_timestamp(YEAR1_UTC, TimestampFormat::Ticks, Some("UTC"), None).unwrap(),
            "0"
        );
        round_trip("0", TimestampFormat::Ticks, Some("UTC"), None);
    }

    #[test]
    fn one_millisecond_keeps_fractional_seconds() {
        let got =
            timestamp_to_datetime("1", TimestampFormat::Milliseconds, Some("UTC"), None).unwrap();
        assert_eq!(got, ONE_MS_UTC);
        assert!(
            got.contains(".001"),
            "must keep millisecond precision: {got}"
        );
        assert_eq!(
            datetime_to_timestamp(&got, TimestampFormat::Milliseconds, Some("UTC"), None).unwrap(),
            "1"
        );
        round_trip("1", TimestampFormat::Milliseconds, Some("UTC"), None);
    }

    #[test]
    fn one_tick_after_unix_epoch_keeps_100ns() {
        let ticks = "621355968000000001";
        let got = timestamp_to_datetime(ticks, TimestampFormat::Ticks, Some("UTC"), None).unwrap();
        assert_eq!(got, ONE_TICK_UTC);
        assert!(got.contains(".0000001"), "must keep tick precision: {got}");
        assert_eq!(
            datetime_to_timestamp(&got, TimestampFormat::Ticks, Some("UTC"), None).unwrap(),
            ticks
        );
        round_trip(ticks, TimestampFormat::Ticks, Some("UTC"), None);
    }

    #[test]
    fn one_tick_from_year_1() {
        let got = timestamp_to_datetime("1", TimestampFormat::Ticks, Some("UTC"), None).unwrap();
        assert_eq!(got, ONE_TICK_YEAR1_UTC);
        round_trip("1", TimestampFormat::Ticks, Some("UTC"), None);
    }

    #[test]
    fn year_1_and_9999_are_in_range() {
        assert_eq!(
            datetime_to_timestamp(YEAR1_UTC, TimestampFormat::Ticks, Some("UTC"), None).unwrap(),
            "0"
        );
        assert_eq!(
            datetime_to_timestamp(YEAR1_UTC, TimestampFormat::Seconds, Some("UTC"), None).unwrap(),
            YEAR1_UNIX_SECONDS
        );
        assert_eq!(
            datetime_to_timestamp(YEAR1_UTC, TimestampFormat::Milliseconds, Some("UTC"), None)
                .unwrap(),
            YEAR1_UNIX_MILLISECONDS
        );
        assert_eq!(
            timestamp_to_datetime(
                YEAR1_UNIX_SECONDS,
                TimestampFormat::Seconds,
                Some("UTC"),
                None
            )
            .unwrap(),
            YEAR1_UTC
        );
        assert_eq!(
            datetime_to_timestamp(YEAR9999_UTC, TimestampFormat::Ticks, Some("UTC"), None).unwrap(),
            MAX_TICKS
        );
        assert_eq!(
            timestamp_to_datetime(MAX_TICKS, TimestampFormat::Ticks, Some("UTC"), None).unwrap(),
            YEAR9999_UTC
        );
        assert_eq!(
            datetime_to_timestamp(
                "9999-12-31T23:59:59.0000000+00:00",
                TimestampFormat::Seconds,
                Some("UTC"),
                None
            )
            .unwrap(),
            YEAR9999_UNIX_SECONDS
        );
        assert_eq!(
            datetime_to_timestamp(
                "9999-12-31T23:59:59.9990000+00:00",
                TimestampFormat::Milliseconds,
                Some("UTC"),
                None
            )
            .unwrap(),
            YEAR9999_UNIX_MILLISECONDS
        );
        round_trip(
            YEAR1_UNIX_SECONDS,
            TimestampFormat::Seconds,
            Some("UTC"),
            None,
        );
        round_trip(
            YEAR1_UNIX_MILLISECONDS,
            TimestampFormat::Milliseconds,
            Some("UTC"),
            None,
        );
        round_trip(MAX_TICKS, TimestampFormat::Ticks, Some("UTC"), None);
    }

    #[test]
    fn before_and_after_unix_epoch() {
        assert_eq!(
            timestamp_to_datetime("-1", TimestampFormat::Seconds, Some("UTC"), None).unwrap(),
            "1969-12-31T23:59:59.0000000+00:00"
        );
        assert_eq!(
            timestamp_to_datetime("-1", TimestampFormat::Milliseconds, Some("UTC"), None).unwrap(),
            "1969-12-31T23:59:59.9990000+00:00"
        );
        assert_eq!(
            timestamp_to_datetime("1", TimestampFormat::Seconds, Some("UTC"), None).unwrap(),
            "1970-01-01T00:00:01.0000000+00:00"
        );
        round_trip("-1", TimestampFormat::Seconds, Some("UTC"), None);
        round_trip("-1", TimestampFormat::Milliseconds, Some("UTC"), None);
        round_trip("1", TimestampFormat::Seconds, Some("UTC"), None);
        // C# (long)TimeSpan.TotalSeconds truncates toward zero.
        assert_eq!(
            datetime_to_timestamp(
                "1969-12-31T23:59:59.1000000+00:00",
                TimestampFormat::Seconds,
                Some("UTC"),
                None
            )
            .unwrap(),
            "0"
        );
        assert_eq!(
            datetime_to_timestamp(
                "1970-01-01T00:00:00.9000000+00:00",
                TimestampFormat::Seconds,
                Some("UTC"),
                None
            )
            .unwrap(),
            "0"
        );
        assert_eq!(
            datetime_to_timestamp(
                "1970-01-01T00:00:01.9000000+00:00",
                TimestampFormat::Seconds,
                Some("UTC"),
                None
            )
            .unwrap(),
            "1"
        );
    }

    #[test]
    fn named_timezone_converts_unix_epoch() {
        assert_eq!(
            timestamp_to_datetime(
                "0",
                TimestampFormat::Seconds,
                Some("America/New_York"),
                None
            )
            .unwrap(),
            "1969-12-31T19:00:00.0000000-05:00"
        );
        assert_eq!(
            timestamp_to_datetime("0", TimestampFormat::Seconds, Some("Asia/Shanghai"), None)
                .unwrap(),
            "1970-01-01T08:00:00.0000000+08:00"
        );
        round_trip(
            "0",
            TimestampFormat::Seconds,
            Some("America/New_York"),
            None,
        );
        round_trip(
            "1",
            TimestampFormat::Milliseconds,
            Some("Asia/Shanghai"),
            None,
        );
        round_trip(
            UNIX_EPOCH_TICKS_STR,
            TimestampFormat::Ticks,
            Some("America/New_York"),
            None,
        );
    }

    #[test]
    fn custom_epoch_from_unix_and_datetime() {
        let epoch = "2000-01-01T00:00:00.0000000+00:00";
        assert_eq!(
            timestamp_to_datetime("1", TimestampFormat::Seconds, Some("UTC"), Some(epoch)).unwrap(),
            "2000-01-01T00:00:01.0000000+00:00"
        );
        assert_eq!(
            timestamp_to_datetime(
                "1500",
                TimestampFormat::Milliseconds,
                Some("UTC"),
                Some(epoch)
            )
            .unwrap(),
            "2000-01-01T00:00:01.5000000+00:00"
        );
        assert_eq!(
            datetime_to_timestamp(
                "2000-01-01T00:00:01.5000000+00:00",
                TimestampFormat::Seconds,
                Some("UTC"),
                Some(epoch)
            )
            .unwrap(),
            "1"
        );
        assert_eq!(
            datetime_to_timestamp(
                "2000-01-01T00:00:01.5000000+00:00",
                TimestampFormat::Milliseconds,
                Some("UTC"),
                Some(epoch)
            )
            .unwrap(),
            "1500"
        );
        assert_eq!(
            datetime_to_timestamp(
                "2000-01-01T00:00:01.5000000+00:00",
                TimestampFormat::Ticks,
                Some("UTC"),
                Some(epoch)
            )
            .unwrap(),
            "15000000"
        );
        round_trip(
            "1500",
            TimestampFormat::Milliseconds,
            Some("UTC"),
            Some(epoch),
        );
        round_trip("15000000", TimestampFormat::Ticks, Some("UTC"), Some(epoch));
        round_trip("0", TimestampFormat::Seconds, Some("UTC"), Some("0"));
    }

    #[test]
    fn out_of_range_and_illegal_values_fail() {
        assert_eq!(
            timestamp_to_datetime("-1", TimestampFormat::Ticks, Some("UTC"), None).unwrap_err(),
            DateConvertError::InvalidInput
        );
        assert_eq!(
            timestamp_to_datetime(
                "3155378976000000000",
                TimestampFormat::Ticks,
                Some("UTC"),
                None
            )
            .unwrap_err(),
            DateConvertError::InvalidInput
        );
        assert_eq!(
            datetime_to_timestamp(
                "0000-12-31T23:59:59.0000000+00:00",
                TimestampFormat::Ticks,
                Some("UTC"),
                None
            )
            .unwrap_err(),
            DateConvertError::InvalidInput
        );
        assert_eq!(
            datetime_to_timestamp(
                "10000-01-01T00:00:00.0000000+00:00",
                TimestampFormat::Seconds,
                Some("UTC"),
                None
            )
            .unwrap_err(),
            DateConvertError::InvalidInput
        );
        assert_eq!(
            timestamp_to_datetime("-62135596801", TimestampFormat::Seconds, Some("UTC"), None)
                .unwrap_err(),
            DateConvertError::InvalidInput
        );
        assert_eq!(
            timestamp_to_datetime("0", TimestampFormat::Seconds, Some("Not/AZone"), None)
                .unwrap_err(),
            DateConvertError::UnknownTimezone
        );
        assert_eq!(
            timestamp_to_datetime(
                "0",
                TimestampFormat::Seconds,
                Some("UTC"),
                Some("not-an-epoch")
            )
            .unwrap_err(),
            DateConvertError::InvalidEpoch
        );
    }

    #[test]
    fn invalid_input_is_err_without_echo() {
        let input = "not-a-date-xyz";
        let err =
            datetime_to_timestamp(input, TimestampFormat::Seconds, Some("UTC"), None).unwrap_err();
        assert_eq!(err, DateConvertError::InvalidInput);
        let message = err.to_string();
        assert!(
            !message.contains(input),
            "error must not include user input"
        );
        assert!(!message.contains("xyz"));
    }

    // 2024-07-15T16:30:45.123Z — unix seconds from Python datetime.
    const FROZEN_UNIX_SECONDS: i64 = 1_721_061_045;
    const FROZEN_UNIX_NANOS: u32 = 123_000_000;
    const FROZEN_SECONDS: &str = "1721061045";
    const FROZEN_MILLISECONDS: &str = "1721061045123";
    const FROZEN_TICKS: &str = "638566578451230000";
    const FROZEN_UTC: &str = "2024-07-15T16:30:45.1230000+00:00";
    const FROZEN_NY: &str = "2024-07-15T12:30:45.1230000-04:00";
    const CUSTOM_EPOCH: &str = "2000-01-01T00:00:00Z";
    const FROZEN_CUSTOM_SECONDS: &str = "774376245";
    const FROZEN_CUSTOM_MILLISECONDS: &str = "774376245123";
    const FROZEN_CUSTOM_TICKS: &str = "7743762451230000";

    fn frozen_now() -> DateTime<Utc> {
        DateTime::from_timestamp(FROZEN_UNIX_SECONDS, FROZEN_UNIX_NANOS).unwrap()
    }

    #[test]
    fn frozen_now_writes_timestamp_and_datetime() {
        let now = frozen_now();

        let (ts, dt) = now_values(now, TimestampFormat::Seconds, Some("UTC"), None).unwrap();
        assert_eq!(ts, FROZEN_SECONDS);
        assert_eq!(dt, FROZEN_UTC);

        let (ts, dt) = now_values(now, TimestampFormat::Milliseconds, Some("UTC"), None).unwrap();
        assert_eq!(ts, FROZEN_MILLISECONDS);
        assert_eq!(dt, FROZEN_UTC);

        let (ts, dt) = now_values(now, TimestampFormat::Ticks, Some("UTC"), None).unwrap();
        assert_eq!(ts, FROZEN_TICKS);
        assert_eq!(dt, FROZEN_UTC);

        let (ts, dt) = now_values(
            now,
            TimestampFormat::Seconds,
            Some("UTC"),
            Some(CUSTOM_EPOCH),
        )
        .unwrap();
        assert_eq!(ts, FROZEN_CUSTOM_SECONDS);
        assert_eq!(dt, FROZEN_UTC);

        let (ts, dt) = now_values(
            now,
            TimestampFormat::Milliseconds,
            Some("UTC"),
            Some(CUSTOM_EPOCH),
        )
        .unwrap();
        assert_eq!(ts, FROZEN_CUSTOM_MILLISECONDS);
        assert_eq!(dt, FROZEN_UTC);

        let (ts, dt) =
            now_values(now, TimestampFormat::Ticks, Some("UTC"), Some(CUSTOM_EPOCH)).unwrap();
        assert_eq!(ts, FROZEN_CUSTOM_TICKS);
        assert_eq!(dt, FROZEN_UTC);

        let (ts, dt) = now_values(
            now,
            TimestampFormat::Seconds,
            Some("America/New_York"),
            None,
        )
        .unwrap();
        assert_eq!(ts, FROZEN_SECONDS);
        assert_eq!(dt, FROZEN_NY);
    }

    #[test]
    fn dst_hint_new_york_july_not_shanghai() {
        let july = DateTime::from_timestamp(FROZEN_UNIX_SECONDS, 0).unwrap();
        // 2024-01-15T16:30:45Z
        let january = DateTime::from_timestamp(1_705_336_245, 0).unwrap();
        assert_eq!(
            dst_hint(Some("America/New_York"), july).unwrap(),
            Some("夏令时。")
        );
        assert_eq!(dst_hint(Some("Asia/Shanghai"), july).unwrap(), None);
        assert_eq!(dst_hint(Some("America/New_York"), january).unwrap(), None);
        assert_eq!(dst_hint(Some("Asia/Shanghai"), january).unwrap(), None);
    }

    #[test]
    fn split_fields_changing_seconds_changes_timestamp() {
        let base = DateParts {
            year: 2024,
            month: 7,
            day: 15,
            hour: 16,
            minute: 30,
            second: 45,
        };
        let bumped = DateParts { second: 46, ..base };

        let (ts, dt) = parts_to_values(base, TimestampFormat::Seconds, Some("UTC"), None).unwrap();
        let (ts2, dt2) =
            parts_to_values(bumped, TimestampFormat::Seconds, Some("UTC"), None).unwrap();
        assert_eq!(ts, FROZEN_SECONDS);
        assert_eq!(dt, "2024-07-15T16:30:45.0000000+00:00");
        assert_eq!(ts2, "1721061046");
        assert_eq!(dt2, "2024-07-15T16:30:46.0000000+00:00");
        assert_ne!(ts, ts2);

        let (ts, _) =
            parts_to_values(base, TimestampFormat::Milliseconds, Some("UTC"), None).unwrap();
        let (ts2, _) =
            parts_to_values(bumped, TimestampFormat::Milliseconds, Some("UTC"), None).unwrap();
        assert_eq!(ts, "1721061045000");
        assert_eq!(ts2, "1721061046000");

        let (ts, _) = parts_to_values(base, TimestampFormat::Ticks, Some("UTC"), None).unwrap();
        let (ts2, _) = parts_to_values(bumped, TimestampFormat::Ticks, Some("UTC"), None).unwrap();
        assert_eq!(ts, "638566578450000000");
        assert_eq!(ts2, "638566578460000000");

        let (ts, _) = parts_to_values(
            base,
            TimestampFormat::Seconds,
            Some("UTC"),
            Some(CUSTOM_EPOCH),
        )
        .unwrap();
        let (ts2, _) = parts_to_values(
            bumped,
            TimestampFormat::Seconds,
            Some("UTC"),
            Some(CUSTOM_EPOCH),
        )
        .unwrap();
        assert_eq!(ts, FROZEN_CUSTOM_SECONDS);
        assert_eq!(ts2, "774376246");
    }

    #[test]
    fn timezone_list_covers_original_common_zones() {
        for name in [
            "本机",
            "UTC",
            "America/New_York",
            "America/Los_Angeles",
            "Europe/London",
            "Europe/Paris",
            "Asia/Shanghai",
            "Asia/Tokyo",
            "Australia/Sydney",
        ] {
            assert!(COMMON_TIMEZONES.contains(&name), "missing {name}");
        }
        let from_list = COMMON_TIMEZONES
            .iter()
            .copied()
            .find(|name| *name == "Asia/Shanghai")
            .unwrap();
        assert_eq!(
            timestamp_to_datetime("0", TimestampFormat::Seconds, Some(from_list), None).unwrap(),
            timestamp_to_datetime("0", TimestampFormat::Seconds, Some("Asia/Shanghai"), None)
                .unwrap()
        );
        assert_eq!(
            timestamp_to_datetime("0", TimestampFormat::Seconds, Some("本机"), None).unwrap(),
            timestamp_to_datetime("0", TimestampFormat::Seconds, Some("local"), None).unwrap()
        );
    }
}
