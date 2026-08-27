use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{write_output, CliError, CliTool};

use super::{generate_uuid, UuidOptions, UuidVersion, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "uuid",
        aliases: &["guid"],
        about: "生成 UUID",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("version")
            .short('v')
            .value_parser(["One", "Four", "Seven"])
            .default_value("Four"),
    )
    .arg(
        Arg::new("hyphens")
            .short('h')
            .long("hyphens")
            .action(ArgAction::Set)
            .value_parser(["true", "false"])
            .num_args(0..=1)
            .default_value("true")
            .default_missing_value("true"),
    )
    .arg(
        Arg::new("uppercase")
            .short('u')
            .long("uppercase")
            .action(ArgAction::SetTrue),
    )
    .arg(
        Arg::new("count")
            .short('c')
            .long("count")
            .value_parser(clap::value_parser!(usize))
            .default_value("1"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let version = UuidVersion::parse(
        matches
            .get_one::<String>("version")
            .map(String::as_str)
            .unwrap_or("Four"),
    )
    .ok_or_else(|| CliError::new("未知版本"))?;
    let hyphens = matches
        .get_one::<String>("hyphens")
        .map(|value| value == "true")
        .unwrap_or(true);
    let options = UuidOptions {
        version,
        hyphens,
        uppercase: matches.get_flag("uppercase"),
        count: matches.get_one::<usize>("count").copied().unwrap_or(1),
    };
    let text = generate_uuid(&options).map_err(|err| CliError::new(err.to_string()))?;
    write_output(None, &text)
}
