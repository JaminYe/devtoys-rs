use std::fs;
use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{decode_image_path, encode_png, encode_svg, is_existing_image_file, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "qrcode",
        aliases: &[],
        about: "二维码编解码",
        configure: configure,
        run: run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("文本，或已有图像文件路径"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出文件；省略则写到 stdout"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let output = matches.get_one::<String>("output").map(String::as_str);

    if is_existing_image_file(input) {
        let text = decode_image_path(Path::new(input))
            .map_err(|err| CliError::new(err.to_string()))?;
        write_output(output, &text)
    } else {
        let source = read_input(input)?;
        match output {
            Some(path) if path.to_ascii_lowercase().ends_with(".svg") => {
                let svg = encode_svg(&source).map_err(|err| CliError::new(err.to_string()))?;
                write_output(Some(path), &svg)
            }
            Some(path) => {
                let png = encode_png(&source).map_err(|err| CliError::new(err.to_string()))?;
                fs::write(path, png).map_err(|_| CliError::new("无法写入输出文件"))
            }
            None => {
                let svg = encode_svg(&source).map_err(|err| CliError::new(err.to_string()))?;
                write_output(None, &svg)
            }
        }
    }
}
