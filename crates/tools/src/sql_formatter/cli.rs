use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{format_sql, Indentation, SqlLanguage, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "sqlFormatter",
        aliases: &["sqlf"],
        about: "美化 SQL",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("输入文件路径；若路径不是已有文件则当作内联 SQL"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出文件；省略则写到 stdout"),
    )
    .arg(
        Arg::new("indentation")
            .long("indentation")
            .value_parser(["TwoSpaces", "FourSpaces", "OneTab", "Minified"])
            .default_value("TwoSpaces"),
    )
    .arg(
        Arg::new("language")
            .long("language")
            .help("SQL 方言")
            .value_parser(SqlLanguage::ALL.map(SqlLanguage::as_str))
            .default_value("Sql"),
    )
    .arg(
        Arg::new("leadingComma")
            .long("leadingComma")
            .alias("leading-comma")
            .action(ArgAction::SetTrue),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let indent = parse_indent(matches.get_one::<String>("indentation").map(String::as_str))?;
    let language = parse_language(matches.get_one::<String>("language").map(String::as_str))?;
    let leading_comma = matches.get_flag("leadingComma");
    let formatted = format_sql(&source, indent, language, leading_comma);
    write_output(
        matches.get_one::<String>("output").map(String::as_str),
        &formatted,
    )
}

fn parse_indent(value: Option<&str>) -> Result<Indentation, CliError> {
    let raw = value.unwrap_or("TwoSpaces");
    Indentation::parse(raw).ok_or_else(|| CliError::new(format!("未知缩进: {raw}")))
}

fn parse_language(value: Option<&str>) -> Result<SqlLanguage, CliError> {
    let raw = value.unwrap_or("Sql");
    SqlLanguage::parse(raw).ok_or_else(|| CliError::new(format!("未知方言: {raw}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Command;

    fn command() -> Command {
        (cli_tool().configure)(Command::new("sqlFormatter"))
    }

    #[test]
    fn language_parser_accepts_all_dialects_and_default() {
        command()
            .try_get_matches_from(["sqlFormatter", "-i", "select 1"])
            .expect("default language should be Sql");
        for lang in SqlLanguage::ALL {
            command()
                .try_get_matches_from([
                    "sqlFormatter",
                    "-i",
                    "select 1",
                    "--language",
                    lang.as_str(),
                ])
                .unwrap_or_else(|e| panic!("{} should be accepted: {e}", lang.as_str()));
        }
    }

    #[test]
    fn language_parser_rejects_unknown_dialect() {
        for name in ["Oracle", "sql", "Postgres"] {
            let err = command()
                .try_get_matches_from(["sqlFormatter", "-i", "select 1", "--language", name])
                .expect_err(name);
            let rendered = err.to_string();
            assert!(
                rendered.contains(name),
                "CLI must reject {name} clearly, got: {rendered}"
            );
        }
        assert!(
            parse_language(Some("Oracle"))
                .unwrap_err()
                .message
                .contains("未知方言")
        );
    }

    #[test]
    fn leading_comma_flag_is_accepted() {
        let matches = command()
            .try_get_matches_from(["sqlFormatter", "-i", "select 1", "--leadingComma"])
            .expect("CLI --leadingComma should be accepted");
        assert!(matches.get_flag("leadingComma"));
        let matches = command()
            .try_get_matches_from(["sqlFormatter", "-i", "select 1"])
            .expect("leadingComma defaults to off");
        assert!(!matches.get_flag("leadingComma"));
    }
}
