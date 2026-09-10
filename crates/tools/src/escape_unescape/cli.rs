use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{convert, Conversion, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "escape",
        aliases: &["esc"],
        about: "C 风格 / JSON 字符串转义或反转义",
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
    let result = convert(&source, conversion).map_err(|_| CliError::new("非法转义序列"))?;
    write_output(
        matches.get_one::<String>("output").map(String::as_str),
        &result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_unescapes_known_and_preserves_unknown() {
        let tool = cli_tool();
        let cmd = (tool.configure)(Command::new("test"));
        let matches = cmd.get_matches_from(["test", "-i", "hello\\q\\nworld\\", "-c", "Decode"]);
        assert!((tool.run)(&matches).is_ok());
    }

    #[test]
    fn cli_decode_invalid_sequence_returns_error() {
        let tool = cli_tool();
        let cmd = (tool.configure)(Command::new("test"));
        let matches = cmd.get_matches_from(["test", "-i", "hello\\uZZZZ", "-c", "Decode"]);
        assert!((tool.run)(&matches).is_err());
    }
}
