mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata};

pub use helper::{markdown_to_html, MarkdownDetector, TYPE_MARKDOWN};
#[cfg(feature = "gui")]
pub use view::MarkdownPreviewView;

pub const ID: &str = "MarkdownPreview";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "Markdown 预览",
        search_keywords: &["markdown", "md"],
        group: GroupId::Text,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_MARKDOWN],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, MarkdownPreviewView::new)
}
