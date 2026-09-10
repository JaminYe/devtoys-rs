use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListMode {
    #[default]
    AInterB,
    AUnionB,
    AOnly,
    BOnly,
}

impl ListMode {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "AInterB" => Self::AInterB,
            "AUnionB" => Self::AUnionB,
            "AOnly" => Self::AOnly,
            "BOnly" => Self::BOnly,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AInterB => "AInterB",
            Self::AUnionB => "AUnionB",
            Self::AOnly => "AOnly",
            Self::BOnly => "BOnly",
        }
    }
}

/// Split like .NET `EnumerateLines`: CR / LF / CRLF, keep surrounding whitespace
/// and empty lines (including a trailing empty item after a final newline).
fn split_lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push(&text[start..i]);
                i += 1;
                start = i;
            }
            b'\r' => {
                lines.push(&text[start..i]);
                i += 1;
                if i < bytes.len() && bytes[i] == b'\n' {
                    i += 1;
                }
                start = i;
            }
            _ => i += 1,
        }
    }
    lines.push(&text[start..]);
    lines
}

fn item_key(item: &str, case_sensitive: bool, ignore_surrounding_whitespace: bool) -> String {
    let item = if ignore_surrounding_whitespace {
        item.trim()
    } else {
        item
    };
    if case_sensitive {
        item.to_string()
    } else {
        item.to_lowercase()
    }
}

fn unique_lines(
    text: &str,
    case_sensitive: bool,
    ignore_surrounding_whitespace: bool,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for line in split_lines(text) {
        if seen.insert(item_key(
            line,
            case_sensitive,
            ignore_surrounding_whitespace,
        )) {
            out.push(line.to_string());
        }
    }
    out
}

