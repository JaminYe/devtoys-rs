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
            format!("{}\n\n下次执行：\n{}", self.description, self.next.join("\n"))
        }
    }
}

pub fn parse_cron(
    expression: &str,
    include_seconds: bool,
    count: usize,
    date_format: &str,
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
        .upcoming(Local)
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
    if !include_seconds && n == 5 {
        format!("0 {expression}")
    } else {
        expression.to_string()
    }
}

fn describe_expression(expression: &str, include_seconds: bool) -> String {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    let labels: &[&str] = match fields.len() {
        5 => &["分", "时", "日", "月", "周"],
        6 => &["秒", "分", "时", "日", "月", "周"],
        n if n >= 7 => &["秒", "分", "时", "日", "月", "周", "年"],
        _ if include_seconds => &["秒", "分", "时", "日", "月", "周"],
        _ => &["分", "时", "日", "月", "周"],
    };

    let parts: Vec<String> = labels
        .iter()
        .zip(fields.iter())
        .map(|(label, field)| describe_field(label, field))
        .collect();

    if parts.is_empty() {
        "Cron 计划".to_string()
    } else {
        format!("计划：{}", parts.join("，"))
    }
}

fn describe_field(label: &str, field: &str) -> String {
    if field == "*" {
        format!("每{label}")
    } else if let Some(step) = field.strip_prefix("*/") {
        format!("每 {step} {label}")
    } else {
        format!("{label}={field}")
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
}
