mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{GroupId, ToolId, ToolMetadata, JSON_FORMATTER_ID, TYPE_JSON};

pub use helper::{format_json, Indentation, JsonFormatError};
#[cfg(feature = "gui")]
pub use view::JsonFormatterView;

pub const ID: &str = JSON_FORMATTER_ID;

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "JSON",
        search_keywords: &["json", "格式化", "美化", "压缩"],
        group: GroupId::Formatters,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_JSON],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(JsonFormatterView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct JsonFormatterTool;

impl Tool for JsonFormatterTool {
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
    Box::new(JsonFormatterTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_formatter_tool_implements_tool() {
        let tool = JsonFormatterTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
