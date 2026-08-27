mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata};

pub use helper::{generate_lorem, Corpus, LoremError, LoremUnit};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::LoremIpsumView;

pub const ID: &str = "LoremIpsumGenerator";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "乱数假文",
        search_keywords: &["lorem", "placeholder"],
        group: GroupId::Generators,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, LoremIpsumView::new)
}
