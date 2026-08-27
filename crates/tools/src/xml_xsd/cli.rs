use clap::{Arg, ArgMatches, Command};

use crate::cli::{read_input, CliError, CliTool};

use super::{format_reports, validate_xml_xsd, XmlReportLevel, ID};

pub fn cli_tool() -> CliTool {
    CliTool {
        tool_id: ID,
        name: "xmltester",
        aliases: &["xsd"],
        about: "用 XSD 校验 XML",
        configure,
        run,
    }
}

fn configure(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("xsd")
            .short('s')
            .required(true)
            .help("XSD 文本或文件路径"),
    )
    .arg(
        Arg::new("xml")
            .short('x')
            .required(true)
            .help("XML 文本或文件路径"),
    )
}

fn run(matches: &ArgMatches) -> Result<(), CliError> {
    let xsd = matches
        .get_one::<String>("xsd")
        .ok_or_else(|| CliError::new("缺少 XSD"))?;
    let xml = matches
        .get_one::<String>("xml")
        .ok_or_else(|| CliError::new("缺少 XML"))?;
    let xsd = read_input(xsd)?;
    let xml = read_input(xml)?;
    let reports = validate_xml_xsd(&xml, &xsd);
    let text = format_reports(&reports);
    if reports.iter().any(|r| r.level == XmlReportLevel::Error) {
        return Err(CliError::new(text));
    }
    println!("{text}");
    Ok(())
}
