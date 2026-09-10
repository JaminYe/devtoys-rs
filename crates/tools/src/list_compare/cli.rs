use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{compare_lists, ListMode, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "listcompare",
        aliases: &["lc"],
        about: "两列表求交 / 并 / 差",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("listA")
            .short('a')
            .required(true)
            .help("列表 A 文件路径；若路径不是已有文件则当作内联文本"),
    )
    .arg(
        Arg::new("listB")
            .short('b')
            .required(true)
            .help("列表 B 文件路径；若路径不是已有文件则当作内联文本"),
    )
    .arg(
        Arg::new("comparisonMode")
            .long("cm")
            .short('m')
            .value_parser(["AInterB", "AUnionB", "AOnly", "BOnly"])
            .default_value("AInterB")
            .help("比对模式：AInterB / AUnionB / AOnly / BOnly"),
    )
    .arg(
        Arg::new("caseSensitive")
            .long("cs")
            .short('s')
            .action(ArgAction::SetTrue)
            .help("大小写敏感"),
    )
    .arg(
        Arg::new("ignoreSurroundingWhitespace")
            .long("ignore-surrounding-whitespace")
            .visible_alias("isw")
            .short('w')
            .action(ArgAction::SetTrue)
            .help("忽略首尾空白（默认关闭）"),
    )
    .arg(
        Arg::new("output")
            .short('o')
            .help("输出文件；省略则写到 stdout"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let a_path = matches
        .get_one::<String>("listA")
        .ok_or_else(|| CliError::new("缺少列表 A"))?;
    let b_path = matches
        .get_one::<String>("listB")
        .ok_or_else(|| CliError::new("缺少列表 B"))?;
    let a = read_input(a_path)?;
    let b = read_input(b_path)?;
    let mode = ListMode::parse(
        matches
            .get_one::<String>("comparisonMode")
            .map(String::as_str)
            .unwrap_or("AInterB"),
    )
    .ok_or_else(|| CliError::new("未知比对模式"))?;
    let case_sensitive = matches.get_flag("caseSensitive");
    let ignore_surrounding_whitespace = matches.get_flag("ignoreSurroundingWhitespace");
    let result = compare_lists(&a, &b, mode, case_sensitive, ignore_surrounding_whitespace);
    write_output(
        matches.get_one::<String>("output").map(String::as_str),
        &result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command() -> Command {
        (cli_tool().configure)(Command::new("listcompare"))
    }

    #[test]
    fn ignore_surrounding_whitespace_defaults_off_independent_of_case() {
        let matches = command()
            .try_get_matches_from(["listcompare", "-a", "a", "-b", "b"])
            .expect("required lists should parse");
        assert!(!matches.get_flag("ignoreSurroundingWhitespace"));
        assert!(!matches.get_flag("caseSensitive"));
    }

    #[test]
    fn ignore_surrounding_whitespace_flag_does_not_enable_case_sensitive() {
        for flag in ["--ignore-surrounding-whitespace", "--isw", "-w"] {
            let matches = command()
                .try_get_matches_from(["listcompare", "-a", " a ", "-b", "a", flag])
                .unwrap_or_else(|err| panic!("{flag} should parse: {err}"));
            assert!(
                matches.get_flag("ignoreSurroundingWhitespace"),
                "{flag} should set ignoreSurroundingWhitespace"
            );
            assert!(
                !matches.get_flag("caseSensitive"),
                "{flag} must not enable caseSensitive"
            );
        }
    }
}
