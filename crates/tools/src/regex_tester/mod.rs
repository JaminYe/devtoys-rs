mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_TEXT};
pub use helper::{substitute, test_regex, RegexOptions, RegexTesterError};
#[cfg(feature = "gui")]
pub use view::RegexTesterView;

pub const ID: &str = "RegExTester";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "正则表达式",
        search_keywords: &["regex", "正则"],
        group: GroupId::Testers,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_TEXT],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(RegexTesterView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct RegexTesterTool;

impl Tool for RegexTesterTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(RegexTesterTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_tester_tool_implements_tool() {
        let tool = RegexTesterTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_none());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
