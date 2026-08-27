mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_JSON_ARRAY};

pub use cli::cli_tool;
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
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, JsonTableView::new)
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}
