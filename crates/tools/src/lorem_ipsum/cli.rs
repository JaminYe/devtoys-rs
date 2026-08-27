use clap::{Arg, ArgMatches, Command};

use crate::cli::{write_output, CliError, CliTool};

use super::{generate_lorem, Corpus, LoremUnit, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "loremipsum",
        aliases: &["li"],
        about: "生成乱数假文",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("corpus")
            .short('c')
            .value_parser([
                "LoremIpsum",
                "ChildHarold",
                "Decameron",
                "Faust",
                "InDerFremde",
                "LeBateauIvre",
                "LeMasque",
                "NagyonFaj",
                "Omagyar",
                "RobinsonoKruso",
                "TheRaven",
                "TierrayLuna",
            ])
            .default_value("LoremIpsum"),
    )
    .arg(
        Arg::new("type")
            .short('t')
            .value_parser(["Paragraphs", "Sentences", "Words", "Characters"])
            .default_value("Paragraphs"),
    )
    .arg(
        Arg::new("length")
            .short('l')
            .value_parser(clap::value_parser!(usize))
            .default_value("1"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let corpus = Corpus::parse(
        matches
            .get_one::<String>("corpus")
            .map(String::as_str)
            .unwrap_or("LoremIpsum"),
    )
    .ok_or_else(|| CliError::new("未知语料"))?;
    let unit = LoremUnit::parse(
        matches
            .get_one::<String>("type")
            .map(String::as_str)
            .unwrap_or("Paragraphs"),
    )
    .ok_or_else(|| CliError::new("未知单位"))?;
    let length = matches.get_one::<usize>("length").copied().unwrap_or(1);
    let text =
        generate_lorem(corpus, unit, length).map_err(|err| CliError::new(err.to_string()))?;
    write_output(None, &text)
}
