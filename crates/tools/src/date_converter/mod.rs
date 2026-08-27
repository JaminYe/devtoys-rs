mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_DATE};

pub use cli::cli_tool;
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
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, DateConverterView::new)
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(detector::DateDetector)]
}
