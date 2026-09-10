mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};
pub use helper::{
    add_thousands_separators, convert_base, convert_custom, convert_rfc4648, decode_custom,
    encode_custom, looks_like_number_base, BasicBaseFields, NumberBase, NumberBaseError,
    Rfc4648Encoding, Signedness,
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
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(NumberBaseView::new())
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(detector::NumberBaseDetector)]
}

#[derive(Default, Debug, Clone, Copy)]
pub struct NumberBaseTool;

impl Tool for NumberBaseTool {
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
    Box::new(NumberBaseTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_base_tool_implements_tool() {
        let tool = NumberBaseTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(!tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
