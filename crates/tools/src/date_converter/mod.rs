mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_DATE};
pub use helper::{
    datetime_to_timestamp, looks_like_date, timestamp_to_datetime, DateConvertError,
    TimestampFormat,
};
#[cfg(feature = "gui")]
pub use view::DateConverterView;

pub const ID: &str = "DateConverter";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "日期",
        search_keywords: &["unix", "timestamp", "时间戳", "epoch"],
        group: GroupId::Converters,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_DATE],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(DateConverterView::new())
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(detector::DateDetector)]
}

#[derive(Default, Debug, Clone, Copy)]
pub struct DateConverterTool;

impl Tool for DateConverterTool {
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
    Box::new(DateConverterTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_converter_tool_implements_tool() {
        let tool = DateConverterTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(!tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
