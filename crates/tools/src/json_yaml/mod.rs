mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_JSON};

pub use cli::cli_tool;
pub use helper::{
    convert_json_yaml, looks_like_json, looks_like_yaml, Conversion, JsonYamlError,
};
#[cfg(feature = "gui")]
pub use view::JsonYamlView;

pub const ID: &str = "JsonYamlConverter";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "JSON <> YAML",
        search_keywords: &["yaml", "yml"],
        group: GroupId::Converters,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_JSON, "Yaml"],
    }
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, JsonYamlView::new)
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(detector::YamlDetector)]
}
