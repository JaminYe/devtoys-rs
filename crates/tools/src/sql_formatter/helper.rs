pub use crate::indent::Indentation;

/// Names match upstream DevToys `SqlLanguage`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum SqlLanguage {
    #[default]
    Sql = 0,
    Tsql,
    Spark,
    RedShift,
    PostgreSql,
    PlSql,
    N1ql,
    MySql,
    MariaDb,
    Db2,
}

impl SqlLanguage {
    pub const ALL: [Self; 10] = [
        Self::Sql,
        Self::Tsql,
        Self::Spark,
        Self::RedShift,
        Self::PostgreSql,
        Self::PlSql,
        Self::N1ql,
        Self::MySql,
        Self::MariaDb,
        Self::Db2,
    ];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Sql" => Some(Self::Sql),
            "Tsql" => Some(Self::Tsql),
            "Spark" => Some(Self::Spark),
            "RedShift" => Some(Self::RedShift),
            "PostgreSql" => Some(Self::PostgreSql),
            "PlSql" => Some(Self::PlSql),
            "N1ql" => Some(Self::N1ql),
            "MySql" => Some(Self::MySql),
            "MariaDb" => Some(Self::MariaDb),
            "Db2" => Some(Self::Db2),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sql => "Sql",
            Self::Tsql => "Tsql",
            Self::Spark => "Spark",
            Self::RedShift => "RedShift",
            Self::PostgreSql => "PostgreSql",
            Self::PlSql => "PlSql",
            Self::N1ql => "N1ql",
            Self::MySql => "MySql",
            Self::MariaDb => "MariaDb",
            Self::Db2 => "Db2",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Sql => "SQL",
            Self::Tsql => "T-SQL",
            Self::Spark => "Spark",
            Self::RedShift => "Redshift",
            Self::PostgreSql => "PostgreSQL",
            Self::PlSql => "PL/SQL",
            Self::N1ql => "N1QL",
            Self::MySql => "MySQL",
            Self::MariaDb => "MariaDB",
            Self::Db2 => "DB2",
        }
    }
}

pub fn format_sql(
    input: &str,
    indent: Indentation,
    language: SqlLanguage,
    leading_comma: bool,
) -> String {
    if indent == Indentation::Minified {
        return minify(input);
    }
    let mut formatted = super::dialect::format(input, indent, language);
    if leading_comma {
        formatted = apply_leading_comma(&formatted);
    }
    formatted
}

/// Collapse whitespace outside strings/comments. Keep a newline after `--`
/// comments so later statements are not commented out. String and block-comment
/// contents are copied unchanged.
fn minify(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;
    let mut pending_space = false;

    while i < len {
        let c = bytes[i];
        if c == b'\'' || c == b'"' {
            flush_space(&mut out, &mut pending_space);
            i = copy_quoted(sql, i, c, &mut out);
            continue;
        }
        if c == b'/' && peek(bytes, i + 1) == Some(b'*') {
            flush_space(&mut out, &mut pending_space);
            i = copy_block_comment(sql, i, &mut out);
            continue;
        }
        if c == b'-' && peek(bytes, i + 1) == Some(b'-') {
            flush_space(&mut out, &mut pending_space);
            i = copy_line_comment(sql, i, &mut out);
            i = skip_line_ending(bytes, i);
            if i < len {
                out.push('\n');
                pending_space = false;
            }
            continue;
        }
        if c.is_ascii_whitespace() {
            if !out.is_empty() && !out.ends_with('\n') {
                pending_space = true;
            }
            i += 1;
            continue;
        }
        flush_space(&mut out, &mut pending_space);
        let n = utf8_char_len(bytes, i);
        out.push_str(&sql[i..i + n]);
        i += n;
    }
    out
}

fn peek(bytes: &[u8], i: usize) -> Option<u8> {
    bytes.get(i).copied()
}

fn flush_space(out: &mut String, pending: &mut bool) {
    if *pending {
        out.push(' ');
        *pending = false;
    }
}

fn utf8_char_len(bytes: &[u8], i: usize) -> usize {
    let n = match bytes[i] {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    };
    n.min(bytes.len() - i)
}

fn copy_quoted(sql: &str, start: usize, quote: u8, out: &mut String) -> usize {
    let bytes = sql.as_bytes();
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == quote {
            if peek(bytes, i + 1) == Some(quote) {
                i += 2;
                continue;
            }
            i += 1;
            break;
        }
        i += 1;
    }
    out.push_str(&sql[start..i]);
    i
}

