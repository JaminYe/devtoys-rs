mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata};

pub use helper::{convert, Conversion, EscapedTextDetector, TYPE_ESCAPED};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::EscapeUnescapeView;

pub const ID: &str = "EscapeUnescape";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "转义 / 反转义",
        search_keywords: &["escape", "unescape"],
        group: GroupId::Text,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_ESCAPED],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, EscapeUnescapeView::new)
}
