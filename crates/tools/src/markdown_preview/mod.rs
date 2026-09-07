mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};
pub use helper::{markdown_to_html, MarkdownDetector, TYPE_MARKDOWN};
#[cfg(feature = "gui")]
pub use view::MarkdownPreviewView;

pub const ID: &str = "MarkdownPreview";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "Markdown 预览",
        search_keywords: &["markdown", "md"],
        group: GroupId::Text,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_MARKDOWN],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(MarkdownPreviewView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct MarkdownPreviewTool;

impl Tool for MarkdownPreviewTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn detectors(&self) -> Vec<Box<dyn Detector>> {
        vec![Box::new(MarkdownDetector)]
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(MarkdownPreviewTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_preview_tool_implements_tool() {
        let tool = MarkdownPreviewTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_none());
        assert_eq!(tool.detectors().len(), 1);
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
