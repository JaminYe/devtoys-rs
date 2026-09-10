use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, write_output, CliError, CliTool};

use super::{convert_base, convert_rfc4648, NumberBase, Rfc4648Encoding, Signedness, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "numberbase",
        aliases: &["nb"],
        about: "数字进制转换",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("input")
            .short('i')
            .required(true)
            .allow_hyphen_values(true)
            .help("输入值；若路径不是已有文件则当作内联文本"),
    )
    .arg(
        Arg::new("from")
            .short('b')
            .value_parser(["Decimal", "Octal", "Hexadecimal", "Binary"])
            .default_value("Decimal")
            .help("输入进制"),
    )
    .arg(
        Arg::new("to")
            .short('o')
            .value_parser(["Decimal", "Octal", "Hexadecimal", "Binary"])
            .default_value("Hexadecimal")
            .help("输出进制"),
    )
    .arg(
        Arg::new("signedness")
            .long("signedness")
            .value_parser(["Signed", "Unsigned"])
            .default_value("Signed")
            .help("基础模式有符号（64 位补码）或无符号 64 位；高级 RFC 路径忽略此参数"),
    )
    .arg(
        Arg::new("from_rfc")
            .long("from-rfc")
            .value_parser(["Base16", "Base32", "Base32Hex", "Base64", "Base64Url"])
            .help("高级模式输入进制（无符号 64 位整数换基）"),
    )
    .arg(
        Arg::new("to_rfc")
            .long("to-rfc")
            .value_parser(["Base16", "Base32", "Base32Hex", "Base64", "Base64Url"])
            .help("高级模式输出进制（无符号 64 位整数换基）"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let input = matches
        .get_one::<String>("input")
        .ok_or_else(|| CliError::new("缺少输入"))?;
    let source = read_input(input)?;
    let converted = convert_source(&source, matches)?;
    write_output(None, &converted)
}

fn convert_source(source: &str, matches: &ArgMatches) -> Result<String, CliError> {
    let from_rfc = matches.get_one::<String>("from_rfc").map(String::as_str);
    let to_rfc = matches.get_one::<String>("to_rfc").map(String::as_str);
    match (from_rfc, to_rfc) {
        (None, None) => {
            let from = parse_base(
                matches.get_one::<String>("from").map(String::as_str),
                NumberBase::Decimal,
            )?;
            let to = parse_base(
                matches.get_one::<String>("to").map(String::as_str),
                NumberBase::Hexadecimal,
            )?;
            let signedness =
                parse_signedness(matches.get_one::<String>("signedness").map(String::as_str))?;
            convert_base(source, from, to, signedness).map_err(|err| CliError::new(err.to_string()))
        }
        (Some(from), Some(to)) => {
            let from = parse_rfc(from)?;
            let to = parse_rfc(to)?;
            convert_rfc4648(source, from, to).map_err(|err| CliError::new(err.to_string()))
        }
        _ => Err(CliError::new("高级模式需同时指定 --from-rfc 与 --to-rfc")),
    }
}

fn parse_base(value: Option<&str>, default: NumberBase) -> Result<NumberBase, CliError> {
    match value {
        None => Ok(default),
        Some(text) => NumberBase::parse(text).ok_or_else(|| CliError::new("未知进制")),
    }
}

fn parse_rfc(value: &str) -> Result<Rfc4648Encoding, CliError> {
    Rfc4648Encoding::parse(value).ok_or_else(|| CliError::new("未知进制"))
}

fn parse_signedness(value: Option<&str>) -> Result<Signedness, CliError> {
    match value {
        None => Ok(Signedness::Signed),
        Some(text) => Signedness::parse(text).ok_or_else(|| CliError::new("未知符号模式")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches_from(args: &[&str]) -> ArgMatches {
        configure(Command::new("numberbase"))
            .try_get_matches_from(args)
            .expect("cli args")
    }

    #[test]
    fn rfc_flags_convert_ff_to_base64() {
        let matches = matches_from(&[
            "numberbase",
            "-i",
            "FF",
            "--from-rfc",
            "Base16",
            "--to-rfc",
            "Base64",
        ]);
        assert_eq!(convert_source("FF", &matches).unwrap(), "D/");
    }

    #[test]
    fn rfc_flags_reverse_and_basic_minus_one() {
        let rfc = matches_from(&[
            "numberbase",
            "-i",
            "D/",
            "--from-rfc",
            "Base64",
            "--to-rfc",
            "Base16",
        ]);
        assert_eq!(convert_source("D/", &rfc).unwrap(), "FF");
        let basic = matches_from(&[
            "numberbase",
            "-i",
            "-1",
            "-b",
            "Decimal",
            "-o",
            "Hexadecimal",
        ]);
        assert_eq!(convert_source("-1", &basic).unwrap(), "FFFFFFFFFFFFFFFF");
    }

    #[test]
    fn signedness_flag_covers_both_modes_default_signed() {
        let default_signed = matches_from(&[
            "numberbase",
            "-i",
            "-1",
            "-b",
            "Decimal",
            "-o",
            "Hexadecimal",
        ]);
        assert_eq!(
            convert_source("-1", &default_signed).unwrap(),
            "FFFFFFFFFFFFFFFF"
        );

        let explicit_signed = matches_from(&[
            "numberbase",
            "-i",
            "-1",
            "-b",
            "Decimal",
            "-o",
            "Hexadecimal",
            "--signedness",
            "Signed",
        ]);
        assert_eq!(
            convert_source("-1", &explicit_signed).unwrap(),
            "FFFFFFFFFFFFFFFF"
        );

        let unsigned_hex = matches_from(&[
            "numberbase",
            "-i",
            "FFFFFFFFFFFFFFFF",
            "-b",
            "Hexadecimal",
            "-o",
            "Decimal",
            "--signedness",
            "Unsigned",
        ]);
        assert_eq!(
            convert_source("FFFFFFFFFFFFFFFF", &unsigned_hex).unwrap(),
            "18446744073709551615"
        );

        let unsigned_round_trip = matches_from(&[
            "numberbase",
            "-i",
            "18446744073709551615",
            "-b",
            "Decimal",
            "-o",
            "Hexadecimal",
            "--signedness",
            "Unsigned",
        ]);
        let hex = convert_source("18446744073709551615", &unsigned_round_trip).unwrap();
        assert_eq!(hex, "FFFFFFFFFFFFFFFF");
        assert!(!hex.contains('-'));

        let rfc_ignores_signedness = matches_from(&[
            "numberbase",
            "-i",
            "FF",
            "--from-rfc",
            "Base16",
            "--to-rfc",
            "Base64",
            "--signedness",
            "Unsigned",
        ]);
        assert_eq!(convert_source("FF", &rfc_ignores_signedness).unwrap(), "D/");
    }

    #[test]
    fn rfc_requires_both_flags() {
        let matches = matches_from(&["numberbase", "-i", "FF", "--from-rfc", "Base16"]);
        let err = convert_source("FF", &matches).unwrap_err();
        assert!(err.message.contains("--from-rfc"));
        assert!(err.message.contains("--to-rfc"));
    }
}
