use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{format_json, Indentation, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "JsonFormatter",
        aliases: &["Jsonf"],
        about: "美化或压缩 JSON",
        configure: configure,
        run: run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("输入文件路径；若路径不是已有文件则当作内联 JSON"),
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
        Arg::new("sortProperties")
            .long("sortProperties")
            .alias("sort-properties")
            .action(ArgAction::SetTrue),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let indent = parse_indent(matches.get_one::<String>("indentation").map(String::as_str))?;
    let sort = matches.get_flag("sortProperties");
    let formatted = format_json(&source, indent, sort).map_err(|_| CliError::new("非法 JSON"))?;
    write_output(matches.get_one::<String>("output").map(String::as_str), &formatted)
}

fn parse_indent(value: Option<&str>) -> Result<Indentation, CliError> {
    match value.unwrap_or("TwoSpaces") {
        "TwoSpaces" => Ok(Indentation::TwoSpaces),
        "FourSpaces" => Ok(Indentation::FourSpaces),
        "OneTab" => Ok(Indentation::OneTab),
        "Minified" => Ok(Indentation::Minified),
        other => Err(CliError::new(format!("未知缩进: {other}"))),
    }
}
