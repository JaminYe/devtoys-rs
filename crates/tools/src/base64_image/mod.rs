mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_BASE64_IMAGE, TYPE_IMAGE};
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
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(Base64ImageView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Base64ImageTool;

impl Tool for Base64ImageTool {
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
    Box::new(Base64ImageTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_image_tool_implements_tool() {
        let tool = Base64ImageTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
