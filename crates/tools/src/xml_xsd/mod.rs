mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_XML, TYPE_XSD};

pub use cli::cli_tool;
pub use detector::XsdDetector;
pub use helper::{format_reports, validate_xml_xsd, XmlReport, XmlReportLevel};
#[cfg(feature = "gui")]
pub use view::XmlXsdView;

pub const ID: &str = "XMLTester";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "XML / XSD",
        search_keywords: &["xsd", "schema"],
        group: GroupId::Testers,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_XML, TYPE_XSD],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, XmlXsdView::new)
}
