mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_JSON};
pub use helper::{eval_jsonpath, JsonPathError};
#[cfg(feature = "gui")]
pub use view::JsonPathView;

pub const ID: &str = "JSONPathTester";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "JSONPath",
        search_keywords: &["jsonpath", "jpt"],
        group: GroupId::Testers,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_JSON],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(JsonPathView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct JsonpathTool;

impl Tool for JsonpathTool {
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
    Box::new(JsonpathTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonpath_tool_implements_tool() {
        let tool = JsonpathTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
