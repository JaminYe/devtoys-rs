mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_TEXT};

pub use helper::{test_regex, RegexOptions, RegexTesterError};
#[cfg(feature = "gui")]
pub use view::RegexTesterView;

pub const ID: &str = "RegExTester";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "正则表达式",
        search_keywords: &["regex", "正则"],
        group: GroupId::Testers,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_TEXT],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, RegexTesterView::new)
}
