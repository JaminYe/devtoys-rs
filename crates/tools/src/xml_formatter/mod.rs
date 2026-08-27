mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_XML};

pub use helper::{format_xml, Indentation, XmlDetector, XmlFormatError};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::XmlFormatterView;

pub const ID: &str = "XmlFormatter";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "XML",
        search_keywords: &["xml", "美化"],
        group: GroupId::Formatters,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_XML],
    }
}

pub fn detector() -> XmlDetector {
    XmlDetector
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, XmlFormatterView::new)
}
