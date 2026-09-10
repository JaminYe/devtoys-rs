use std::str::FromStr;

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike, Weekday};
use cron::Schedule;

pub const DEFAULT_DATE_FORMAT: &str = "yyyy-MM-dd ddd HH:mm:ss";
pub const DEFAULT_EXPR_WITH_SECONDS: &str = "* * * * * *";
pub const DEFAULT_EXPR_WITHOUT_SECONDS: &str = "* * * * *";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CronParseError {
    #[error("非法 Cron 表达式")]
    InvalidExpression,
    #[error("预览条数无效")]
    InvalidCount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronParseResult {
    pub description: String,
    pub next: Vec<String>,
}

impl CronParseResult {
    pub fn display_text(&self) -> String {
        if self.next.is_empty() {
            self.description.clone()
        } else {
            format!(
                "{}\n\n下次执行：\n{}",
                self.description,
                self.next.join("\n")
            )
        }
    }
}

pub fn parse_cron(
    expression: &str,
    include_seconds: bool,
    count: usize,
    date_format: &str,
) -> Result<CronParseResult, CronParseError> {
    parse_cron_after(
        expression,
        include_seconds,
        count,
        date_format,
        &Local::now(),
    )
}

fn parse_cron_after<Tz: TimeZone>(
    expression: &str,
    include_seconds: bool,
    count: usize,
    date_format: &str,
    after: &DateTime<Tz>,
) -> Result<CronParseResult, CronParseError> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Err(CronParseError::InvalidExpression);
    }
    if count == 0 || count > 1000 {
        return Err(CronParseError::InvalidCount);
    }

    let normalized = normalize_expression(expression, include_seconds);
    let schedule =
        Schedule::from_str(&normalized).map_err(|_| CronParseError::InvalidExpression)?;

    let pattern = if date_format.trim().is_empty() {
        DEFAULT_DATE_FORMAT
    } else {
        date_format
    };

    let next = schedule
        .after(after)
        .take(count)
        .map(|dt| apply_date_format(&dt, pattern))
        .collect();

    Ok(CronParseResult {
        description: describe_expression(expression, include_seconds),
        next,
    })
}

fn normalize_expression(expression: &str, include_seconds: bool) -> String {
    let n = expression.split_whitespace().count();
    let with_seconds = if !include_seconds && n == 5 {
        format!("0 {expression}")
    } else {
        expression.to_string()
    };
    remap_unix_weekdays(&with_seconds)
}

/// `cron` 0.15 uses Quartz weekdays (1=Sunday). Map Unix/Cronos numbers first:
/// 0 and 7 = Sunday, 1 = Monday. Names and other fields are left unchanged.
fn remap_unix_weekdays(expression: &str) -> String {
    if expression.starts_with('@') {
        return expression.to_string();
    }
    let mut fields: Vec<String> = expression.split_whitespace().map(str::to_string).collect();
    if fields.len() < 6 {
        return expression.to_string();
    }
    fields[5] = remap_weekday_field(&fields[5]);
    fields.join(" ")
}

fn remap_weekday_field(field: &str) -> String {
    field
        .split(',')
        .map(remap_weekday_item)
        .collect::<Vec<_>>()
        .join(",")
}

fn remap_weekday_item(item: &str) -> String {
    let item = item.trim();
    if item.is_empty() {
        return item.to_string();
    }

    let (base, step) = match item.split_once('/') {
        Some((base, step)) => (base, Some(step)),
        None => (item, None),
    };

    if base == "*" || base == "?" || base.chars().any(|ch| ch.is_ascii_alphabetic()) {
        return item.to_string();
    }

    match expand_unix_weekdays(base, step) {
        Some(days) if !days.is_empty() => days
            .into_iter()
            .map(|day| day.to_string())
            .collect::<Vec<_>>()
            .join(","),
        _ => item.to_string(),
    }
}

fn unix_to_quartz(day: u32) -> Option<u32> {
    match day {
        0 | 7 => Some(1),
        1..=6 => Some(day + 1),
        _ => None,
    }
}

fn expand_unix_weekdays(base: &str, step: Option<&str>) -> Option<Vec<u32>> {
    let step_n: u32 = match step {
        Some(value) => value.parse().ok().filter(|n| *n > 0)?,
        None => 1,
    };

    let (start, end) = if let Some((left, right)) = base.split_once('-') {
        (left.parse::<u32>().ok()?, right.parse::<u32>().ok()?)
    } else {
        let start = base.parse::<u32>().ok()?;
        if step.is_some() {
            (start, 7)
        } else {
            return Some(vec![unix_to_quartz(start)?]);
        }
    };

    if start > end {
        return None;
    }

    let mut days = Vec::new();
    let mut seen = [false; 8];
    let mut current = start;
    while current <= end {
        let quartz = unix_to_quartz(current)?;
        if !seen[quartz as usize] {
            seen[quartz as usize] = true;
            days.push(quartz);
        }
        current = current.saturating_add(step_n);
    }
    Some(days)
}

