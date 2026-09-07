mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;
use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
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
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(XmlXsdView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct XmlXsdTool;

impl Tool for XmlXsdTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn cli(&self) -> Option<CliTool> {
        Some(cli_tool())
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(XmlXsdTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_xsd_tool_implements_tool() {
        let tool = XmlXsdTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
