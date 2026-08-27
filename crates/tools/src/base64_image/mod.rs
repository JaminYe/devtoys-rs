mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_BASE64_IMAGE, TYPE_IMAGE};

pub use cli::cli_tool;
pub use helper::{decode_base64, encode_bytes, inspect_image, Base64ImageError, ImageInfo};

#[cfg(feature = "gui")]
pub use view::Base64ImageView;

pub const ID: &str = "Base64ImageEncoderDecoder";
pub const TYPE_BASE64_IMAGE_FILE: &str = "Base64ImageFile";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "Base64 图片",
        search_keywords: &["datauri", "image"],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_BASE64_IMAGE, TYPE_IMAGE, TYPE_BASE64_IMAGE_FILE],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, Base64ImageView::new)
}