struct CronParts<'a> {
    seconds: Option<&'a str>,
    minutes: &'a str,
    hours: &'a str,
    dom: &'a str,
    month: &'a str,
    dow: &'a str,
    year: Option<&'a str>,
}

fn describe_expression(expression: &str, include_seconds: bool) -> String {
    let expression = expression.trim();
    if let Some(expanded) = expand_shorthand(expression) {
        return describe_expression(expanded, false);
    }

    let fields: Vec<&str> = expression.split_whitespace().collect();
    let Some(parts) = split_cron_fields(&fields, include_seconds) else {
        // Legal parsers may accept nicknames already handled above; never use a placeholder.
        return "按表达式执行".to_string();
    };

    let time = describe_time(parts.seconds, parts.minutes, parts.hours);
    let day = describe_day(parts.dom, parts.dow);
    let month = describe_month(parts.month);
    let year = parts.year.and_then(describe_year).unwrap_or_default();
    compose_description(&time, &day, &month, &year)
}

/// `cron` 0.15 nicknames. `@weekly` is Sunday (Quartz ordinal 1), same as Unix `0`.
fn expand_shorthand(expression: &str) -> Option<&'static str> {
    match expression {
        "@yearly" => Some("0 0 1 1 *"),
        "@monthly" => Some("0 0 1 * *"),
        "@weekly" => Some("0 0 * * 0"),
        "@daily" => Some("0 0 * * *"),
        "@hourly" => Some("0 * * * *"),
        _ => None,
    }
}

fn split_cron_fields<'a>(fields: &'a [&'a str], _include_seconds: bool) -> Option<CronParts<'a>> {
    match fields.len() {
        5 => Some(CronParts {
            seconds: None,
            minutes: fields[0],
            hours: fields[1],
            dom: fields[2],
            month: fields[3],
            dow: fields[4],
            year: None,
        }),
        6 => Some(CronParts {
            seconds: Some(fields[0]),
            minutes: fields[1],
            hours: fields[2],
            dom: fields[3],
            month: fields[4],
            dow: fields[5],
            year: None,
        }),
        n if n >= 7 => Some(CronParts {
            seconds: Some(fields[0]),
            minutes: fields[1],
            hours: fields[2],
            dom: fields[3],
            month: fields[4],
            dow: fields[5],
            year: Some(fields[6]),
        }),
        _ => None,
    }
}

fn is_unrestricted(field: &str) -> bool {
    field == "*" || field == "?"
}

fn plain_u32(field: &str) -> Option<u32> {
    if field.is_empty() || !field.bytes().all(|b| b.is_ascii_digit()) {
        None
    } else {
        field.parse().ok()
    }
}

fn format_clock(hour: u32, minute: u32, second: Option<u32>) -> String {
    match second {
        Some(s) if s != 0 => format!("{hour:02}:{minute:02}:{s:02}"),
        _ if hour == 0 && minute == 0 => "零点".to_string(),
        _ => format!("{hour:02}:{minute:02}"),
    }
}

fn has_special(field: &str) -> bool {
    field.contains(['*', '/', '-', ',', '?'])
}

fn normalize_star_step(field: &str) -> String {
    if let Some(rest) = field.strip_prefix("0/") {
        format!("*/{rest}")
    } else if field == "*/1" {
        "*".to_string()
    } else {
        field.to_string()
    }
}

fn join_chinese(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{}和{}", parts[0], parts[1]),
        n => format!("{}和{}", parts[..n - 1].join("、"), parts[n - 1]),
    }
}

fn describe_every(field: &str, unit: &str) -> Option<String> {
    if is_unrestricted(field) {
        return Some(format!("每{unit}"));
    }
    let (base, step) = field.split_once('/')?;
    let step_n: u32 = step.parse().ok()?;
    if step_n == 0 {
        return None;
    }
    let every = if step_n == 1 {
        format!("每{unit}")
    } else {
        format!("每 {step_n} {unit}")
    };
    if is_unrestricted(base) || plain_u32(base) == Some(0) {
        Some(every)
    } else if let Some((start, end)) = base.split_once('-') {
        Some(format!("{every}，从 {start} 到 {end}"))
    } else {
        Some(format!("{every}，从 {base} 起"))
    }
}

