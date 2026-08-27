mod cli;
mod indent;
#[cfg(feature = "gui")]
mod slot;

mod base64_image;
mod base64_text;
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

use devtoys_api::{Detector, ToolMetadata};

pub use cli::{build_cli, run_cli, CliError, CliTool};
pub use indent::Indentation;
pub use json_formatter::{format_json, JsonFormatError};

#[cfg(feature = "gui")]
pub use slot::{ReceivesData, ToolHandle};

pub fn all_tools() -> Vec<ToolMetadata> {
    vec![
        cron_parser::metadata(),
        date_converter::metadata(),
        json_table::metadata(),
        json_yaml::metadata(),
        number_base::metadata(),
        base64_text::metadata(),
        base64_image::metadata(),
        certificate::metadata(),
        gzip::metadata(),
        html::metadata(),
        jwt::metadata(),
        qrcode::metadata(),
        url::metadata(),
        json_formatter::metadata(),
        sql_formatter::metadata(),
        xml_formatter::metadata(),
        hash_checksum::metadata(),
        lorem_ipsum::metadata(),
        password::metadata(),
        uuid_gen::metadata(),
        color_blindness::metadata(),
        image_converter::metadata(),
        jsonpath::metadata(),
        regex_tester::metadata(),
        xml_xsd::metadata(),
        text_analyzer::metadata(),
        text_compare::metadata(),
        escape_unescape::metadata(),
        list_compare::metadata(),
        markdown_preview::metadata(),
    ]
}

pub fn all_cli_tools() -> Vec<CliTool> {
    vec![
        cron_parser::cli_tool(),
        date_converter::cli_tool(),
        json_table::cli_tool(),
        json_yaml::cli_tool(),
        number_base::cli_tool(),
        base64_text::cli_tool(),
        base64_image::cli_tool(),
        certificate::cli_tool(),
        gzip::cli_tool(),
        html::cli_tool(),
        qrcode::cli_tool(),
        url::cli_tool(),
        json_formatter::cli_tool(),
        sql_formatter::cli_tool(),
        xml_formatter::cli_tool(),
        hash_checksum::cli_tool(),
        lorem_ipsum::cli_tool(),
        password::cli_tool(),
        uuid_gen::cli_tool(),
        color_blindness::cli_tool(),
        image_converter::cli_tool(),
        jsonpath::cli_tool(),
        xml_xsd::cli_tool(),
        text_analyzer::cli_tool(),
        escape_unescape::cli_tool(),
        list_compare::cli_tool(),
    ]
}

pub fn tool_detectors() -> Vec<Box<dyn Detector>> {
    let mut detectors = Vec::new();
    detectors.extend(json_yaml::detectors());
    detectors.extend(number_base::detectors());
    detectors.extend(certificate::detectors());
    detectors.push(Box::new(color_blindness::StaticImageFileDetector));
    detectors.extend(image_converter::detectors());
    detectors.push(Box::new(escape_unescape::EscapedTextDetector));
    detectors.push(Box::new(markdown_preview::MarkdownDetector));
    detectors
}

#[cfg(feature = "gui")]
pub fn open_gui_tool(
    id: &str,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> Option<ToolHandle> {
    Some(match id {
        cron_parser::ID => cron_parser::open_view(window, cx),
        date_converter::ID => date_converter::open_view(window, cx),
        json_table::ID => json_table::open_view(window, cx),
        json_yaml::ID => json_yaml::open_view(window, cx),
        number_base::ID => number_base::open_view(window, cx),
        base64_text::ID => base64_text::open_view(window, cx),
        base64_image::ID => base64_image::open_view(window, cx),
        certificate::ID => certificate::open_view(window, cx),
        gzip::ID => gzip::open_view(window, cx),
        html::ID => html::open_view(window, cx),
        jwt::ID => jwt::open_view(window, cx),
        qrcode::ID => qrcode::open_view(window, cx),
        url::ID => url::open_view(window, cx),
        json_formatter::ID => json_formatter::open_view(window, cx),
        sql_formatter::ID => sql_formatter::open_view(window, cx),
        xml_formatter::ID => xml_formatter::open_view(window, cx),
        hash_checksum::ID => hash_checksum::open_view(window, cx),
        lorem_ipsum::ID => lorem_ipsum::open_view(window, cx),
        password::ID => password::open_view(window, cx),
        uuid_gen::ID => uuid_gen::open_view(window, cx),
        color_blindness::ID => color_blindness::open_view(window, cx),
        image_converter::ID => image_converter::open_view(window, cx),
        jsonpath::ID => jsonpath::open_view(window, cx),
        regex_tester::ID => regex_tester::open_view(window, cx),
        xml_xsd::ID => xml_xsd::open_view(window, cx),
        text_analyzer::ID => text_analyzer::open_view(window, cx),
        text_compare::ID => text_compare::open_view(window, cx),
        escape_unescape::ID => escape_unescape::open_view(window, cx),
        list_compare::ID => list_compare::open_view(window, cx),
        markdown_preview::ID => markdown_preview::open_view(window, cx),
        _ => return None,
    })
}
