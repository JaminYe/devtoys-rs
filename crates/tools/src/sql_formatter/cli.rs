use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{format_sql, Indentation, SqlLanguage, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "sqlFormatter",
        aliases: &["sqlf"],
        about: "按方言美化 SQL",
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
            .value_parser([
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
            ])
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
