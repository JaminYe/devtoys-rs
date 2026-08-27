mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata};

pub use helper::{format_sql, Indentation, SqlLanguage};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::SqlFormatterView;

pub const ID: &str = "SqlFormatter";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "SQL",
        search_keywords: &["sql", "美化"],
        group: GroupId::Formatters,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, SqlFormatterView::new)
}
