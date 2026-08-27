use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{convert_base, NumberBase, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "numberbase",
        aliases: &["nb"],
        about: "数字进制转换",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("输入值；若路径不是已有文件则当作内联文本"),
    )
    .arg(
        Arg::new("from")
            .short('b')
            .value_parser(["Decimal", "Octal", "Hexadecimal", "Binary"])
            .default_value("Decimal")
            .help("输入进制"),
    )
    .arg(
        Arg::new("to")
            .short('o')
            .value_parser(["Decimal", "Octal", "Hexadecimal", "Binary"])
            .default_value("Hexadecimal")
            .help("输出进制"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let from = parse_base(
        matches.get_one::<String>("from").map(String::as_str),
        NumberBase::Decimal,
    )?;
    let to = parse_base(
        matches.get_one::<String>("to").map(String::as_str),
        NumberBase::Hexadecimal,
    )?;
    let converted =
        convert_base(&source, from, to).map_err(|err| CliError::new(err.to_string()))?;
    write_output(None, &converted)
}

fn parse_base(value: Option<&str>, default: NumberBase) -> Result<NumberBase, CliError> {
    match value {
        None => Ok(default),
        Some(text) => NumberBase::parse(text).ok_or_else(|| CliError::new("未知进制")),
    }
}
