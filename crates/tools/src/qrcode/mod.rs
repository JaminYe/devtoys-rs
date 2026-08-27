mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_IMAGE, TYPE_TEXT};

pub use helper::{
    decode_image_bytes, decode_image_path, encode_png, encode_svg, is_existing_image_file, QrError,
};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::QrcodeView;

pub const ID: &str = "QRCodeEncoderDecoder";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "二维码",
        search_keywords: &["qr", "二维码"],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_TEXT, TYPE_IMAGE],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, QrcodeView::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_text_and_image() {
        assert_eq!(metadata().accepted_types, &["text", "image"]);
        assert_eq!(metadata().id.as_str(), ID);
        assert_eq!(metadata().display_name, "二维码");
        assert_eq!(metadata().group, GroupId::EncodersDecoders);
        assert!(detectors().is_empty());
    }
}
