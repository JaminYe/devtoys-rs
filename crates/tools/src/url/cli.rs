use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{convert, Conversion, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "url",
        aliases: &[],
        about: "URL percent-encoding 编解码",
        configure: configure,
        run: run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("输入文本；若路径是已有文件则读取该文件"),
    )
    .arg(
        Arg::new("conversion")
            .short('c')
            .value_parser(["Encode", "Decode"])
            .default_value("Encode"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let conversion = Conversion::parse(
        matches
            .get_one::<String>("conversion")
            .map(String::as_str)
            .unwrap_or("Encode"),
    )
    .ok_or_else(|| CliError::new("未知转换模式"))?;
    let output =
        convert(&source, conversion, false).map_err(|err| CliError::new(err.to_string()))?;
    write_output(None, &output)
}
