use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{convert, GzipMode, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "gzip",
        aliases: &[],
        about: "文本 GZip 压缩 / 解压（Base64 传输）",
        configure: configure,
        run: run,
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
        Arg::new("mode")
            .short('m')
            .value_parser(["Compress", "Decompress"])
            .default_value("Compress"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let mode = GzipMode::parse(
        matches
            .get_one::<String>("mode")
            .map(String::as_str)
            .unwrap_or("Compress"),
    )
    .ok_or_else(|| CliError::new("未知模式"))?;
    let result = convert(&source, mode).map_err(|err| CliError::new(err.to_string()))?;
    write_output(None, &result)
}
