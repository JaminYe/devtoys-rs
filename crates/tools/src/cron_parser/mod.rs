mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};

pub use cli::cli_tool;
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
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, CronParserView::new)
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}
