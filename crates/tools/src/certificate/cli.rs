use std::fs;
use std::path::Path;

use clap::{Arg, ArgMatches, Command};

use crate::cli::{write_output, CliError, CliTool};

use super::{decode_certificate, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "certificate",
        aliases: &["cert"],
        about: "解码 PEM / CER / CRT 证书",
        configure: configure,
        run: run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("输入文件路径或内联 PEM"),
    )
    .arg(Arg::new("password").short('p').help("PFX 密码"))
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
    let password = matches.get_one::<String>("password").map(String::as_str);
    let bytes = read_bytes(input)?;
    let result =
        decode_certificate(&bytes, password).map_err(|err| CliError::new(err.to_string()))?;
    write_output(
        matches.get_one::<String>("output").map(String::as_str),
        &result,
    )
}

fn read_bytes(input: &str) -> Result<Vec<u8>, CliError> {
    let path = Path::new(input);
    if path.is_file() {
        fs::read(path).map_err(|_| CliError::new("无法读取输入文件"))
    } else {
        Ok(input.as_bytes().to_vec())
    }
}
