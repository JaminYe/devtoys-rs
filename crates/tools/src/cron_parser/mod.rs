mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};
pub use helper::{
    parse_cron, CronParseError, CronParseResult, DEFAULT_DATE_FORMAT, DEFAULT_EXPR_WITHOUT_SECONDS,
    DEFAULT_EXPR_WITH_SECONDS,
};
#[cfg(feature = "gui")]
pub use view::CronParserView;

pub const ID: &str = "CronParser";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "Cron 解析器",
        search_keywords: &["cron", "crontab", "调度"],
        group: GroupId::Converters,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(CronParserView::new())
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[derive(Default, Debug, Clone, Copy)]
pub struct CronParserTool;

impl Tool for CronParserTool {
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
    Box::new(CronParserTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_parser_tool_implements_tool() {
        let tool = CronParserTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
