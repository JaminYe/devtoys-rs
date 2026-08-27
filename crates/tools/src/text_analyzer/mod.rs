mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_TEXT};

pub use helper::{apply, stats, Operation, TextStats, OPERATION_NAMES};
pub use cli::cli_tool;
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
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, TextAnalyzerView::new)
}
