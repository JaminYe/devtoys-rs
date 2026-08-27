mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};

pub use cli::cli_tool;
pub use helper::{
    add_thousands_separators, convert_base, convert_rfc4648, decode_custom, decode_rfc4648,
    encode_custom, encode_rfc4648, looks_like_number_base, NumberBase, NumberBaseError,
    Rfc4648Encoding,
};
#[cfg(feature = "gui")]
pub use view::NumberBaseView;

pub const ID: &str = "NumberBaseConverter";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "数字进制",
        search_keywords: &["hex", "binary", "base64", "进制"],
        group: GroupId::Converters,
        searchable: true,
        favorable: true,
        accepted_types: &["NumberBase"],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, NumberBaseView::new)
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(detector::NumberBaseDetector)]
}