fn copy_block_comment(sql: &str, start: usize, out: &mut String) -> usize {
    let bytes = sql.as_bytes();
    let mut i = start + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            i += 2;
            out.push_str(&sql[start..i]);
            return i;
        }
        i += 1;
    }
    out.push_str(&sql[start..]);
    bytes.len()
}

fn copy_line_comment(sql: &str, start: usize, out: &mut String) -> usize {
    let bytes = sql.as_bytes();
    let mut i = start + 2;
    while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
        i += 1;
    }
    out.push_str(&sql[start..i]);
    i
}

fn skip_line_ending(bytes: &[u8], i: usize) -> usize {
    if i >= bytes.len() {
        return i;
    }
    match bytes[i] {
        b'\r' if peek(bytes, i + 1) == Some(b'\n') => i + 2,
        b'\r' | b'\n' => i + 1,
        _ => i,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Span {
    Code,
    Quote(u8),
    BlockComment,
}

/// Move end-of-line syntactic commas in front of the next code line.
/// String literals and comments are copied unchanged.
fn apply_leading_comma(sql: &str) -> String {
    let mut out = String::new();
    let mut pending_comma = false;
    let mut span = Span::Code;
    for (i, line) in sql.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        pending_comma = write_leading_comma_line(&mut out, line, &mut span, pending_comma);
    }
    if sql.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn write_leading_comma_line(
    out: &mut String,
    line: &str,
    span: &mut Span,
    pending_comma: bool,
) -> bool {
    let started_in = *span;
    let mask = classify_line(line, span);
    if started_in == Span::Code {
        let indent_len = line.len() - line.trim_start().len();
        out.push_str(&line[..indent_len]);
        if pending_comma {
            out.push_str(", ");
        }
        return write_code_line_body(out, &line[indent_len..], &mask[indent_len..]);
    }

    let mut prefix_end = 0;
    while prefix_end < mask.len() && !mask[prefix_end] {
        prefix_end += 1;
    }
    out.push_str(&line[..prefix_end]);
    if prefix_end == line.len() {
        pending_comma
    } else {
        if pending_comma {
            out.push_str(", ");
        }
        write_code_line_body(out, &line[prefix_end..], &mask[prefix_end..])
    }
}

fn write_code_line_body(out: &mut String, rest: &str, mask: &[bool]) -> bool {
    let trimmed_end = rest.trim_end().len();
    let had_comma =
        trimmed_end > 0 && rest.as_bytes()[trimmed_end - 1] == b',' && mask[trimmed_end - 1];
    let body = if had_comma {
        rest[..trimmed_end - 1].trim_end()
    } else {
        rest.trim_end()
    };
    out.push_str(body);
    had_comma
}

fn classify_line(line: &str, span: &mut Span) -> Vec<bool> {
    let bytes = line.as_bytes();
    let mut mask = vec![true; bytes.len()];
    let mut i = 0;
    while i < bytes.len() {
        match *span {
            Span::Quote(quote) => {
                let (next, closed) = scan_quoted_body(bytes, i, quote);
                mask[i..next].fill(false);
                i = next;
                if closed {
                    *span = Span::Code;
                } else {
                    return mask;
                }
            }
            Span::BlockComment => {
                let (next, closed) = scan_block_comment_body(bytes, i);
                mask[i..next].fill(false);
                i = next;
                if closed {
                    *span = Span::Code;
                } else {
                    return mask;
                }
            }
            Span::Code => {
                let c = bytes[i];
                if c == b'\'' || c == b'"' {
                    mask[i] = false;
                    *span = Span::Quote(c);
                    i += 1;
                    continue;
                }
                if c == b'/' && peek(bytes, i + 1) == Some(b'*') {
                    mask[i] = false;
                    mask[i + 1] = false;
                    *span = Span::BlockComment;
                    i += 2;
                    continue;
                }
                if c == b'-' && peek(bytes, i + 1) == Some(b'-') {
                    mask[i..].fill(false);
                    return mask;
                }
                i += utf8_char_len(bytes, i);
            }
        }
    }
    mask
}

fn scan_quoted_body(bytes: &[u8], mut i: usize, quote: u8) -> (usize, bool) {
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && matches!(quote, b'\'' | b'"') && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next == quote || next == b'\\' {
                i += 2;
                continue;
            }
        }
        if c == quote {
            if peek(bytes, i + 1) == Some(quote) {
                i += 2;
                continue;
            }
            return (i + 1, true);
        }
        i += 1;
    }
    (bytes.len(), false)
}

