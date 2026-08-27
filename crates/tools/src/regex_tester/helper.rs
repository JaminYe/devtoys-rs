use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use regex::{Captures, Regex, RegexBuilder};

/// Match budget. Nested quantifiers and huge DFAs must fail instead of hanging.
const MATCH_TIMEOUT: Duration = Duration::from_millis(100);
/// Cap compiled DFA cache so a pathological pattern cannot grow without bound.
const DFA_SIZE_LIMIT: usize = 2 * 1024 * 1024;
/// Parser nest limit (groups / concatenations).
const NEST_LIMIT: u32 = 50;

#[derive(Debug, Clone)]
pub struct RegexOptions {
    pub all_matches: bool,
    pub ignore_case: bool,
    /// Verbose / `x` flag → [`RegexBuilder::ignore_whitespace`].
    pub ignore_whitespace: bool,
    /// `s` flag → [`RegexBuilder::dot_matches_new_line`].
    pub singleline: bool,
    pub multiline: bool,
    /// Kept for GUI parity with .NET; the `regex` crate has no ECMAScript mode.
    pub ecmascript: bool,
    /// Kept for GUI parity; the `regex` crate is always Unicode / culture-invariant.
    pub culture_invariant: bool,
    /// Kept for GUI parity; the `regex` crate has no right-to-left search.
    pub right_to_left: bool,
}

impl Default for RegexOptions {
    fn default() -> Self {
        Self {
            all_matches: true,
            ignore_case: false,
            ignore_whitespace: false,
            singleline: false,
            multiline: false,
            ecmascript: false,
            culture_invariant: false,
            right_to_left: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureGroup {
    pub index: usize,
    pub name: Option<String>,
    pub start: usize,
    pub end: usize,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexMatch {
    pub index: usize,
    pub start: usize,
    pub end: usize,
    pub value: String,
    pub groups: Vec<CaptureGroup>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RegexTesterError {
    #[error("非法正则")]
    InvalidPattern,
    #[error("匹配超时")]
    Timeout,
}

/// Common tokens for the cheat sheet (syntax, Chinese description).
pub const CHEAT_SHEET: &[(&str, &str)] = &[
    (".", "任意字符（不含换行，除非 Singleline）"),
    ("\\d", "数字"),
    ("\\D", "非数字"),
    ("\\w", "单词字符"),
    ("\\W", "非单词字符"),
    ("\\s", "空白"),
    ("\\S", "非空白"),
    ("^", "行首（Multiline 时每行）"),
    ("$", "行尾（Multiline 时每行）"),
    ("\\b", "单词边界"),
    ("\\B", "非单词边界"),
    ("[abc]", "字符集"),
    ("[^abc]", "否定字符集"),
    ("[a-z]", "字符范围"),
    ("*", "0 次或多次"),
    ("+", "1 次或多次"),
    ("?", "0 次或 1 次"),
    ("{n,m}", "重复 n 到 m 次"),
    ("|", "或"),
    ("(abc)", "捕获分组"),
    ("(?:abc)", "非捕获分组"),
    ("(?<name>abc)", "命名分组"),
];

/// Test `pattern` against `text`. Spans are byte offsets into `text`.
///
/// Matching runs on a worker thread with [`MATCH_TIMEOUT`]. The `regex` crate
/// is linear-time, so nested quantifiers will not ReDoS it; they are still
/// refused as 匹配超时 so a backtracking-style expression never occupies the
/// UI or test process. Compile also uses `dfa_size_limit` / `nest_limit`.
pub fn test_regex(
    pattern: &str,
    text: &str,
    options: &RegexOptions,
) -> Result<Vec<RegexMatch>, RegexTesterError> {
    if pattern.is_empty() {
        return Ok(Vec::new());
    }
    let re = build_regex(pattern, options)?;
    if has_nested_quantifiers(pattern) {
        return Err(RegexTesterError::Timeout);
    }
    let haystack = text.to_string();
    let all = options.all_matches;
    run_timed(MATCH_TIMEOUT, move || collect_matches(&re, &haystack, all))
}

fn build_regex(pattern: &str, options: &RegexOptions) -> Result<Regex, RegexTesterError> {
    RegexBuilder::new(pattern)
        .case_insensitive(options.ignore_case)
        .ignore_whitespace(options.ignore_whitespace)
        .dot_matches_new_line(options.singleline)
        .multi_line(options.multiline)
        .dfa_size_limit(DFA_SIZE_LIMIT)
        .nest_limit(NEST_LIMIT)
        .build()
        .map_err(|_| RegexTesterError::InvalidPattern)
}

fn collect_matches(re: &Regex, text: &str, all_matches: bool) -> Vec<RegexMatch> {
    let mut out = Vec::new();
    for (i, caps) in re.captures_iter(text).enumerate() {
        out.push(extract_match(i, re, &caps));
        if !all_matches {
            break;
        }
    }
    out
}

fn extract_match(index: usize, re: &Regex, caps: &Captures<'_>) -> RegexMatch {
    let full = caps.get(0).expect("group 0 always present");
    let names: Vec<Option<&str>> = re.capture_names().collect();
    let mut groups = Vec::new();
    for (idx, cap) in caps.iter().enumerate().skip(1) {
        let Some(m) = cap else {
            continue;
        };
        groups.push(CaptureGroup {
            index: idx,
            name: names.get(idx).copied().flatten().map(str::to_string),
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
        });
    }
    RegexMatch {
        index,
        start: full.start(),
        end: full.end(),
        value: full.as_str().to_string(),
        groups,
    }
}

fn run_timed<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, RegexTesterError> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("regex-tester".into())
        .spawn(move || {
            let _ = tx.send(work());
        })
        .map_err(|_| RegexTesterError::Timeout)?;
    rx.recv_timeout(timeout).map_err(|_| RegexTesterError::Timeout)
}

/// Nested quantifiers such as `(a+)+` that explode on a backtracking engine.
fn has_nested_quantifiers(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b')' {
            continue;
        }
        let Some(&next) = bytes.get(i + 1) else {
            continue;
        };
        if !is_quantifier(next) {
            continue;
        }
        if let Some(open) = pattern[..i].rfind('(') {
            let inner = &pattern[open + 1..i];
            if inner.bytes().any(is_quantifier) || inner.contains('{') {
                return true;
            }
        }
    }
    false
}

fn is_quantifier(b: u8) -> bool {
    matches!(b, b'+' | b'*' | b'?')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_group_one_is_digits() {
        let matches = test_regex(r"(\d+)", "a12b", &RegexOptions::default()).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].groups.len(), 1);
        assert_eq!(matches[0].groups[0].index, 1);
        assert_eq!(matches[0].groups[0].value, "12");
    }

    #[test]
    fn invalid_pattern_is_err_without_input() {
        let err = test_regex("(", "a12b", &RegexOptions::default()).unwrap_err();
        assert_eq!(err, RegexTesterError::InvalidPattern);
        let message = err.to_string();
        assert!(!message.contains('('), "error must not include user input");
    }

    #[test]
    fn nested_quantifier_times_out() {
        let err = test_regex(
            r"(a+)+$",
            "aaaaaaaaaaaaaaaaaaaaab",
            &RegexOptions::default(),
        )
        .unwrap_err();
        assert_eq!(err, RegexTesterError::Timeout);
        assert_eq!(err.to_string(), "匹配超时");
    }

    #[test]
    fn cheat_sheet_lists_digit() {
        assert!(CHEAT_SHEET.iter().any(|(s, _)| *s == "\\d"));
    }
}
