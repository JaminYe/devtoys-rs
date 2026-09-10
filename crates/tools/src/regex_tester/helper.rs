use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use regex::{Captures, Regex, RegexBuilder};

/// Match budget. Pathological compiles or matches fail instead of hanging.
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
}

impl Default for RegexOptions {
    fn default() -> Self {
        Self {
            all_matches: true,
            ignore_case: false,
            ignore_whitespace: false,
            singleline: false,
            multiline: false,
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
    /// ASCII `(` is forbidden here: callers must not echo the user pattern.
    #[error("非法或不支持的正则语法，Rust regex 引擎，非 .NET")]
    InvalidPattern,
    #[error("匹配超时")]
    Timeout,
    /// Must not echo the user replacement string.
    #[error("非法替换式")]
    InvalidReplacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchRowKind {
    Match,
    Group,
}

/// One grid row: a whole match or one of its capture groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTableRow {
    pub kind: MatchRowKind,
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Flatten matches into table rows (match row, then its groups). Empty input → empty vec.
pub fn match_table_rows(matches: &[RegexMatch]) -> Vec<MatchTableRow> {
    let mut rows = Vec::new();
    for m in matches {
        rows.push(MatchTableRow {
            kind: MatchRowKind::Match,
            label: format!("匹配 {}", m.index + 1),
            start: m.start,
            end: m.end,
            text: m.value.clone(),
        });
        for g in &m.groups {
            let label = match &g.name {
                Some(name) => format!("分组 {name}"),
                None => format!("分组 {}", g.index),
            };
            rows.push(MatchTableRow {
                kind: MatchRowKind::Group,
                label,
                start: g.start,
                end: g.end,
                text: g.value.clone(),
            });
        }
    }
    rows
}

/// One-line engine boundary shown in the GUI. Always Unicode / culture-invariant.
pub const ENGINE_NOTE: &str = "引擎：Rust regex（非 .NET）；始终 Unicode / 区域不变";

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
/// Matching runs on a worker thread with [`MATCH_TIMEOUT`]. Compile uses
/// `dfa_size_limit` / `nest_limit`. Nested quantifiers are executed for real;
/// timeout is only returned when the budget is actually exceeded.
pub fn test_regex(
    pattern: &str,
    text: &str,
    options: &RegexOptions,
) -> Result<Vec<RegexMatch>, RegexTesterError> {
    if pattern.is_empty() {
        return Ok(Vec::new());
    }
    let re = build_regex(pattern, options)?;
    let haystack = text.to_string();
    let all = options.all_matches;
    run_timed(MATCH_TIMEOUT, move || collect_matches(&re, &haystack, all))
}

/// Pattern the GUI should send to the engine, or `None` to skip.
///
/// Only a truly empty string is empty input. Do not trim: `" "` is a valid
/// expression. `ignore_whitespace` is an engine flag, not a reason to strip.
pub fn gui_effective_pattern(pattern: &str) -> Option<&str> {
    if pattern.is_empty() {
        None
    } else {
        Some(pattern)
    }
}

/// GUI rematch path. `Ok(None)` is empty input; `Ok(Some(vec![]))` is no match;
/// `Err` is invalid / timeout. Distinct from [`test_regex`], which maps empty
/// pattern to an empty match list.
pub fn evaluate_gui_pattern(
    pattern: &str,
    text: &str,
    options: &RegexOptions,
) -> Result<Option<Vec<RegexMatch>>, RegexTesterError> {
    match gui_effective_pattern(pattern) {
        None => Ok(None),
        Some(pat) => test_regex(pat, text, options).map(Some),
    }
}

/// Replace matches of `pattern` in `text` using `replacement`.
///
/// Replacement is not a regex. Supported interpolations are those of
/// [`Captures::expand`]: `$0` whole match, `$1` numbered, `$name` / `${name}`
/// named. `$$` is a literal `$`. `\n` `\t` `\\` in the replacement become those
/// characters; any other `\` escape is [`RegexTesterError::InvalidReplacement`].
/// `all_matches=false` replaces only the first match.
pub fn substitute(
    pattern: &str,
    text: &str,
    replacement: &str,
    options: &RegexOptions,
) -> Result<String, RegexTesterError> {
    if pattern.is_empty() {
        return Ok(text.to_string());
    }
    let re = build_regex(pattern, options)?;
    let prepared = prepare_replacement(replacement)?;
    let haystack = text.to_string();
    let all = options.all_matches;
    run_timed(MATCH_TIMEOUT, move || {
        if all {
            re.replace_all(&haystack, prepared.as_str()).into_owned()
        } else {
            re.replace(&haystack, prepared.as_str()).into_owned()
        }
    })
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

/// Interpret `\n` `\t` `\\` and reject other backslash escapes / unclosed `${`.
fn prepare_replacement(input: &str) -> Result<String, RegexTesterError> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(_) | None => return Err(RegexTesterError::InvalidReplacement),
        }
    }
    if has_unclosed_named_dollar(&out) {
        return Err(RegexTesterError::InvalidReplacement);
    }
    Ok(out)
}

fn has_unclosed_named_dollar(s: &str) -> bool {
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            continue;
        }
        match chars.peek() {
            Some('$') => {
                chars.next();
            }
            Some('{') => {
                chars.next();
                let mut closed = false;
                for inner in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
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
    rx.recv_timeout(timeout)
        .map_err(|_| RegexTesterError::Timeout)
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
        let lower = message.to_ascii_lowercase();
        assert!(
            lower.contains("rust") && lower.contains("regex"),
            "error must mention the Rust regex engine: {message}"
        );
        assert!(
            message.contains("不支持") || message.contains("非法"),
            "error must indicate unsupported or invalid syntax: {message}"
        );
    }

    #[test]
    fn nested_quantifier_matches() {
        let matches = test_regex(r"(a+)+$", "aaa", &RegexOptions::default()).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "aaa");
    }