fn describe_field_items(field: &str, atom: impl Fn(&str) -> Option<String>) -> String {
    let mut parts = Vec::new();
    for item in field.split(',') {
        let item = item.trim();
        if item.is_empty() {
            return String::new();
        }
        let (base, step) = match item.split_once('/') {
            Some((base, step)) => (base, Some(step)),
            None => (item, None),
        };
        let core = if let Some((start, end)) = base.split_once('-') {
            match (atom(start), atom(end)) {
                (Some(a), Some(b)) => format!("{a}到{b}"),
                _ => return String::new(),
            }
        } else {
            match atom(base) {
                Some(value) => value,
                None => return String::new(),
            }
        };
        if let Some(step) = step {
            parts.push(format!("{core}，每隔 {step}"));
        } else {
            parts.push(core);
        }
    }
    join_chinese(&parts)
}

fn describe_time(seconds: Option<&str>, minutes: &str, hours: &str) -> String {
    let seconds = match seconds {
        None | Some("0") => String::new(),
        Some(value) => normalize_star_step(value),
    };
    let minutes = normalize_star_step(minutes);
    let hours = normalize_star_step(hours);

    let specific_seconds = seconds.is_empty() || !has_special(&seconds);
    if specific_seconds && !has_special(&minutes) && !has_special(&hours) {
        if let (Some(hour), Some(minute)) = (plain_u32(&hours), plain_u32(&minutes)) {
            let second = if seconds.is_empty() {
                None
            } else {
                plain_u32(&seconds)
            };
            return format_clock(hour, minute, second);
        }
    }

    if seconds.is_empty()
        && minutes.contains('-')
        && !minutes.contains(',')
        && !minutes.contains('/')
        && !has_special(&hours)
    {
        if let (Some(hour), Some((start, end))) = (plain_u32(&hours), minutes.split_once('-')) {
            if let (Some(start_m), Some(end_m)) = (plain_u32(start), plain_u32(end)) {
                return format!(
                    "{} 到 {} 的每分钟",
                    format_clock(hour, start_m, None),
                    format_clock(hour, end_m, None)
                );
            }
        }
    }

    if seconds.is_empty()
        && hours.contains(',')
        && !hours.contains('-')
        && !hours.contains('/')
        && !has_special(&minutes)
    {
        if let Some(minute) = plain_u32(&minutes) {
            if let Some(clocks) = describe_hour_list_clocks(&hours, minute) {
                return clocks;
            }
        }
    }

    let mut parts = Vec::new();
    if !seconds.is_empty() {
        if let Some(desc) = describe_seconds_field(&seconds) {
            parts.push(desc);
        }
    }
    if let Some(desc) = describe_minutes_field(&minutes, seconds.is_empty()) {
        parts.push(desc);
    }
    if let Some(desc) = describe_hours_field(&hours) {
        parts.push(desc);
    }

    if parts.is_empty() {
        if is_unrestricted(&hours) && plain_u32(&minutes) == Some(0) {
            "每小时".to_string()
        } else {
            "每分钟".to_string()
        }
    } else {
        parts.join("，")
    }
}

fn describe_hour_list_clocks(hours: &str, minute: u32) -> Option<String> {
    let mut clocks = Vec::new();
    for item in hours.split(',') {
        clocks.push(format_clock(plain_u32(item.trim())?, minute, None));
    }
    if clocks.is_empty() {
        None
    } else {
        Some(join_chinese(&clocks))
    }
}

fn hour_range_between(start: &str, end: &str) -> Option<String> {
    let start_h = plain_u32(start)?;
    let end_h = plain_u32(end)?;
    Some(format!("在 {start_h:02}:00 和 {end_h:02}:59 之间"))
}

fn describe_seconds_field(seconds: &str) -> Option<String> {
    if is_unrestricted(seconds) {
        return Some("每秒".to_string());
    }
    if let Some(desc) = describe_every(seconds, "秒") {
        return Some(desc);
    }
    if let Some(n) = plain_u32(seconds) {
        return if n == 0 {
            None
        } else {
            Some(format!("在每分钟的 {n} 秒"))
        };
    }
    if let Some((start, end)) = seconds.split_once('-') {
        if !seconds.contains(',') && !seconds.contains('/') {
            if plain_u32(start).is_some() && plain_u32(end).is_some() {
                return Some(format!("在每分钟的 {start} 到 {end} 秒"));
            }
        }
    }
    let items = describe_field_items(seconds, |tok| plain_u32(tok).map(|n| n.to_string()));
    if items.is_empty() {
        Some(format!("秒 {seconds}"))
    } else {
        Some(format!("在每分钟的 {items} 秒"))
    }
}

