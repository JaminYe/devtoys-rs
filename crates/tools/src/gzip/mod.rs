mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_GZIP};

pub use cli::cli_tool;
pub use helper::{compress, convert, decompress, GzipError, GzipMode};

#[cfg(feature = "gui")]
pub use view::GzipView;

pub const ID: &str = "GZipEncoderDecoder";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "GZip",
        search_keywords: &["gzip", "compress"],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_GZIP],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, GzipView::new)
}
