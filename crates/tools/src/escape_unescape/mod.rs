mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};
pub use helper::{convert, Conversion, EscapedTextDetector, TYPE_ESCAPED};
#[cfg(feature = "gui")]
pub use view::EscapeUnescapeView;

pub const ID: &str = "EscapeUnescape";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "转义 / 反转义",
        search_keywords: &["escape", "unescape"],
        group: GroupId::Text,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_ESCAPED],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(EscapeUnescapeView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct EscapeUnescapeTool;

impl Tool for EscapeUnescapeTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn detectors(&self) -> Vec<Box<dyn Detector>> {
        vec![Box::new(EscapedTextDetector)]
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(EscapeUnescapeTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_unescape_tool_implements_tool() {
        let tool = EscapeUnescapeTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert_eq!(tool.detectors().len(), 1);
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
