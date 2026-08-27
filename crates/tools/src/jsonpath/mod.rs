mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_JSON};

pub use helper::{eval_jsonpath, JsonPathError};
pub use cli::cli_tool;
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
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, JsonPathView::new)
}
