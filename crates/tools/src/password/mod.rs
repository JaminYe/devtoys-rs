mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata};

pub use helper::{generate_password, PasswordError, PasswordOptions};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::PasswordView;

pub const ID: &str = "PasswordGenerator";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "密码",
        search_keywords: &["password", "密码"],
        group: GroupId::Generators,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, PasswordView::new)
}
