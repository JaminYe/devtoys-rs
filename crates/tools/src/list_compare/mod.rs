mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{GroupId, ToolId, ToolMetadata};
pub use helper::{compare_lists, ListMode};
#[cfg(feature = "gui")]
pub use view::ListCompareView;

pub const ID: &str = "ListCompare";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "列表比对",
        search_keywords: &["intersect", "union", "diff"],
        group: GroupId::Text,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(ListCompareView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ListCompareTool;

impl Tool for ListCompareTool {
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
    Box::new(ListCompareTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_compare_tool_implements_tool() {
        let tool = ListCompareTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