fn describe_minutes_field(minutes: &str, seconds_absent: bool) -> Option<String> {
    if is_unrestricted(minutes) {
        return None;
    }
    if seconds_absent && plain_u32(minutes) == Some(0) {
        return None;
    }
    if let Some(desc) = describe_every(minutes, "分钟") {
        return Some(desc);
    }
    if let Some(n) = plain_u32(minutes) {
        return Some(format!("在每小时的 {n} 分"));
    }
    if let Some((start, end)) = minutes.split_once('-') {
        if !minutes.contains(',') && !minutes.contains('/') {
            if plain_u32(start).is_some() && plain_u32(end).is_some() {
                return Some(format!("在每小时的 {start} 到 {end} 分钟"));
            }
        }
    }
    let items = describe_field_items(minutes, |tok| plain_u32(tok).map(|n| format!("{n} 分")));
    if items.is_empty() {
        Some(format!("分钟 {minutes}"))
    } else {
        Some(format!("在每小时的 {items}"))
    }
}

fn describe_hours_field(hours: &str) -> Option<String> {
    if is_unrestricted(hours) {
        return None;
    }
    if let Some(desc) = describe_every(hours, "小时") {
        return Some(desc);
    }
    let mut parts = Vec::new();
    for item in hours.split(',') {
        let item = item.trim();
        if item.is_empty() {
            return Some(format!("小时 {hours}"));
        }
        let (base, step) = match item.split_once('/') {
            Some((base, step)) => (base, Some(step)),
            None => (item, None),
        };
        let core = if let Some((start, end)) = base.split_once('-') {
            hour_range_between(start, end)?
        } else if let Some(hour) = plain_u32(base) {
            format!("在 {}", format_clock(hour, 0, None))
        } else {
            return Some(format!("在 {hours} 点"));
        };
        if let Some(step) = step {
            parts.push(format!("{core}，每隔 {step} 小时"));
        } else {
            parts.push(core);
        }
    }
    Some(join_chinese(&parts))
}

fn weekday_name(token: &str) -> Option<String> {
    Some(
        match token.trim().to_ascii_uppercase().as_str() {
            "0" | "7" | "SUN" => "周日",
            "1" | "MON" => "周一",
            "2" | "TUE" => "周二",
            "3" | "WED" => "周三",
            "4" | "THU" => "周四",
            "5" | "FRI" => "周五",
            "6" | "SAT" => "周六",
            _ => return None,
        }
        .to_string(),
    )
}

fn month_name(token: &str) -> Option<String> {
    let n = match token.trim().to_ascii_uppercase().as_str() {
        "1" | "JAN" => 1,
        "2" | "FEB" => 2,
        "3" | "MAR" => 3,
        "4" | "APR" => 4,
        "5" | "MAY" => 5,
        "6" | "JUN" => 6,
        "7" | "JUL" => 7,
        "8" | "AUG" => 8,
        "9" | "SEP" => 9,
        "10" | "OCT" => 10,
        "11" | "NOV" => 11,
        "12" | "DEC" => 12,
        _ => return None,
    };
    Some(format!("{n} 月"))
}

fn describe_dom(field: &str) -> String {
    if is_unrestricted(field) {
        return String::new();
    }
    if let Some(desc) = describe_every(field, "天") {
        return if desc == "每天" {
            String::new()
        } else {
            desc
        };
    }
    let items = describe_field_items(field, |tok| plain_u32(tok).map(|n| format!("{n} 号")));
    if items.is_empty() {
        format!("日期 {field}")
    } else {
        format!("每月 {items}")
    }
}

fn describe_dow(field: &str) -> String {
    if is_unrestricted(field) {
        return String::new();
    }
    if let Some((base, step)) = field.split_once('/') {
        if is_unrestricted(base) {
            return match step.parse::<u32>().ok().filter(|n| *n > 1) {
                Some(n) => format!("每隔 {n} 天"),
                None => String::new(),
            };
        }
    }
    let items = describe_field_items(field, weekday_name);
    if items.is_empty() {
        format!("星期 {field}")
    } else if items.contains('到') || items.contains('和') || items.contains('、') {
        if items.contains("每隔") {
            format!("{items} 天")
        } else {
            items
        }
    } else if items.contains("每隔") {
        format!("每{items} 天")
    } else {
        format!("每{items}")
    }
}

