use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::helper::{apply_transform, Transform};

pub(super) fn cli_tool(tool_id: &'static str, name: &'static str, about: &'static str) -> CliTool {
    CliTool {
        tool_id,
        name,
        aliases: &[],
        about,
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
        Arg::new("mode")
            .short('m')
            .value_parser(["echo", "uppercase"])
            .default_value("uppercase"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let mode = Transform::parse(
        matches
            .get_one::<String>("mode")
            .map(String::as_str)
            .unwrap_or("uppercase"),
    )
    .ok_or_else(|| CliError::new("未知转换模式"))?;
    write_output(
        matches.get_one::<String>("output").map(String::as_str),
        &apply_transform(&source, mode),
    )
}