    #[test]
    fn nested_quantifier_no_match_is_empty_not_timeout() {
        let matches = test_regex(r"(a+)+$", "aaab", &RegexOptions::default()).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn ignore_case_matches_uppercase() {
        let options = RegexOptions {
            ignore_case: true,
            ..Default::default()
        };
        let matches = test_regex("a", "A", &options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "A");
    }

    #[test]
    fn multiline_caret_matches_after_newline() {
        let options = RegexOptions {
            multiline: true,
            ..Default::default()
        };
        let matches = test_regex("^b", "a\nb", &options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "b");
    }

    #[test]
    fn singleline_dot_matches_newline() {
        let options = RegexOptions {
            singleline: true,
            ..Default::default()
        };
        let matches = test_regex("a.b", "a\nb", &options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "a\nb");
    }

    #[test]
    fn ignore_whitespace_ignores_spaces_in_pattern() {
        let options = RegexOptions {
            ignore_whitespace: true,
            ..Default::default()
        };
        let matches = test_regex("a b", "ab", &options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "ab");
    }

    #[test]
    fn all_matches_false_returns_only_first() {
        let options = RegexOptions {
            all_matches: false,
            ..Default::default()
        };
        let matches = test_regex(r"\d+", "12 34 56", &options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "12");
    }

    #[test]
    fn default_all_matches_is_true() {
        assert!(RegexOptions::default().all_matches);
        let matches = test_regex(r"\d+", "12 34 56", &RegexOptions::default()).unwrap();
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn run_timed_times_out_when_work_exceeds_budget() {
        let err = run_timed(Duration::from_millis(50), || {
            thread::sleep(Duration::from_millis(200));
        })
        .unwrap_err();
        assert_eq!(err, RegexTesterError::Timeout);
        assert_eq!(err.to_string(), "匹配超时");
    }

    #[test]
    fn run_timed_ok_when_work_finishes_in_budget() {
        let value = run_timed(Duration::from_millis(200), || 7).unwrap();
        assert_eq!(value, 7);
    }

    #[test]
    fn cheat_sheet_lists_digit() {
        assert!(CHEAT_SHEET.iter().any(|(s, _)| *s == "\\d"));
    }

    #[test]
    fn gui_effective_pattern_empty_string_vs_spaces() {
        assert_eq!(gui_effective_pattern(""), None);
        assert_eq!(gui_effective_pattern(" "), Some(" "));
        assert_eq!(gui_effective_pattern("   "), Some("   "));
        assert_eq!(gui_effective_pattern("\t"), Some("\t"));
        assert_eq!(gui_effective_pattern(" a"), Some(" a"));
    }

    #[test]
    fn gui_space_pattern_matches_sample_space_when_not_ignoring_whitespace() {
        let options = RegexOptions {
            ignore_whitespace: false,
            ..Default::default()
        };
        assert_eq!(gui_effective_pattern(" "), Some(" "));
        let matches = evaluate_gui_pattern(" ", "a b", &options)
            .unwrap()
            .expect("space pattern must reach the engine");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start, 1);
        assert_eq!(matches[0].end, 2);
        assert_eq!(matches[0].value, " ");
    }

    #[test]
    fn gui_space_pattern_ignore_whitespace_reaches_engine() {
        let options = RegexOptions {
            ignore_whitespace: true,
            ..Default::default()
        };
        assert_eq!(gui_effective_pattern(" "), Some(" "));
        let result = evaluate_gui_pattern(" ", "a b", &options);
        assert!(
            !matches!(result, Ok(None)),
            "GUI must not treat space-only pattern as empty input"
        );
        let re = RegexBuilder::new(" ")
            .ignore_whitespace(true)
            .build()
            .expect("space-only pattern remains valid with ignore_whitespace");
        let expected: Vec<_> = re.find_iter("a b").map(|m| (m.start(), m.end())).collect();
        let actual: Vec<_> = result
            .unwrap()
            .unwrap()
            .iter()
            .map(|m| (m.start, m.end))
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn gui_many_spaces_pattern_is_not_empty_input() {
        let off = RegexOptions {
            ignore_whitespace: false,
            ..Default::default()
        };
        let on = RegexOptions {
            ignore_whitespace: true,
            ..Default::default()
        };
        assert_eq!(gui_effective_pattern("   "), Some("   "));
        assert_eq!(
            evaluate_gui_pattern("   ", "a b", &off).unwrap(),
            Some(Vec::new()),
            "three spaces vs one space is no-match, not empty input"
        );
        let consecutive = evaluate_gui_pattern("   ", "a   b", &off)
            .unwrap()
            .expect("many-space pattern must run");
        assert_eq!(consecutive.len(), 1);
        assert_eq!(consecutive[0].start, 1);
        assert_eq!(consecutive[0].end, 4);
        assert!(
            !matches!(evaluate_gui_pattern("   ", "a b", &on), Ok(None)),
            "ignore-whitespace must not skip a many-space pattern"
        );
    }

    #[test]
    fn gui_empty_invalid_and_no_match_are_distinct() {
        let options = RegexOptions::default();
        assert_eq!(evaluate_gui_pattern("", "a b", &options), Ok(None));
        assert_eq!(
            evaluate_gui_pattern("z", "a b", &options),
            Ok(Some(Vec::new()))
        );
        assert_eq!(
            evaluate_gui_pattern("(", "a b", &options),
            Err(RegexTesterError::InvalidPattern)
        );
    }

    #[test]
    fn email_pattern_two_matches_with_groups() {
        let matches = test_regex(r"(\w+)@(\w+)", "a@b c@d", &RegexOptions::default()).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].index, 0);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 3);
        assert_eq!(matches[0].value, "a@b");
        assert_eq!(matches[0].groups.len(), 2);
        assert_eq!(matches[0].groups[0].index, 1);
        assert_eq!(matches[0].groups[0].name, None);
        assert_eq!(matches[0].groups[0].start, 0);
        assert_eq!(matches[0].groups[0].end, 1);
        assert_eq!(matches[0].groups[0].value, "a");
        assert_eq!(matches[0].groups[1].index, 2);
        assert_eq!(matches[0].groups[1].value, "b");
        assert_eq!(matches[1].index, 1);
        assert_eq!(matches[1].start, 4);
        assert_eq!(matches[1].end, 7);
        assert_eq!(matches[1].value, "c@d");
        assert_eq!(matches[1].groups[0].value, "c");
        assert_eq!(matches[1].groups[1].value, "d");
        let rows = match_table_rows(&matches);
        assert_eq!(
            rows.iter()
                .filter(|r| r.kind == MatchRowKind::Match)
                .count(),
            2
        );
        assert_eq!(rows[0].label, "匹配 1");
        assert_eq!(rows[1].label, "分组 1");
        assert_eq!(rows[2].label, "分组 2");
        assert_eq!(rows[3].label, "匹配 2");
    }

    #[test]
    fn named_group_shows_name_in_table() {
        let matches = test_regex(r"(?<user>\w+)", "alice", &RegexOptions::default()).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].groups.len(), 1);
        assert_eq!(matches[0].groups[0].name.as_deref(), Some("user"));
        assert_eq!(matches[0].groups[0].value, "alice");
        let rows = match_table_rows(&matches);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].kind, MatchRowKind::Match);
        assert_eq!(rows[1].kind, MatchRowKind::Group);
        assert_eq!(rows[1].label, "分组 user");
        assert_eq!(rows[1].start, 0);
        assert_eq!(rows[1].end, 5);
        assert_eq!(rows[1].text, "alice");
    }

    #[test]
    fn no_match_empty_list_and_empty_table() {
        let matches = test_regex(r"xyz", "a@b c@d", &RegexOptions::default()).unwrap();
        assert!(matches.is_empty());
        assert!(match_table_rows(&matches).is_empty());
    }

    #[test]
    fn email_all_matches_off_only_first_row() {
        let options = RegexOptions {
            all_matches: false,
            ..Default::default()
        };
        let matches = test_regex(r"(\w+)@(\w+)", "a@b c@d", &options).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "a@b");
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 3);
        assert_eq!(
            match_table_rows(&matches)
                .iter()
                .filter(|r| r.kind == MatchRowKind::Match)
                .count(),
            1
        );
    }

    #[test]
    fn substitute_numbered_groups() {
        let out = substitute(r"(\w+)@(\w+)", "a@b", "$2/$1", &RegexOptions::default()).unwrap();
        assert_eq!(out, "b/a");
    }

    #[test]
    fn substitute_whole_match_dollar_zero() {
        let out = substitute(r"(\w+)@(\w+)", "a@b", "[$0]", &RegexOptions::default()).unwrap();
        assert_eq!(out, "[a@b]");
    }

    #[test]
    fn substitute_named_groups() {
        let braced = substitute(
            r"(?<user>\w+)@(?<host>\w+)",
            "a@b",
            "${host}/${user}",
            &RegexOptions::default(),
        )
        .unwrap();
        assert_eq!(braced, "b/a");
        let bare = substitute(
            r"(?<user>\w+)@(?<host>\w+)",
            "a@b",
            "$host/$user",
            &RegexOptions::default(),
        )
        .unwrap();
        assert_eq!(bare, "b/a");
    }

    #[test]
    fn substitute_first_only_when_all_matches_off() {
        let options = RegexOptions {
            all_matches: false,
            ..Default::default()
        };
        let out = substitute(r"(\w+)@(\w+)", "a@b c@d", "$2/$1", &options).unwrap();
        assert_eq!(out, "b/a c@d");
    }

    #[test]
    fn substitute_all_matches_default() {
        let out = substitute(r"(\w+)@(\w+)", "a@b c@d", "$2/$1", &RegexOptions::default()).unwrap();
        assert_eq!(out, "b/a d/c");
    }

    #[test]
    fn substitute_interprets_newline_tab_and_backslash() {
        let out = substitute(
            r"(\w+)@(\w+)",
            "a@b",
            r"$1\n$2\t\\",
            &RegexOptions::default(),
        )
        .unwrap();
        assert_eq!(out, "a\nb\t\\");
    }

    #[test]
    fn substitute_illegal_escape_errors() {
        let err = substitute(r"(\w+)@(\w+)", "a@b", r"\x", &RegexOptions::default()).unwrap_err();
        assert_eq!(err, RegexTesterError::InvalidReplacement);
        let message = err.to_string();
        assert!(!message.contains('\\'), "{message}");
        assert!(!message.contains('x'), "{message}");
        assert!(message.contains("非法替换式"), "{message}");
    }

    #[test]
    fn substitute_trailing_backslash_is_illegal() {
        let err = substitute(r"(\w+)@(\w+)", "a@b", "a\\", &RegexOptions::default()).unwrap_err();
        assert_eq!(err, RegexTesterError::InvalidReplacement);
    }

    #[test]
    fn substitute_unclosed_named_dollar_is_illegal() {
        let err =
            substitute(r"(?<user>\w+)", "alice", "${user", &RegexOptions::default()).unwrap_err();
        assert_eq!(err, RegexTesterError::InvalidReplacement);
    }

    #[test]
    fn substitute_invalid_pattern_takes_precedence() {
        let err = substitute("(", "a@b", r"\x", &RegexOptions::default()).unwrap_err();
        assert_eq!(err, RegexTesterError::InvalidPattern);
    }

    #[test]
    fn substitute_no_match_returns_original() {
        let out = substitute("xyz", "a@b", "$0", &RegexOptions::default()).unwrap();
        assert_eq!(out, "a@b");
    }
}
