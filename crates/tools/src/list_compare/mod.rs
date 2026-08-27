mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata};

pub use helper::{compare_lists, ListMode};
pub use cli::cli_tool;
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
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, ListCompareView::new)
}
