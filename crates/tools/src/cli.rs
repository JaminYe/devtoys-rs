use std::fs;
use std::io::{self, Write};
use std::path::Path;

use clap::{ArgMatches, Command};

#[derive(Debug)]
pub struct CliError {
    pub message: String,
}

impl CliError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// One CLI subcommand, registered next to the GUI metadata of the same tool.
pub struct CliTool {
    pub tool_id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub about: &'static str,
    pub configure: fn(Command) -> Command,
    pub run: fn(&ArgMatches) -> Result<(), CliError>,
}

pub fn build_cli() -> Command {
    build_cli_from(crate::default_catalog())
}

/// Builds CLI subcommands from `catalog` (builtins and any loaded extensions).
pub fn build_cli_from(catalog: &crate::ToolCatalog) -> Command {
    let mut cmd = Command::new("devtoys-cli")
        .about("DevToys CLI：调用工具 Helper，不启动 GUI")
        .subcommand_required(false);
    for tool in catalog.all_cli() {
        let mut sub = Command::new(tool.name).about(tool.about);
        for alias in tool.aliases {
            sub = sub.visible_alias(*alias);
        }
        cmd = cmd.subcommand((tool.configure)(sub));
    }
    cmd
}

pub fn run_cli(matches: &ArgMatches) -> Result<(), CliError> {
    run_cli_from(crate::default_catalog(), matches)
}

/// Dispatches a parsed CLI invocation against `catalog`.
pub fn run_cli_from(catalog: &crate::ToolCatalog, matches: &ArgMatches) -> Result<(), CliError> {
    let Some((name, sub)) = matches.subcommand() else {
        return Ok(());
    };
    let tool = catalog
        .all_cli()
        .into_iter()
        .find(|tool| tool.name == name)
        .ok_or_else(|| CliError::new(format!("未知命令: {name}")))?;
    (tool.run)(sub)
}

pub fn read_input(input: &str) -> Result<String, CliError> {
    let path = Path::new(input);
    if path.is_file() {
        fs::read_to_string(path).map_err(|_| CliError::new("无法读取输入文件"))
    } else {
        Ok(input.to_string())
    }
}

pub fn write_output(output: Option<&str>, formatted: &str) -> Result<(), CliError> {
    match output {
        Some(path) => fs::write(path, formatted).map_err(|_| CliError::new("无法写入输出文件")),
        None => {
            let mut stdout = io::stdout().lock();
            stdout
                .write_all(formatted.as_bytes())
                .map_err(|_| CliError::new("无法写入标准输出"))?;
            if !formatted.ends_with('\n') {
                stdout
                    .write_all(b"\n")
                    .map_err(|_| CliError::new("无法写入标准输出"))?;
            }
            Ok(())
        }
    }
}
