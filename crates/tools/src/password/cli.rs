use clap::{Arg, ArgAction, ArgMatches, Command};
use rand::thread_rng;

use crate::cli::{write_output, CliError, CliTool};

use super::{generate_password, PasswordOptions, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "password",
        aliases: &["pwd"],
        about: "按字符集生成随机密码",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("length")
            .short('l')
            .value_parser(clap::value_parser!(usize))
            .default_value("30"),
    )
    .arg(bool_flag(
        "uppercase",
        'u',
        "uppercase",
        "包含大写字母",
        true,
    ))
    .arg(bool_flag(
        "lowercase",
        'm',
        "lowercase",
        "包含小写字母",
        true,
    ))
    .arg(bool_flag("digits", 'd', "digits", "包含数字", true))
    .arg(bool_flag("special", 's', "special", "包含特殊字符", true))
    .arg(
        Arg::new("exclude")
            .short('e')
            .long("exclude")
            .help("排除字符"),
    )
    .arg(
        Arg::new("count")
            .long("count")
            .value_parser(clap::value_parser!(usize))
            .default_value("1"),
    )
}

fn bool_flag(
    name: &'static str,
    short: char,
    long: &'static str,
    help: &'static str,
    default: bool,
) -> Arg {
    Arg::new(name)
        .short(short)
        .long(long)
        .help(help)
        .action(ArgAction::Set)
        .value_parser(["true", "false"])
        .num_args(0..=1)
        .default_value(if default { "true" } else { "false" })
        .default_missing_value("true")
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let options = PasswordOptions {
        length: matches.get_one::<usize>("length").copied().unwrap_or(30),
        uppercase: flag(matches, "uppercase", true),
        lowercase: flag(matches, "lowercase", true),
        digits: flag(matches, "digits", true),
        special: flag(matches, "special", true),
        exclude: matches
            .get_one::<String>("exclude")
            .cloned()
            .unwrap_or_default(),
        count: matches.get_one::<usize>("count").copied().unwrap_or(1),
    };
    let mut rng = thread_rng();
    let password =
        generate_password(&options, &mut rng).map_err(|err| CliError::new(err.to_string()))?;
    write_output(None, &password)
}

fn flag(matches: &ArgMatches, name: &str, default: bool) -> bool {
    matches
        .get_one::<String>(name)
        .map(|value| value == "true")
        .unwrap_or(default)
}
