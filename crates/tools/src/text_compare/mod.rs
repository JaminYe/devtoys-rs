mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata};

pub use helper::{diff_lines, DiffMode, DiffTag};
#[cfg(feature = "gui")]
pub use view::TextCompareView;

pub const ID: &str = "TextCompare";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "比较",
        search_keywords: &["diff", "比较"],
        group: GroupId::Text,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, TextCompareView::new)
}
