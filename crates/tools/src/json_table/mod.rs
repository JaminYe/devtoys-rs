mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_JSON_ARRAY};
pub use helper::{json_to_table, JsonTableError, TableFormat};
#[cfg(feature = "gui")]
pub use view::JsonTableView;

pub const ID: &str = "JsonTableConverter";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "JSON > 表格",
        search_keywords: &["csv", "tsv", "表格"],
        group: GroupId::Converters,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_JSON_ARRAY],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(JsonTableView::new())
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[derive(Default, Debug, Clone, Copy)]
pub struct JsonTableTool;

impl Tool for JsonTableTool {
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
    Box::new(JsonTableTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_table_tool_implements_tool() {
        let tool = JsonTableTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
