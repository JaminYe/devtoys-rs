mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_TEXT};
pub use helper::{apply, stats, Operation, TextStats, OPERATION_NAMES};
#[cfg(feature = "gui")]
pub use view::TextAnalyzerView;

pub const ID: &str = "TextAnalyzerAndUtilities";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "文本分析和实用工具",
        search_keywords: &["word", "count", "case", "词频"],
        group: GroupId::Text,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_TEXT],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(TextAnalyzerView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct TextAnalyzerTool;

impl Tool for TextAnalyzerTool {
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
    Box::new(TextAnalyzerTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_analyzer_tool_implements_tool() {
        let tool = TextAnalyzerTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
