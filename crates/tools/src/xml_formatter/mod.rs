mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_XML};
pub use helper::{format_xml, Indentation, XmlDetector, XmlFormatError};
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
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(XmlFormatterView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct XmlFormatterTool;

impl Tool for XmlFormatterTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(XmlFormatterTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_formatter_tool_implements_tool() {
        let tool = XmlFormatterTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
