mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_JSON};
pub use helper::{convert_json_yaml, looks_like_json, looks_like_yaml, Conversion, JsonYamlError};
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
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(JsonYamlView::new())
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(detector::YamlDetector)]
}

#[derive(Default, Debug, Clone, Copy)]
pub struct JsonYamlTool;

impl Tool for JsonYamlTool {
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
    Box::new(JsonYamlTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_yaml_tool_implements_tool() {
        let tool = JsonYamlTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(!tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
