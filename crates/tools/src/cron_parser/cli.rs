use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{write_output, CliError, CliTool};

use super::{parse_cron, DEFAULT_DATE_FORMAT, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "cronparser",
        aliases: &["cron"],
        about: "解析 Cron 表达式",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("expression")
            .short('e')
            .required(true)
            .help("Cron 表达式"),
    )
    .arg(
        Arg::new("seconds")
            .short('s')
            .action(ArgAction::SetTrue)
            .help("包含秒字段"),
    )
    .arg(
        Arg::new("count")
            .short('c')
            .default_value("5")
            .help("预览条数"),
    )
    .arg(
        Arg::new("date_format")
            .short('d')
            .default_value(DEFAULT_DATE_FORMAT)
            .help("日期格式"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let expression = matches
        .get_one::<String>("expression")
        .ok_or_else(|| CliError::new("缺少表达式"))?;
    let include_seconds = matches.get_flag("seconds");
    let count = parse_count(matches.get_one::<String>("count").map(String::as_str))?;
    let date_format = matches
        .get_one::<String>("date_format")
        .map(String::as_str)
        .unwrap_or(DEFAULT_DATE_FORMAT);
    let result = parse_cron(expression, include_seconds, count, date_format)
        .map_err(|err| CliError::new(err.to_string()))?;
    write_output(None, &result.display_text())
}

fn parse_count(value: Option<&str>) -> Result<usize, CliError> {
    value
        .unwrap_or("5")
        .parse::<usize>()
        .map_err(|_| CliError::new("预览条数无效"))
}
