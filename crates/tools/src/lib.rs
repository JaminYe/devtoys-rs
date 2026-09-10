mod cli;
mod extension;
mod indent;
#[cfg(feature = "gui")]
mod slot;
#[cfg(feature = "gui")]
pub mod ui;

mod base64_image;
mod base64_text;
pub mod catalog;
mod cron_parser;
mod date_converter;
mod escape_unescape;
mod hash_checksum;
mod image_converter;
mod json_formatter;
mod json_table;
mod json_yaml;
mod jsonpath;
mod jwt;
mod list_compare;
mod markdown_preview;
mod number_base;
mod password;
mod regex_tester;
mod sql_formatter;
mod text_analyzer;
mod text_compare;
mod url;
mod uuid_gen;
mod xml_formatter;
pub use catalog::{default_catalog, Tool, ToolCatalog};
pub use cli::{build_cli, build_cli_from, run_cli, run_cli_from, CliError, CliTool};
pub use extension::{
    catalog_with_extensions_dir, default_extensions_dir, load_extensions, ExtensionLoadError,
    ExtensionLoadResult,
};

use devtoys_api::{Detector, ToolMetadata};

pub use base64_image::Base64ImageTool;
pub use base64_text::Base64TextTool;

pub use cron_parser::CronParserTool;
pub use date_converter::DateConverterTool;
pub use escape_unescape::EscapeUnescapeTool;
pub use hash_checksum::HashChecksumTool;
pub use image_converter::ImageConverterTool;
pub use indent::Indentation;
pub use json_formatter::{format_json, JsonFormatError, JsonFormatterTool};
pub use json_table::JsonTableTool;
pub use json_yaml::JsonYamlTool;
pub use jsonpath::JsonpathTool;
pub use jwt::JwtTool;
pub use list_compare::ListCompareTool;
pub use markdown_preview::MarkdownPreviewTool;
pub use number_base::NumberBaseTool;
pub use password::PasswordTool;
pub use regex_tester::RegexTesterTool;
pub use sql_formatter::SqlFormatterTool;
pub use text_analyzer::TextAnalyzerTool;
pub use text_compare::TextCompareTool;
pub use url::UrlTool;
pub use uuid_gen::UuidGenTool;
pub use xml_formatter::XmlFormatterTool;

#[cfg(feature = "gui")]
pub use slot::{ToolHandle, ToolView};

pub fn all_tools() -> Vec<ToolMetadata> {
    default_catalog().all_metadata()
}

pub fn all_cli_tools() -> Vec<CliTool> {
    default_catalog().all_cli()
}

pub fn tool_detectors() -> Vec<Box<dyn Detector>> {
    default_catalog().all_detectors()
}

#[cfg(feature = "gui")]
pub fn open_gui_tool(id: &str) -> Option<ToolHandle> {
    default_catalog().open_view(id)
}
