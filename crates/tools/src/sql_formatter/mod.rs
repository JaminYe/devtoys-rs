mod cli;
mod dialect;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
use devtoys_api::{GroupId, ToolId, ToolMetadata};
pub use helper::{format_sql, Indentation, SqlLanguage};
#[cfg(feature = "gui")]
pub use view::SqlFormatterView;

pub const ID: &str = "SqlFormatter";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "SQL",
        search_keywords: &["sql", "美化"],
        group: GroupId::Formatters,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(SqlFormatterView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct SqlFormatterTool;

impl Tool for SqlFormatterTool {
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
    Box::new(SqlFormatterTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_formatter_tool_implements_tool() {
        let tool = SqlFormatterTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
