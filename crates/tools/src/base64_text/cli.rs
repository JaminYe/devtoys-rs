use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{convert, Charset, Conversion, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "base64",
        aliases: &["b64"],
        about: "文本与 RFC 4648 Base64 互转",
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
        Arg::new("output")
            .short('o')
            .help("输出文件；省略则写到 stdout"),
    )
    .arg(
        Arg::new("conversion")
            .short('c')
            .value_parser(["Encode", "Decode"])
            .default_value("Encode"),
    )
    .arg(
        Arg::new("encoding")
            .short('e')
            .value_parser(["Utf8", "Ascii"])
            .default_value("Utf8"),
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
    let charset = Charset::parse(
        matches
            .get_one::<String>("encoding")
            .map(String::as_str)
            .unwrap_or("Utf8"),
    )
    .ok_or_else(|| CliError::new("未知字符集"))?;
    let result = convert(&source, conversion, charset, false)
        .map_err(|err| CliError::new(err.to_string()))?;
    write_output(
        matches.get_one::<String>("output").map(String::as_str),
        &result,
    )
}