fn describe_day(dom: &str, dow: &str) -> String {
    let dom_desc = describe_dom(dom);
    let dow_desc = describe_dow(dow);
    match (dom_desc.as_str(), dow_desc.as_str()) {
        ("", "") => "每天".to_string(),
        ("", day) => day.to_string(),
        (day, "") => day.to_string(),
        (dom, dow) => format!("{dom}或{dow}"),
    }
}

fn describe_month(field: &str) -> String {
    if is_unrestricted(field) {
        return String::new();
    }
    if let Some(desc) = describe_every(field, "月") {
        return if desc == "每月" {
            String::new()
        } else {
            desc
        };
    }
    let items = describe_field_items(field, month_name);
    if items.is_empty() {
        format!("月份 {field}")
    } else if items.contains('到') || items.contains('和') || items.contains('、') {
        items
    } else {
        format!("仅在 {items}")
    }
}

fn describe_year(field: &str) -> Option<String> {
    if is_unrestricted(field) {
        return None;
    }
    if let Some(desc) = describe_every(field, "年") {
        return if desc == "每年" { None } else { Some(desc) };
    }
    let items = describe_field_items(field, |tok| plain_u32(tok).map(|n| n.to_string()));
    if items.is_empty() {
        Some(format!("年份 {field}"))
    } else {
        Some(format!("仅在 {items}"))
    }
}

fn compose_description(time: &str, day: &str, month: &str, year: &str) -> String {
    let head = attach_day_and_time(day, time);
    let mut parts = vec![head];
    if !month.is_empty() {
        parts.push(month.to_string());
    }
    if !year.is_empty() {
        parts.push(year.to_string());
    }
    parts.join("，")
}

fn attach_day_and_time(day: &str, time: &str) -> String {
    if day.is_empty() {
        return time.to_string();
    }
    if time.starts_with('每') {
        if day == "每天" {
            time.to_string()
        } else {
            format!("{day}，{time}")
        }
    } else if time == "零点" || time == "午夜" {
        if day.starts_with('每')
            && !day.contains('和')
            && !day.contains('到')
            && !day.contains('、')
            && !day.contains('，')
        {
            format!("{day}{time}")
        } else {
            format!("{day} {time}")
        }
    } else if time.starts_with("在 ") {
        format!("{day}，{time}")
    } else {
        format!("{day} {time}")
    }
}

fn weekday_ddd<Tz: TimeZone>(dt: &DateTime<Tz>) -> &'static str {
    match dt.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

