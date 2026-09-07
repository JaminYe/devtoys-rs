mod cli;
mod indent;
#[cfg(feature = "gui")]
mod slot;
#[cfg(feature = "gui")]
pub mod ui;

mod base64_image;
mod base64_text;
pub mod catalog;
mod certificate;
mod color_blindness;
mod cron_parser;
mod date_converter;
mod escape_unescape;
mod gzip;
mod hash_checksum;
mod html;
mod image_converter;
mod json_formatter;
mod json_table;
mod json_yaml;
mod jsonpath;
mod jwt;
mod list_compare;
mod lorem_ipsum;
mod markdown_preview;
mod number_base;
mod password;
mod qrcode;
mod regex_tester;
mod sql_formatter;
mod text_analyzer;
mod text_compare;
mod url;
mod uuid_gen;
mod xml_formatter;
mod xml_xsd;
pub use catalog::{default_catalog, Tool, ToolCatalog};

use devtoys_api::{Detector, ToolMetadata};

pub use base64_image::Base64ImageTool;
pub use base64_text::Base64TextTool;
pub use certificate::CertificateTool;
pub use cli::{build_cli, run_cli, CliError, CliTool};
pub use color_blindness::ColorBlindnessTool;
pub use cron_parser::CronParserTool;
pub use date_converter::DateConverterTool;
pub use escape_unescape::EscapeUnescapeTool;
pub use gzip::GzipTool;
pub use hash_checksum::HashChecksumTool;
pub use html::HtmlTool;
pub use image_converter::ImageConverterTool;
pub use indent::Indentation;
pub use json_formatter::{format_json, JsonFormatError, JsonFormatterTool};
pub use json_table::JsonTableTool;
pub use json_yaml::JsonYamlTool;
pub use jsonpath::JsonpathTool;
pub use jwt::JwtTool;
pub use list_compare::ListCompareTool;
pub use lorem_ipsum::LoremIpsumTool;
pub use markdown_preview::MarkdownPreviewTool;
pub use number_base::NumberBaseTool;
pub use password::PasswordTool;
pub use qrcode::QrcodeTool;
pub use regex_tester::RegexTesterTool;
pub use sql_formatter::SqlFormatterTool;
pub use text_analyzer::TextAnalyzerTool;
pub use text_compare::TextCompareTool;
pub use url::UrlTool;
pub use uuid_gen::UuidGenTool;
pub use xml_formatter::XmlFormatterTool;
pub use xml_xsd::XmlXsdTool;

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
