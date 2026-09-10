mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{GroupId, ToolId, ToolMetadata};
pub use helper::{diff_lines, diff_rows, DiffLine, DiffMode, DiffRow, DiffSpan, DiffTag};
#[cfg(feature = "gui")]
pub use view::TextCompareView;

pub const ID: &str = "TextCompare";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "比较",
        search_keywords: &["diff", "比较"],
        group: GroupId::Text,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(TextCompareView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct TextCompareTool;

impl Tool for TextCompareTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn supports_compact_overlay(&self) -> bool {
        false
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(TextCompareTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_compare_tool_implements_tool() {
        let tool = TextCompareTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_none());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