/// Compare two lists. `ignore_surrounding_whitespace` defaults off at call sites
/// (GUI checkbox / CLI `--isw`); comparison keys may trim, result items keep source text.
pub fn compare_lists(
    a: &str,
    b: &str,
    mode: ListMode,
    case_sensitive: bool,
    ignore_surrounding_whitespace: bool,
) -> String {
    let list_a = unique_lines(a, case_sensitive, ignore_surrounding_whitespace);
    let list_b = unique_lines(b, case_sensitive, ignore_surrounding_whitespace);
    let keys_a: HashSet<String> = list_a
        .iter()
        .map(|s| item_key(s, case_sensitive, ignore_surrounding_whitespace))
        .collect();
    let keys_b: HashSet<String> = list_b
        .iter()
        .map(|s| item_key(s, case_sensitive, ignore_surrounding_whitespace))
        .collect();

    let result: Vec<String> = match mode {
        ListMode::AInterB => list_a
            .into_iter()
            .filter(|s| {
                keys_b.contains(&item_key(s, case_sensitive, ignore_surrounding_whitespace))
            })
            .collect(),
        ListMode::AUnionB => {
            let mut out = list_a;
            for item in list_b {
                if !keys_a.contains(&item_key(
                    &item,
                    case_sensitive,
                    ignore_surrounding_whitespace,
                )) {
                    out.push(item);
                }
            }
            out
        }
        ListMode::AOnly => list_a
            .into_iter()
            .filter(|s| {
                !keys_b.contains(&item_key(s, case_sensitive, ignore_surrounding_whitespace))
            })
            .collect(),
        ListMode::BOnly => list_b
            .into_iter()
            .filter(|s| {
                !keys_a.contains(&item_key(s, case_sensitive, ignore_surrounding_whitespace))
            })
            .collect(),
    };
    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keep(a: &str, b: &str, mode: ListMode, case_sensitive: bool) -> String {
        compare_lists(a, b, mode, case_sensitive, false)
    }

    fn ignore_ws(a: &str, b: &str, mode: ListMode, case_sensitive: bool) -> String {
        compare_lists(a, b, mode, case_sensitive, true)
    }

    #[test]
    fn split_lines_matches_enumerate_lines() {
        assert_eq!(split_lines(""), [""]);
        assert_eq!(split_lines("hello"), ["hello"]);
        assert_eq!(split_lines("hello\r"), ["hello", ""]);
        assert_eq!(split_lines("hello\n"), ["hello", ""]);
        assert_eq!(split_lines("hello\r\n"), ["hello", ""]);
        assert_eq!(split_lines("hello\n\r"), ["hello", "", ""]);
        assert_eq!(split_lines("hello\nworld\rhi"), ["hello", "world", "hi"]);
        assert_eq!(
            split_lines("hello \n world \r hi \r\n "),
            ["hello ", " world ", " hi ", " "]
        );
    }

    #[test]
    fn cr_lf_crlf_all_split_items() {
        assert_eq!(keep("a\rb", "b", ListMode::AInterB, true), "b");
        assert_eq!(
            keep("A\nBa\nC\nC", "Ba\nC\nD", ListMode::AInterB, true),
            "Ba\nC"
        );
        assert_eq!(
            keep("A\rBa\rC\rC", "Ba\rC\rD", ListMode::AInterB, true),
            "Ba\nC"
        );
        assert_eq!(
            keep("A\r\nBa\r\nC\r\nC", "Ba\r\nC\r\nD", ListMode::AInterB, true),
            "Ba\nC"
        );
        assert_eq!(
            keep("A\r\nBa\nC\nC", "Ba\nC\r\nD", ListMode::AInterB, true),
            "Ba\nC"
        );
    }

    #[test]
    fn default_keeps_surrounding_whitespace() {
        assert_eq!(keep(" a ", "a", ListMode::AInterB, true), "");
        assert_eq!(keep(" a ", "a", ListMode::AUnionB, true), " a \na");
        assert_eq!(keep(" a ", "a", ListMode::AOnly, true), " a ");
        assert_eq!(keep(" a ", "a", ListMode::BOnly, true), "a");
        assert_eq!(keep("hello ", "hello", ListMode::AInterB, true), "");
    }

    #[test]
    fn ignore_surrounding_whitespace_compares_trimmed_keeps_source() {
        assert_eq!(ignore_ws(" a ", "a", ListMode::AInterB, true), " a ");
        assert_eq!(ignore_ws(" a ", "a", ListMode::AUnionB, true), " a ");
        assert_eq!(ignore_ws(" a ", "a", ListMode::AOnly, true), "");
        assert_eq!(ignore_ws(" a ", "a", ListMode::BOnly, true), "");
        assert_eq!(ignore_ws(" a \n a", "a", ListMode::AInterB, true), " a ");
    }

    #[test]
    fn empty_lines_are_kept() {
        assert_eq!(keep("A\nB\nC", "", ListMode::AUnionB, true), "A\nB\nC\n");
        assert_eq!(keep("", "A\nB\nC", ListMode::AUnionB, true), "\nA\nB\nC");
        assert_eq!(keep("A\nB\nC", "", ListMode::AInterB, true), "");
        assert_eq!(keep("", "A\nB\nC", ListMode::AInterB, true), "");
        assert_eq!(keep("A\nB\nC", "", ListMode::AOnly, true), "A\nB\nC");
        assert_eq!(keep("", "A\nb\nC", ListMode::AOnly, true), "");
        assert_eq!(keep("A\nb\nC", "", ListMode::BOnly, true), "");
        assert_eq!(keep("", "A\nb\nC", ListMode::BOnly, true), "A\nb\nC");
        assert_eq!(keep("a\n\nb", "\nb", ListMode::AInterB, true), "\nb");
        assert_eq!(keep("a\nb\n", "b\nc", ListMode::AUnionB, true), "a\nb\n\nc");
    }

    #[test]
    fn duplicates_keep_first_source_order() {
        assert_eq!(
            keep("A\nBa\nC\nC", "Ba\nC\nD", ListMode::AInterB, true),
            "Ba\nC"
        );
        assert_eq!(
            keep("A\nBa\nC\nBa", "BA\nC\nD", ListMode::AUnionB, true),
            "A\nBa\nC\nBA\nD"
        );
        assert_eq!(
            keep("A\nBa\nC\nBa", "BA\nC\nD", ListMode::AUnionB, false),
            "A\nBa\nC\nD"
        );
    }

    #[test]
    fn case_sensitive_and_insensitive_are_independent_of_whitespace() {
        assert_eq!(keep("A", "a", ListMode::AInterB, false), "A");
        assert_eq!(keep("A", "a", ListMode::AInterB, true), "");
        assert_eq!(keep("A\nb\nC", "B\nC\nD", ListMode::AInterB, false), "b\nC");
        assert_eq!(keep("A\nb\nC", "B\nC\nD", ListMode::AInterB, true), "C");
        assert_eq!(keep(" A ", "a", ListMode::AInterB, false), "");
        assert_eq!(ignore_ws(" A ", "a", ListMode::AInterB, false), " A ");
        assert_eq!(ignore_ws(" A ", "a", ListMode::AInterB, true), "");
        assert_eq!(ignore_ws(" A ", "A", ListMode::AInterB, true), " A ");
    }

    #[test]
    fn each_set_operation_matches_upstream_case_insensitive() {
        assert_eq!(
            keep("A\nBa\nC\nBa", "BA\nC\nD", ListMode::AInterB, false),
            "Ba\nC"
        );
        assert_eq!(
            keep("A\nBa\nC\nBa", "BA\nC\nD", ListMode::AUnionB, false),
            "A\nBa\nC\nD"
        );
        assert_eq!(keep("A\nBa\nC\nc", "Ba\nC\nD", ListMode::AOnly, false), "A");
        assert_eq!(keep("A\nBa\nC\nc", "Ba\nC\nD", ListMode::BOnly, false), "D");
        assert_eq!(
            keep("A\nb\nC", "B\nC\nD", ListMode::AUnionB, false),
            "A\nb\nC\nD"
        );
        assert_eq!(keep("A\nb\nC", "B\nC\nD", ListMode::AOnly, false), "A");
        assert_eq!(keep("A\nb\nC", "B\nC\nD", ListMode::BOnly, false), "D");
        assert_eq!(keep("A\nB\nC", "D\nE\nF", ListMode::AInterB, false), "");
        assert_eq!(
            keep("A\nB\nC", "D\nE\nF", ListMode::AUnionB, false),
            "A\nB\nC\nD\nE\nF"
        );
        assert_eq!(
            keep("A\nB\nC", "D\nE\nF", ListMode::AOnly, false),
            "A\nB\nC"
        );
        assert_eq!(
            keep("A\nB\nC", "D\nE\nF", ListMode::BOnly, false),
            "D\nE\nF"
        );
    }

    #[test]
    fn each_set_operation_matches_upstream_case_sensitive() {
        assert_eq!(keep("A\nb\nC", "B\nC\nD", ListMode::AInterB, true), "C");
        assert_eq!(
            keep("A\nb\nC", "B\nC\nD", ListMode::AUnionB, true),
            "A\nb\nC\nB\nD"
        );
        assert_eq!(keep("A\nb\nC", "B\nC\nD", ListMode::AOnly, true), "A\nb");
        assert_eq!(keep("A\nb\nC", "B\nC\nD", ListMode::BOnly, true), "B\nD");
        assert_eq!(
            keep("A\nBa\nC\nc", "Ba\nC\nD", ListMode::AOnly, true),
            "A\nc"
        );
        assert_eq!(keep("A\nBa\nC\nc", "Ba\nC\nD", ListMode::BOnly, true), "D");
    }

    #[test]
    fn toggling_options_updates_results() {
        let a = " A ";
        let b = "a";
        assert_eq!(keep(a, b, ListMode::AInterB, true), "");
        assert_eq!(keep(a, b, ListMode::AInterB, false), "");
        assert_eq!(ignore_ws(a, b, ListMode::AInterB, true), "");
        assert_eq!(ignore_ws(a, b, ListMode::AInterB, false), " A ");
        assert_eq!(keep(a, b, ListMode::AUnionB, false), " A \na");
        assert_eq!(ignore_ws(a, b, ListMode::AUnionB, false), " A ");
        assert_eq!(keep(a, b, ListMode::AOnly, false), " A ");
        assert_eq!(ignore_ws(a, b, ListMode::AOnly, false), "");
        assert_eq!(keep(a, b, ListMode::BOnly, false), "a");
        assert_eq!(ignore_ws(a, b, ListMode::BOnly, false), "");
    }

    #[test]
    fn simple_sets_without_trailing_newline() {
        let a = "a\nb";
        let b = "b\nc";
        assert_eq!(keep(a, b, ListMode::AInterB, true), "b");
        assert_eq!(keep(a, b, ListMode::AUnionB, true), "a\nb\nc");
        assert_eq!(keep(a, b, ListMode::AOnly, true), "a");
        assert_eq!(keep(a, b, ListMode::BOnly, true), "c");
    }
}
