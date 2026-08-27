use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{apply, Operation, OPERATION_NAMES, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "textutilities",
        aliases: &["txt"],
        about: "文本统计与大小写/换行/行排序变换",
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
        Arg::new("action")
            .short('a')
            .required(true)
            .action(ArgAction::Append)
            .value_parser(PossibleValuesParser::new(OPERATION_NAMES))
            .help("一个或多个操作（可重复 -a）"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let ops = matches
        .get_many::<String>("action")
        .ok_or_else(|| CliError::new("缺少操作"))?
        .map(|name| {
            Operation::parse(name).ok_or_else(|| CliError::new(format!("未知操作: {name}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rng = rand::thread_rng();
    let result = apply(&source, &ops, &mut rng);
    write_output(
        matches.get_one::<String>("output").map(String::as_str),
        &result,
    )
}
