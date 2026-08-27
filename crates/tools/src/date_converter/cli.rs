use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{datetime_to_timestamp, timestamp_to_datetime, TimestampFormat, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "date",
        aliases: &[],
        about: "Unix 时间戳与日期互转",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .help("时间戳或日期；若路径不是已有文件则当作内联文本"),
    )
    .arg(Arg::new("epoch").short('e').help("自定义纪元"))
    .arg(
        Arg::new("timezone")
            .long("tz")
            .short('z')
            .help("时区（--tz / -z）"),
    )
    .arg(
        Arg::new("format")
            .short('f')
            .value_parser(["Ticks", "Seconds", "Milliseconds"])
            .default_value("Seconds"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let format = parse_format(matches.get_one::<String>("format").map(String::as_str))?;
    let timezone = matches.get_one::<String>("timezone").map(String::as_str);
    let epoch = matches.get_one::<String>("epoch").map(String::as_str);

    let output = if is_timestamp(&source) {
        timestamp_to_datetime(&source, format, timezone, epoch)
    } else {
        datetime_to_timestamp(&source, format, timezone, epoch)
    }
    .map_err(|err| CliError::new(err.to_string()))?;

    write_output(None, &output)
}

fn parse_format(value: Option<&str>) -> Result<TimestampFormat, CliError> {
    TimestampFormat::parse(value.unwrap_or("Seconds")).ok_or_else(|| CliError::new("未知时间格式"))
}

fn is_timestamp(value: &str) -> bool {
    let t = value.trim();
    let digits = t.strip_prefix('-').unwrap_or(t);
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}
