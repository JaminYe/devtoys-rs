use std::fs;
use std::io::{self, Write};
use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{write_output, CliError, CliTool};

use super::{decode_base64, encode_bytes, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "base64img",
        aliases: &["b64i"],
        about: "图片与 Base64 互转",
        configure: configure,
        run: run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("图片文件路径或 Base64 / data URI"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出文件；编码时省略则写到 stdout"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let output = matches.get_one::<String>("output").map(String::as_str);
    let path = Path::new(input);
    if path.is_file() {
        let bytes = fs::read(path).map_err(|_| CliError::new("无法读取输入文件"))?;
        let encoded = encode_bytes(&bytes);
        write_output(output, &encoded)
    } else {
        let bytes = decode_base64(input).map_err(|err| CliError::new(err.to_string()))?;
        match output {
            Some(path) => fs::write(path, bytes).map_err(|_| CliError::new("无法写入输出文件")),
            None => {
                let mut stdout = io::stdout().lock();
                stdout
                    .write_all(&bytes)
                    .map_err(|_| CliError::new("无法写入标准输出"))
            }
        }
    }
}
