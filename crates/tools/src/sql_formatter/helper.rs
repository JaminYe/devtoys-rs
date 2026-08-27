pub use crate::indent::Indentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SqlLanguage {
    #[default]
    Sql,
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
}

/// sqlformat 0.3 has no Dialect; every CLI/GUI name maps to generic `QueryParams::None`.
fn query_params(language: SqlLanguage) -> sqlformat::QueryParams {
    match language {
        SqlLanguage::Sql
        | SqlLanguage::Tsql
        | SqlLanguage::Spark
        | SqlLanguage::RedShift
        | SqlLanguage::PostgreSql
        | SqlLanguage::PlSql
        | SqlLanguage::N1ql
        | SqlLanguage::MySql
        | SqlLanguage::MariaDb
        | SqlLanguage::Db2 => sqlformat::QueryParams::None,
    }
}

fn sql_indent(indent: Indentation) -> sqlformat::Indent {
    match indent {
        Indentation::TwoSpaces | Indentation::Minified => sqlformat::Indent::Spaces(2),
        Indentation::FourSpaces => sqlformat::Indent::Spaces(4),
        Indentation::OneTab => sqlformat::Indent::Tabs,
    }
}

pub fn format_sql(
    input: &str,
    indent: Indentation,
    language: SqlLanguage,
    leading_comma: bool,
) -> String {
    let options = sqlformat::FormatOptions {
        indent: sql_indent(indent),
        uppercase: Some(true),
        ..sqlformat::FormatOptions::default()
    };
    let mut formatted = sqlformat::format(input, &query_params(language), &options);
    if leading_comma && indent != Indentation::Minified {
        formatted = apply_leading_comma(&formatted);
    }
    if indent == Indentation::Minified {
        minify(&formatted)
    } else {
        formatted
    }
}

fn minify(sql: &str) -> String {
    sql.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn apply_leading_comma(sql: &str) -> String {
    let mut out = String::new();
    let mut pending_comma = false;
    for (i, line) in sql.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let trimmed = line.trim();
        let (content, had_comma) = if let Some(stripped) = trimmed.strip_suffix(',') {
            (stripped.trim_end(), true)
        } else {
            (trimmed, false)
        };
        out.push_str(indent);
        if pending_comma {
            out.push_str(", ");
        }
        out.push_str(content);
        pending_comma = had_comma;
    }
    if sql.ends_with('\n') {
        out.push('\n');
    }
    out
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

    #[test]
    fn languages_are_accepted() {
        for name in [
            "Sql",
            "Tsql",
            "Spark",
            "RedShift",
            "PostgreSql",
            "PlSql",
            "N1ql",
            "MySql",
            "MariaDb",
            "Db2",
        ] {
            let lang = SqlLanguage::parse(name).expect(name);
            assert_eq!(lang.as_str(), name);
            let got = format_sql("select 1", Indentation::TwoSpaces, lang, false);
            assert!(got.to_ascii_lowercase().contains("select"));
        }
    }

    #[test]
    fn leading_comma_moves_list_commas() {
        let trailing = format_sql(
            "select a, b from t",
            Indentation::TwoSpaces,
            SqlLanguage::Sql,
            false,
        );
        let leading = format_sql(
            "select a, b from t",
            Indentation::TwoSpaces,
            SqlLanguage::Sql,
            true,
        );
        assert_ne!(trailing, leading);
        assert!(
            leading.contains("\n")
                && (leading.contains(", b") || leading.contains(",b") || leading.contains(", ")),
            "leading comma should put comma before the next item: {leading:?}"
        );
    }
}
