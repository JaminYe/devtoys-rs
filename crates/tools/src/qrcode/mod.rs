mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_IMAGE, TYPE_TEXT};
pub use helper::{
    decode_image_bytes, decode_image_path, encode_png, encode_svg, is_existing_image_file, QrError,
};
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
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(QrcodeView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct QrcodeTool;

impl Tool for QrcodeTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn cli(&self) -> Option<CliTool> {
        Some(cli_tool())
    }

    fn detectors(&self) -> Vec<Box<dyn Detector>> {
        detectors()
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(QrcodeTool)
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

    #[test]
    fn qrcode_tool_implements_tool() {
        let tool = QrcodeTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
