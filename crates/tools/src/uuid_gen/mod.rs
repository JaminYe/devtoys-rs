mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata};

pub use helper::{generate_uuid, UuidError, UuidOptions, UuidVersion};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::UuidGenView;

pub const ID: &str = "UUIDGenerator";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "UUID",
        search_keywords: &["guid", "uuid"],
        group: GroupId::Generators,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, UuidGenView::new)
}
