mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
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
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(Base64TextView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Base64TextTool;

impl Tool for Base64TextTool {
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
    Box::new(Base64TextTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_text_tool_implements_tool() {
        let tool = Base64TextTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
