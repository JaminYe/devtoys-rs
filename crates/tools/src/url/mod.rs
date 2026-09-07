mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};

pub use cli::cli_tool;
pub use helper::{convert, decode, encode, Conversion, UrlError};
#[cfg(feature = "gui")]
pub use view::UrlView;

pub const ID: &str = "UrlEncoderDecoder";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "URL",
        search_keywords: &["url", "percent", "uri"],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(UrlView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct UrlTool;

impl Tool for UrlTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn cli(&self) -> Option<CliTool> {
        Some(cli_tool())
    }

    fn detectors(&self) -> Vec<Box<dyn Detector>> {
        detectors()
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(UrlTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_detection_not_wired() {
        assert!(metadata().accepted_types.is_empty());
        assert_eq!(metadata().id.as_str(), ID);
        assert_eq!(metadata().display_name, "URL");
        assert_eq!(metadata().group, GroupId::EncodersDecoders);
        assert!(detectors().is_empty());
    }

    #[test]
    fn url_tool_implements_tool() {
        let tool = UrlTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
