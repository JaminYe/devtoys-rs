mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata, JSON_FORMATTER_ID, TYPE_JSON};

pub use helper::{format_json, Indentation, JsonFormatError};
pub use cli::cli_tool;
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
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, JsonFormatterView::new)
}
