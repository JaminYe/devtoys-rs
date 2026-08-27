use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};
use crate::indent::Indentation;

use super::{convert_json_yaml, Conversion, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "jsonToYaml",
        aliases: &[],
        about: "JSON 与 YAML 互转",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("输入文件路径；若路径不是已有文件则当作内联文本"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出文件；省略则写到 stdout"),
    )
    .arg(
        Arg::new("conversion")
            .short('c')
            .value_parser(["JsonToYaml", "YamlToJson"])
            .default_value("JsonToYaml"),
    )
    .arg(
        Arg::new("indentation")
            .long("indentation")
            .value_parser(["TwoSpaces", "FourSpaces", "OneTab", "Minified"])
            .default_value("TwoSpaces"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let direction = parse_conversion(matches.get_one::<String>("conversion").map(String::as_str))?;
    let indent = parse_indent(matches.get_one::<String>("indentation").map(String::as_str))?;
    let converted = convert_json_yaml(&source, direction, indent)
        .map_err(|err| CliError::new(err.to_string()))?;
    write_output(matches.get_one::<String>("output").map(String::as_str), &converted)
}

fn parse_conversion(value: Option<&str>) -> Result<Conversion, CliError> {
    Conversion::parse(value.unwrap_or("JsonToYaml")).ok_or_else(|| CliError::new("未知转换方向"))
}

fn parse_indent(value: Option<&str>) -> Result<Indentation, CliError> {
    Indentation::parse(value.unwrap_or("TwoSpaces")).ok_or_else(|| CliError::new("未知缩进"))
}
