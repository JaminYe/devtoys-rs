use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{write_output, CliError, CliTool};

use super::{checksum_matches_input, compute_hash, HashAlgorithm, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "checksum",
        aliases: &["hash"],
        about: "计算文本或文件的哈希 / 校验和",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("文本；若路径存在或设置 -s 则按文件字节哈希"),
    )
    .arg(
        Arg::new("algorithm")
            .short('a')
            .value_parser(["Md5", "Sha1", "Sha256", "Sha384", "Sha512"])
            .default_value("Md5"),
    )
    .arg(
        Arg::new("uppercase")
            .short('u')
            .long("uppercase")
            .action(ArgAction::SetTrue)
            .help("输出大写十六进制"),
    )
    .arg(
        Arg::new("hmac")
            .short('m')
            .long("hmac")
            .help("HMAC 密钥；提供则计算 HMAC"),
    )
    .arg(
        Arg::new("checksum")
            .short('c')
            .long("checksum")
            .help("期望校验和或校验和文件路径（大小写不敏感）"),
    )
    .arg(
        Arg::new("as-file")
            .short('s')
            .long("file")
            .action(ArgAction::SetTrue)
            .help("将 -i 视为文件路径"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let algorithm = HashAlgorithm::parse(
        matches
            .get_one::<String>("algorithm")
            .map(String::as_str)
            .unwrap_or("Md5"),
    )
    .ok_or_else(|| CliError::new("未知算法"))?;
    let uppercase = matches.get_flag("uppercase");
    let hmac = matches.get_one::<String>("hmac").map(String::as_str);
    let as_file = matches.get_flag("as-file");
    let hex = compute_hash(input, algorithm, hmac, uppercase, as_file)
        .map_err(|err| CliError::new(err.to_string()))?;
    write_output(None, &hex)?;
    if let Some(expected) = matches.get_one::<String>("checksum") {
        if !checksum_matches_input(&hex, expected) {
            return Err(CliError::new("校验和不匹配"));
        }
    }
    Ok(())
}
