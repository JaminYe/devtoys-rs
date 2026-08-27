mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_FILE, TYPE_TEXT};

pub use helper::{checksum_matches, compute_hash, HashAlgorithm, HashError};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::HashChecksumView;

pub const ID: &str = "HashAndChecksumGenerator";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "哈希 / 校验和",
        search_keywords: &["md5", "sha", "hmac", "checksum"],
        group: GroupId::Generators,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_TEXT, TYPE_FILE],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, HashChecksumView::new)
}
