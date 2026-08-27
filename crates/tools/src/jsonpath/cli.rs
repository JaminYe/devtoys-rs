use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{eval_jsonpath, JsonPathError, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "jsonpathtester",
        aliases: &["jpt"],
        about: "对 JSON 执行 JSONPath，列出匹配",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("json")
            .short('j')
            .required(true)
            .help("JSON 文本或文件路径"),
    )
    .arg(
        Arg::new("path")
            .short('p')
            .required(true)
            .help("JSONPath 表达式或含表达式的文件"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出文件；省略则写到 stdout"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let json = matches
        .get_one::<String>("json")
        .ok_or_else(|| CliError::new("缺少 JSON"))?;
    let path = matches
        .get_one::<String>("path")
        .ok_or_else(|| CliError::new("缺少 JSONPath"))?;
    let json = read_input(json)?;
    let path = read_input(path)?;
    let result = eval_jsonpath(&json, &path).map_err(map_err)?;
    write_output(matches.get_one::<String>("output").map(String::as_str), &result)
}

fn map_err(err: JsonPathError) -> CliError {
    CliError::new(err.to_string())
}