fn scan_block_comment_body(bytes: &[u8], mut i: usize) -> (usize, bool) {
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return (i + 2, true);
        }
        i += 1;
    }
    (bytes.len(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_select_1_has_keyword_and_break() {
        let got = format_sql("select 1", Indentation::TwoSpaces, SqlLanguage::Sql, false);
        let folded = got.to_ascii_lowercase();
        assert!(
            folded.contains("select"),
            "pretty SQL should keep the keyword: {got:?}"
        );
        assert!(
            got.contains('\n') || got.contains("  ") || got.contains('\t'),
            "pretty SQL should indent or wrap: {got:?}"
        );
    }

    #[test]
    fn minified_is_single_line_ish() {
        let got = format_sql(
            "select 1 from dual",
            Indentation::Minified,
            SqlLanguage::Sql,
            false,
        );
        assert!(
            !got.trim().contains('\n'),
            "minified SQL should be single-line-ish: {got:?}"
        );
        assert!(got.to_ascii_lowercase().contains("select"));
    }

    #[test]
    fn junk_does_not_panic() {
        let got = format_sql(
            ")))not sql!!! {{{ \0",
            Indentation::TwoSpaces,
            SqlLanguage::Sql,
            false,
        );
        let _ = got;
    }

    fn second_select_is_executable(got: &str) {
        let lower = got.to_ascii_lowercase();
        assert!(
            lower.contains("select 2"),
            "minified SQL must keep the second statement: {got:?}"
        );
        for line in got.lines() {
            if let Some(pos) = line.find("--") {
                let after = line[pos..].to_ascii_lowercase();
                assert!(
                    !after.contains("select 2"),
                    "line comment must not swallow the next statement: {got:?}"
                );
            }
        }
    }

    #[test]
    fn minify_line_comment_does_not_swallow_next_statement() {
        let got = format_sql(
            "select 1; -- comment\nselect 2;",
            Indentation::Minified,
            SqlLanguage::Sql,
            false,
        );
        second_select_is_executable(&got);
    }

    #[test]
    fn minify_preserves_string_with_comment_marker_and_newlines() {
        let input = "select 'hello\n-- not a comment\nworld'";
        let got = format_sql(input, Indentation::Minified, SqlLanguage::Sql, false);
        assert!(
            got.contains("hello\n-- not a comment\nworld"),
            "string contents must be unchanged: {got:?}"
        );
    }

    #[test]
    fn minify_block_comment_dashes_are_not_line_comments() {
        let got = format_sql(
            "select 1 /* -- not a line comment */;\nselect 2;",
            Indentation::Minified,
            SqlLanguage::Sql,
            false,
        );
        let lower = got.to_ascii_lowercase();
        assert!(
            lower.contains("select 2"),
            "second statement must remain: {got:?}"
        );
        let start = got
            .find("/*")
            .expect("block comment should stay a block comment");
        let end = got.find("*/").expect("block comment should stay closed");
        assert!(
            got[start..=end + 1].contains("--"),
            "dashes inside the block comment must stay inside it: {got:?}"
        );
        assert!(
            !got[start..=end + 1].contains('\n'),
            "block comment must not be split as if -- started a line comment: {got:?}"
        );
        assert!(!got[start..=end + 1]
            .to_ascii_lowercase()
            .contains("select 2"));
    }

    #[test]
    fn minify_crlf_line_comment_keeps_following_statement() {
        let got = format_sql(
            "select 1; -- comment\r\nselect 2;",
            Indentation::Minified,
            SqlLanguage::Sql,
            false,
        );
        second_select_is_executable(&got);
    }

    #[test]
    fn minify_without_trailing_newline_keeps_following_statement() {
        let input = "select 1; -- comment\nselect 2;";
        assert!(!input.ends_with('\n'));
        let got = format_sql(input, Indentation::Minified, SqlLanguage::Sql, false);
        second_select_is_executable(&got);
    }

    #[test]
    fn all_ten_languages_parse_and_reject_unknown() {
        assert_eq!(SqlLanguage::ALL.len(), 10);
        for lang in SqlLanguage::ALL {
            assert_eq!(
                SqlLanguage::parse(lang.as_str()),
                Some(lang),
                "{}",
                lang.as_str()
            );
        }
        for name in ["sql", "TSQL", "Oracle", "Postgres", ""] {
            assert_eq!(SqlLanguage::parse(name), None, "{name}");
        }
    }

    fn pretty_lang(input: &str, language: SqlLanguage) -> String {
        format_sql(input, Indentation::TwoSpaces, language, false)
    }

    #[test]
    fn tsql_keeps_brackets_and_top_and_differs_from_sql() {
        let input = "select top 5 [Name] from dbo.Users";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let tsql = pretty_lang(input, SqlLanguage::Tsql);
        assert_ne!(
            sql, tsql,
            "T-SQL must format differently from generic SQL:\nSQL={sql:?}\nTSQL={tsql:?}"
        );
        assert!(
            tsql.contains("[Name]"),
            "T-SQL must keep bracket identifier: {tsql:?}"
        );
        assert!(
            tsql.to_ascii_uppercase().contains("TOP"),
            "T-SQL must keep TOP: {tsql:?}"
        );
        let folded = tsql.to_ascii_uppercase();
        let top_at = folded.find("TOP").expect("TOP");
        assert!(
            !folded[top_at..top_at + 3].is_empty(),
            "TOP token present: {tsql:?}"
        );
    }

    #[test]
    fn generic_sql_formats_select_from_where() {
        let got = format_sql(
            "select 1 from t where a = 1",
            Indentation::TwoSpaces,
            SqlLanguage::Sql,
            false,
        );
        let folded = got.to_ascii_lowercase();
        assert!(folded.contains("select"), "missing SELECT: {got:?}");
        assert!(folded.contains("from"), "missing FROM: {got:?}");
        assert!(folded.contains("where"), "missing WHERE: {got:?}");
    }

    /// Unescaped contents of `'...'` / `"..."` literals. Skips `--` and `/* */`.
    fn sql_string_literal_values(sql: &str) -> Vec<String> {
        let mut chars = sql.chars().peekable();
        let mut out = Vec::new();
        while let Some(c) = chars.next() {
            match c {
                '\'' | '"' => out.push(read_sql_quoted(&mut chars, c)),
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    skip_sql_block_comment(&mut chars);
                }
                '-' if chars.peek() == Some(&'-') => {
                    chars.next();
                    skip_sql_line_comment(&mut chars);
                }
                _ => {}
            }
        }
        out
    }

    fn read_sql_quoted<I: Iterator<Item = char>>(
        chars: &mut std::iter::Peekable<I>,
        quote: char,
    ) -> String {
        let mut value = String::new();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.peek().copied() {
                    Some(next) if next == quote || next == '\\' => {
                        value.push(chars.next().expect("peeked escape"));
                    }
                    _ => value.push(c),
                }
                continue;
            }
            if c == quote {
                if chars.peek() == Some(&quote) {
                    value.push(quote);
                    chars.next();
                    continue;
                }
                break;
            }
            value.push(c);
        }
        value
    }

    fn skip_sql_block_comment<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) {
        let mut prev_star = false;
        for c in chars {
            if prev_star && c == '/' {
                return;
            }
            prev_star = c == '*';
        }
    }

    fn skip_sql_line_comment<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) {
        for c in chars.by_ref() {
            if c == '\n' {
                return;
            }
        }
    }

    fn sql_block_comment_bodies(sql: &str) -> Vec<String> {
        let mut chars = sql.chars().peekable();
        let mut out = Vec::new();
        while let Some(c) = chars.next() {
            match c {
                '\'' | '"' => {
                    let _ = read_sql_quoted(&mut chars, c);
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    let mut body = String::new();
                    let mut prev_star = false;
                    while let Some(ch) = chars.next() {
                        if prev_star && ch == '/' {
                            body.pop();
                            break;
                        }
                        prev_star = ch == '*';
                        body.push(ch);
                    }
                    out.push(body);
                }
                '-' if chars.peek() == Some(&'-') => {
                    chars.next();
                    skip_sql_line_comment(&mut chars);
                }
                _ => {}
            }
        }
        out
    }

    fn sql_line_comment_texts(sql: &str) -> Vec<String> {
        let mut chars = sql.chars().peekable();
        let mut out = Vec::new();
        while let Some(c) = chars.next() {
            match c {
                '\'' | '"' => {
                    let _ = read_sql_quoted(&mut chars, c);
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    skip_sql_block_comment(&mut chars);
                }
                '-' if chars.peek() == Some(&'-') => {
                    chars.next();
                    let mut body = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == '\n' {
                            break;
                        }
                        body.push(ch);
                    }
                    out.push(body);
                }
                _ => {}
            }
        }
        out
    }

    fn pretty(input: &str, leading_comma: bool) -> String {
        format_sql(
            input,
            Indentation::TwoSpaces,
            SqlLanguage::Sql,
            leading_comma,
        )
    }

    #[test]
    fn leading_comma_moves_list_commas() {
        let trailing = pretty("select a, b, c from t", false);
        let leading = pretty("select a, b, c from t", true);
        assert_ne!(trailing, leading);
        let trailing_lines: Vec<_> = trailing.lines().map(str::trim).collect();
        assert!(
            trailing_lines.iter().any(|l| *l == "a,"),
            "leading-comma off must keep trailing commas: {trailing:?}"
        );
        assert!(
            trailing_lines.iter().any(|l| *l == "b,"),
            "leading-comma off must keep trailing commas: {trailing:?}"
        );
        let leading_lines: Vec<_> = leading.lines().map(str::trim).collect();
        assert!(
            leading_lines.iter().any(|l| *l == "a"),
            "first item should not keep a trailing comma: {leading:?}"
        );
        assert!(
            leading_lines.iter().any(|l| *l == ", b"),
            "leading comma should precede b: {leading:?}"
        );
        assert!(
            leading_lines.iter().any(|l| *l == ", c"),
            "leading comma should precede c: {leading:?}"
        );
    }

    #[test]
    fn leading_comma_preserves_multiline_string_contents() {
        let input = "SELECT 'a,\nb';";
        let got = pretty(input, true);
        assert_eq!(
            sql_string_literal_values(&got),
            vec!["a,\nb".to_string()],
            "string value must stay a,\\nb: {got:?}"
        );
    }

    #[test]
    fn leading_comma_off_preserves_multiline_string_and_trailing_commas() {
        let got = pretty("SELECT 'a,\nb';", false);
        assert_eq!(
            sql_string_literal_values(&got),
            vec!["a,\nb".to_string()],
            "leading-comma off must not rewrite the string: {got:?}"
        );
        let list = pretty("select a, b from t", false);
        let lines: Vec<_> = list.lines().map(str::trim).collect();
        assert!(
            lines.iter().any(|l| *l == "a,"),
            "off must keep previous trailing-comma layout: {list:?}"
        );
        assert!(
            !lines.iter().any(|l| *l == ", b"),
            "off must not insert leading commas: {list:?}"
        );
    }

    #[test]
    fn leading_comma_preserves_escaped_quotes_and_comment_markers_in_strings() {
        let input = "SELECT 'it''s,\nnot -- a comment\n/* still string */', x FROM t";
        let got = pretty(input, true);
        assert_eq!(
            sql_string_literal_values(&got),
            vec!["it's,\nnot -- a comment\n/* still string */".to_string()],
            "escaped quotes and comment markers inside strings must stay: {got:?}"
        );
        let lines: Vec<_> = got.lines().map(str::trim).collect();
        assert!(
            lines.iter().any(|l| *l == ", x"),
            "syntactic comma before x should still move: {got:?}"
        );
    }

    #[test]
    fn leading_comma_preserves_block_and_line_comment_commas() {
        let block_input = "SELECT a, /* keep,\ncomma */ b FROM t";
        let block = pretty(block_input, true);
        let bodies = sql_block_comment_bodies(&block);
        assert!(
            bodies.iter().any(|c| c.contains("keep,")),
            "block comment must keep its comma after keep: {block:?}"
        );
        assert!(
            !bodies
                .iter()
                .any(|c| c.contains("keep\n,") || c.contains("keep\n ,")),
            "leading comma must not move the comma inside the block comment: {block:?}"
        );

        let line_input = "SELECT a -- keep,\nb FROM t";
        let line = pretty(line_input, true);
        let comments = sql_line_comment_texts(&line);
        assert!(
            comments.iter().any(|c| c.contains("keep,")),
            "line comment must keep its trailing comma: {line:?}"
        );
    }

    #[test]
    fn leading_comma_moves_comma_after_multiline_string_item() {
        let got = pretty("SELECT 'a,\nb', c FROM t", true);
        assert_eq!(
            sql_string_literal_values(&got),
            vec!["a,\nb".to_string()],
            "multiline string item must stay intact: {got:?}"
        );
        let lines: Vec<_> = got.lines().map(str::trim).collect();
        assert!(
            lines.iter().any(|l| *l == ", c"),
            "comma after the string item should lead the next field: {got:?}"
        );
    }

    #[test]
    fn spark_keeps_eqeq_unlike_sql() {
        let input = "select 1 == 2 from t";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let spark = pretty_lang(input, SqlLanguage::Spark);
        assert_ne!(sql, spark, "SQL={sql:?}\nSpark={spark:?}");
        assert!(spark.contains("=="), "Spark must keep ==: {spark:?}");
        assert!(!spark.contains("= ="), "Spark must not smash ==: {spark:?}");
    }

    #[test]
    fn redshift_keeps_pipe_slash_unlike_sql() {
        let input = "select 1 |/ 2 from t";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let rs = pretty_lang(input, SqlLanguage::RedShift);
        assert_ne!(sql, rs, "SQL={sql:?}\nRedshift={rs:?}");
        assert!(rs.contains("|/"), "Redshift must keep |/: {rs:?}");
    }

    #[test]
    fn postgresql_keeps_dollar_quotes_unlike_sql() {
        let input = "select $$foo, bar$$ from t";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let pg = pretty_lang(input, SqlLanguage::PostgreSql);
        assert_ne!(sql, pg, "SQL={sql:?}\nPG={pg:?}");
        assert!(
            pg.contains("$$foo, bar$$"),
            "PostgreSQL must keep dollar quote: {pg:?}"
        );
    }

    #[test]
    fn plsql_keeps_assign_unlike_sql() {
        let input = "begin x := 1; end;";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let pl = pretty_lang(input, SqlLanguage::PlSql);
        assert_ne!(sql, pl, "SQL={sql:?}\nPL/SQL={pl:?}");
        assert!(pl.contains(":="), "PL/SQL must keep :=: {pl:?}");
        assert!(!pl.contains(": ="), "PL/SQL must not smash :=: {pl:?}");
    }

    #[test]
    fn n1ql_keeps_eqeq_unlike_sql() {
        let input = "select 1 == 2 from t";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let n1ql = pretty_lang(input, SqlLanguage::N1ql);
        assert_ne!(sql, n1ql, "SQL={sql:?}\nN1QL={n1ql:?}");
        assert!(n1ql.contains("=="), "N1QL must keep ==: {n1ql:?}");
    }

    #[test]
    fn mysql_keeps_hash_comment_unlike_sql() {
        let input = "select `col` := 1 #keep";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let mysql = pretty_lang(input, SqlLanguage::MySql);
        assert_ne!(sql, mysql, "SQL={sql:?}\nMySQL={mysql:?}");
        assert!(
            mysql.contains("`col`"),
            "MySQL must keep backtick ident: {mysql:?}"
        );
        assert!(mysql.contains(":="), "MySQL must keep :=: {mysql:?}");
        assert!(
            mysql.contains("#keep"),
            "MySQL must keep # comment: {mysql:?}"
        );
    }

    #[test]
    fn mariadb_keeps_assign_unlike_sql() {
        let input = "select @a := 1";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let maria = pretty_lang(input, SqlLanguage::MariaDb);
        assert_ne!(sql, maria, "SQL={sql:?}\nMariaDB={maria:?}");
        assert!(maria.contains(":="), "MariaDB must keep :=: {maria:?}");
    }

    #[test]
    fn db2_keeps_starstar_unlike_sql() {
        let input = "select 1 ** 2 from t";
        let sql = pretty_lang(input, SqlLanguage::Sql);
        let db2 = pretty_lang(input, SqlLanguage::Db2);
        assert_ne!(sql, db2, "SQL={sql:?}\nDB2={db2:?}");
        assert!(db2.contains("**"), "DB2 must keep **: {db2:?}");
        assert!(!db2.contains("* *"), "DB2 must not smash **: {db2:?}");
    }
}
