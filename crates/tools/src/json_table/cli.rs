use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{json_to_table, TableFormat, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "jsonToTable",
        aliases: &[],
        about: "JSON 对象数组转表格",
        configure,
        run,
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
        Arg::new("format")
            .long("format")
            .value_parser(["TSV", "CSV", "FSV"])
            .default_value("CSV"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let format = parse_format(matches.get_one::<String>("format").map(String::as_str))?;
    let table = json_to_table(&source, format).map_err(|err| CliError::new(err.to_string()))?;
    write_output(matches.get_one::<String>("output").map(String::as_str), &table)
}

fn parse_format(value: Option<&str>) -> Result<TableFormat, CliError> {
    TableFormat::parse(value.unwrap_or("CSV")).ok_or_else(|| CliError::new("未知表格格式"))
}
