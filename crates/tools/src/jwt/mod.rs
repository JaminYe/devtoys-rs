mod helper;
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};
pub use helper::{
    claims_table, decode_jwt, encode_jwt, JwtAlgorithm, JwtDecodeOptions, JwtDecoded,
    JwtEncodeOptions, JwtError,
};
#[cfg(feature = "gui")]
pub use view::JwtView;

pub const ID: &str = "JsonWebTokenEncoderDecoder";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "JWT",
        search_keywords: &["jwt", "token"],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(JwtView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct JwtTool;

impl Tool for JwtTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
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
    Box::new(JwtTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_recommend_from_json() {
        assert!(
            metadata().accepted_types.is_empty(),
            "JWT must not be recommended from JSON clipboard"
        );
        assert_eq!(metadata().id.as_str(), ID);
        assert_eq!(metadata().display_name, "JWT");
        assert_eq!(metadata().group, GroupId::EncodersDecoders);
        assert!(!metadata().search_keywords.contains(&"json"));
        assert!(detectors().is_empty());
    }

    #[test]
    fn jwt_tool_implements_tool() {
        let tool = JwtTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