fn apply_date_format<Tz: TimeZone>(dt: &DateTime<Tz>, pattern: &str) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix("yyyy") {
            out.push_str(&format!("{:04}", dt.year()));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("ddd") {
            out.push_str(weekday_ddd(dt));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("MM") {
            out.push_str(&format!("{:02}", dt.month()));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("dd") {
            out.push_str(&format!("{:02}", dt.day()));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("HH") {
            out.push_str(&format!("{:02}", dt.hour()));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("mm") {
            out.push_str(&format!("{:02}", dt.minute()));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("ss") {
            out.push_str(&format!("{:02}", dt.second()));
            rest = tail;
        } else {
            let mut chars = rest.chars();
            if let Some(ch) = chars.next() {
                out.push(ch);
            }
            rest = chars.as_str();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    /// Wednesday 2024-01-03 12:00:00 UTC. Independent of wall-clock "now".
    fn start() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 3, 12, 0, 0).unwrap()
    }

    fn next_runs(expression: &str, include_seconds: bool, count: usize) -> Vec<String> {
        parse_cron_after(
            expression,
            include_seconds,
            count,
            DEFAULT_DATE_FORMAT,
            &start(),
        )
        .unwrap()
        .next
    }

    fn parsed(expression: &str, include_seconds: bool, count: usize) -> CronParseResult {
        parse_cron_after(
            expression,
            include_seconds,
            count,
            DEFAULT_DATE_FORMAT,
            &start(),
        )
        .unwrap()
    }

    #[test]
    fn six_field_produces_non_empty_next_times() {
        let got = parse_cron("* * * * * *", true, 5, DEFAULT_DATE_FORMAT).unwrap();
        assert!(!got.description.is_empty());
        assert_eq!(got.next.len(), 5);
        assert!(got.next.iter().all(|item| !item.is_empty()));
    }

    #[test]
    fn five_field_produces_non_empty_next_times() {
        let got = parse_cron("* * * * *", false, 5, DEFAULT_DATE_FORMAT).unwrap();
        assert!(!got.description.is_empty());
        assert_eq!(got.next.len(), 5);
        assert!(got.next.iter().all(|item| !item.is_empty()));
    }

    #[test]
    fn illegal_expression_is_err_without_echoing_input() {
        let expr = "not-a-cron-xyz";
        let err = parse_cron(expr, true, 5, DEFAULT_DATE_FORMAT).unwrap_err();
        assert_eq!(err, CronParseError::InvalidExpression);
        let message = err.to_string();
        assert!(!message.contains(expr), "error must not include user input");
        assert!(!message.contains("not-a-cron"));
    }

    #[test]
    fn date_pattern_maps_tokens() {
        let dt = Utc.with_ymd_and_hms(2020, 1, 2, 3, 4, 5).unwrap();
        assert_eq!(
            apply_date_format(&dt, "yyyy-MM-dd ddd HH:mm:ss"),
            "2020-01-02 Thu 03:04:05"
        );
    }

    #[test]
    fn numeric_weekday_1_is_monday() {
        assert_eq!(
            next_runs("0 0 * * 1", false, 3),
            [
                "2024-01-08 Mon 00:00:00",
                "2024-01-15 Mon 00:00:00",
                "2024-01-22 Mon 00:00:00",
            ]
        );
    }

    #[test]
    fn numeric_weekday_0_and_7_are_sunday() {
        let sundays = [
            "2024-01-07 Sun 00:00:00",
            "2024-01-14 Sun 00:00:00",
            "2024-01-21 Sun 00:00:00",
        ];
        assert_eq!(next_runs("0 0 * * 0", false, 3), sundays);
        assert_eq!(next_runs("0 0 * * 7", false, 3), sundays);
        assert_eq!(next_runs("0 0 * * 0,7", false, 3), sundays);
    }

    #[test]
    fn weekday_names_match_numbers() {
        assert_eq!(
            next_runs("0 0 * * 1", false, 4),
            next_runs("0 0 * * MON", false, 4)
        );
        assert_eq!(
            next_runs("0 0 * * 0", false, 4),
            next_runs("0 0 * * SUN", false, 4)
        );
        assert_eq!(
            next_runs("0 0 * * 7", false, 4),
            next_runs("0 0 * * SUN", false, 4)
        );
        assert_eq!(
            next_runs("0 0 * * 1-5", false, 5),
            next_runs("0 0 * * MON-FRI", false, 5)
        );
    }

    #[test]
    fn weekday_lists_ranges_and_steps() {
        assert_eq!(
            next_runs("0 0 * * 1,5", false, 4),
            [
                "2024-01-05 Fri 00:00:00",
                "2024-01-08 Mon 00:00:00",
                "2024-01-12 Fri 00:00:00",
                "2024-01-15 Mon 00:00:00",
            ]
        );
        assert_eq!(
            next_runs("0 0 * * 1-5", false, 5),
            [
                "2024-01-04 Thu 00:00:00",
                "2024-01-05 Fri 00:00:00",
                "2024-01-08 Mon 00:00:00",
                "2024-01-09 Tue 00:00:00",
                "2024-01-10 Wed 00:00:00",
            ]
        );
        assert_eq!(
            next_runs("0 0 * * 1-5/2", false, 4),
            [
                "2024-01-05 Fri 00:00:00",
                "2024-01-08 Mon 00:00:00",
                "2024-01-10 Wed 00:00:00",
                "2024-01-12 Fri 00:00:00",
            ]
        );
        assert_eq!(
            next_runs("0 0 * * 6-7", false, 3),
            [
                "2024-01-06 Sat 00:00:00",
                "2024-01-07 Sun 00:00:00",
                "2024-01-13 Sat 00:00:00",
            ]
        );
        assert_eq!(
            next_runs("0 0 * * */2", false, 4),
            [
                "2024-01-04 Thu 00:00:00",
                "2024-01-06 Sat 00:00:00",
                "2024-01-07 Sun 00:00:00",
                "2024-01-09 Tue 00:00:00",
            ]
        );
    }

    #[test]
    fn seconds_on_and_off_agree_for_weekdays() {
        let without_seconds = next_runs("0 0 * * 1", false, 3);
        let with_seconds = next_runs("0 0 0 * * 1", true, 3);
        assert_eq!(without_seconds, with_seconds);
        assert_eq!(
            next_runs("0 0 0 * * 0", true, 2),
            ["2024-01-07 Sun 00:00:00", "2024-01-14 Sun 00:00:00",]
        );
        assert_eq!(
            next_runs("0 0 0 * * 7", true, 2),
            next_runs("0 0 0 * * SUN", true, 2)
        );
    }

    #[test]
    fn week_boundary_saturday_night_to_sunday() {
        let saturday_night = Utc.with_ymd_and_hms(2024, 1, 6, 23, 0, 0).unwrap();
        let got =
            parse_cron_after("0 0 * * 0", false, 1, DEFAULT_DATE_FORMAT, &saturday_night).unwrap();
        assert_eq!(got.next, ["2024-01-07 Sun 00:00:00"]);
    }

    #[test]
    fn description_keeps_user_weekday_and_matches_preview() {
        let monday = parsed("0 0 * * 1", false, 1);
        assert!(monday.description.contains("周一"));
        assert!(!monday.description.contains("周日"));
        assert!(monday.next[0].contains("Mon"));
        assert!(!monday.next[0].contains("Sun"));

        let sunday = parsed("0 0 * * 0", false, 1);
        assert!(sunday.description.contains("周日"));
        assert!(sunday.next[0].contains("Sun"));
    }

    fn assert_daily_midnight(description: &str) {
        assert!(description.contains("每天"), "{description}");
        assert!(
            description.contains("零点")
                || description.contains("午夜")
                || description.contains("00:00")
                || description.contains("0:00"),
            "{description}"
        );
        assert!(!description.contains("Cron 计划"), "{description}");
    }

    #[test]
    fn human_description_daily_midnight_without_seconds() {
        let got = parse_cron("0 0 * * *", false, 5, DEFAULT_DATE_FORMAT).unwrap();
        assert_daily_midnight(&got.description);
        assert_eq!(got.next.len(), 5);
        assert!(!got.display_text().contains("计划："));
    }

    #[test]
    fn human_description_unix_weekdays_not_swapped() {
        let monday = parse_cron("0 0 * * 1", false, 3, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            monday.description.contains("周一"),
            "{}",
            monday.description
        );
        assert!(
            !monday.description.contains("周日"),
            "{}",
            monday.description
        );
        assert_eq!(monday.next.len(), 3);

        let sunday0 = parse_cron("0 0 * * 0", false, 3, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            sunday0.description.contains("周日"),
            "{}",
            sunday0.description
        );
        assert_eq!(sunday0.next.len(), 3);

        let sunday7 = parse_cron("0 0 * * 7", false, 3, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            sunday7.description.contains("周日"),
            "{}",
            sunday7.description
        );
        assert_eq!(sunday7.next.len(), 3);
    }

    #[test]
    fn human_description_handles_star_step_range_list() {
        let every_min = parse_cron("* * * * *", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            every_min.description.contains("每分钟"),
            "{}",
            every_min.description
        );

        let every_five = parse_cron("*/5 * * * *", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            every_five.description.contains("5"),
            "{}",
            every_five.description
        );
        assert!(
            every_five.description.contains("分钟"),
            "{}",
            every_five.description
        );

        let range = parse_cron("0 0 * * 1-5", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert!(range.description.contains("周一"), "{}", range.description);
        assert!(range.description.contains("周五"), "{}", range.description);
        assert!(!range.description.contains("周日"), "{}", range.description);

        let list = parse_cron("0 0 * * 1,5", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert!(list.description.contains("周一"), "{}", list.description);
        assert!(list.description.contains("周五"), "{}", list.description);
        assert!(!list.description.contains("周日"), "{}", list.description);
    }

    #[test]
    fn human_description_weekday_names_and_seconds_field() {
        let monday = parse_cron("0 0 * * MON", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            monday.description.contains("周一"),
            "{}",
            monday.description
        );
        assert!(
            !monday.description.contains("周日"),
            "{}",
            monday.description
        );

        let sunday = parse_cron("0 0 * * SUN", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            sunday.description.contains("周日"),
            "{}",
            sunday.description
        );

        let with_seconds = parse_cron("0 0 0 * * 1", true, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            with_seconds.description.contains("周一"),
            "{}",
            with_seconds.description
        );
        assert!(
            !with_seconds.description.contains("周日"),
            "{}",
            with_seconds.description
        );
        assert_eq!(with_seconds.next.len(), 1);

        let midnight_sec = parse_cron("0 0 0 * * *", true, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert_daily_midnight(&midnight_sec.description);
    }

    #[test]
    fn other_fields_are_not_remapped_as_weekdays() {
        assert_eq!(
            next_runs("0 0 15 * *", false, 2),
            ["2024-01-15 Mon 00:00:00", "2024-02-15 Thu 00:00:00",]
        );
        assert_eq!(
            next_runs("30 1 * * *", false, 2),
            ["2024-01-04 Thu 01:30:00", "2024-01-05 Fri 01:30:00",]
        );
    }

    #[test]
    fn illegal_weekday_and_fields_still_error() {
        assert_eq!(
            parse_cron_after("0 0 * * 8", false, 5, DEFAULT_DATE_FORMAT, &start()).unwrap_err(),
            CronParseError::InvalidExpression
        );
        assert_eq!(
            parse_cron_after("0 0 * * 9", false, 5, DEFAULT_DATE_FORMAT, &start()).unwrap_err(),
            CronParseError::InvalidExpression
        );
        assert_eq!(
            parse_cron("", false, 5, DEFAULT_DATE_FORMAT).unwrap_err(),
            CronParseError::InvalidExpression
        );
        assert_eq!(
            parse_cron("0 0 * * *", false, 0, DEFAULT_DATE_FORMAT).unwrap_err(),
            CronParseError::InvalidCount
        );
    }

    fn weekday_mentions(description: &str, names: &[&str]) -> bool {
        names.iter().any(|name| description.contains(name))
    }

    fn assert_readable(description: &str) {
        assert!(!description.is_empty(), "{description}");
        assert!(
            !description.contains("Cron 计划"),
            "placeholder description: {description}"
        );
    }

    #[test]
    fn human_description_quality_seconds_off() {
        let monday = parse_cron("0 0 * * 1", false, 3, DEFAULT_DATE_FORMAT).unwrap();
        assert_readable(&monday.description);
        assert!(
            weekday_mentions(&monday.description, &["周一", "星期一"]),
            "{}",
            monday.description
        );
        assert!(
            !weekday_mentions(&monday.description, &["周日", "星期日"]),
            "{}",
            monday.description
        );
        assert!(
            monday.description.contains("零点") || monday.description.contains("00:00"),
            "{}",
            monday.description
        );
        assert_eq!(monday.next.len(), 3);
        assert!(monday.display_text().starts_with(&monday.description));

        let sunday0 = parse_cron("0 0 * * 0", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert_readable(&sunday0.description);
        assert!(
            weekday_mentions(&sunday0.description, &["周日", "星期日"]),
            "{}",
            sunday0.description
        );

        let sunday7 = parse_cron("0 0 * * 7", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert_readable(&sunday7.description);
        assert!(
            weekday_mentions(&sunday7.description, &["周日", "星期日"]),
            "{}",
            sunday7.description
        );

        let every_five = parse_cron("*/5 * * * *", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert_readable(&every_five.description);
        assert!(
            every_five.description.contains('5') && every_five.description.contains("分钟"),
            "{}",
            every_five.description
        );

        let workdays = parse_cron("0 9-17 * * 1-5", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert_readable(&workdays.description);
        assert!(
            weekday_mentions(&workdays.description, &["周一", "星期一"]),
            "{}",
            workdays.description
        );
        assert!(
            weekday_mentions(&workdays.description, &["周五", "星期五"]),
            "{}",
            workdays.description
        );
        assert!(
            workdays.description.contains('9') && workdays.description.contains("17"),
            "{}",
            workdays.description
        );

        let list = parse_cron("0 0 * * 1,3,5", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert_readable(&list.description);
        assert!(list.description.contains("周一"), "{}", list.description);
        assert!(
            list.description.contains("周三") || list.description.contains("周五"),
            "{}",
            list.description
        );

        let step = parse_cron("0/15 * * * *", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert_readable(&step.description);
        assert!(
            step.description.contains("15") && step.description.contains("分钟"),
            "{}",
            step.description
        );

        let weekday_step = parse_cron("0 0 * * 1-5/2", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert_readable(&weekday_step.description);
        assert!(
            weekday_mentions(&weekday_step.description, &["周一", "星期一"]),
            "{}",
            weekday_step.description
        );
    }

    #[test]
    fn legal_shorthand_is_not_placeholder() {
        for expr in ["@daily", "@hourly", "@weekly", "@monthly", "@yearly"] {
            let got = parse_cron(expr, false, 1, DEFAULT_DATE_FORMAT).unwrap();
            assert_readable(&got.description);
            assert_eq!(got.next.len(), 1);
        }
        let weekly = parse_cron("@weekly", false, 1, DEFAULT_DATE_FORMAT).unwrap();
        assert!(
            weekday_mentions(&weekly.description, &["周日", "星期日"]),
            "{}",
            weekly.description
        );
    }

    #[test]
    fn illegal_expression_has_no_description() {
        let err = parse_cron("not-a-cron-xyz", false, 5, DEFAULT_DATE_FORMAT).unwrap_err();
        assert_eq!(err, CronParseError::InvalidExpression);
        let message = err.to_string();
        assert!(!message.contains("Cron 计划"));
        assert!(!message.contains("零点"));
        assert!(!message.contains("每周"));
    }
}
