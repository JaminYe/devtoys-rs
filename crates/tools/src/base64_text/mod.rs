mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_BASE64_TEXT, TYPE_TEXT};

pub use cli::cli_tool;
pub use helper::{
    convert, decode, encode, looks_like_base64, Base64TextError, Charset, Conversion,
};

#[cfg(feature = "gui")]
pub use view::Base64TextView;

pub const ID: &str = "Base64TextEncoderDecoder";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "Base64 文本",
        search_keywords: &["base64", "b64"],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_BASE64_TEXT, TYPE_TEXT],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, Base64TextView::new)
}
