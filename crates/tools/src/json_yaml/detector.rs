use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_TEXT};

use super::looks_like_yaml;

pub const TYPE_YAML: &str = "Yaml";

#[derive(Debug, Default, Clone, Copy)]
pub struct YamlDetector;

impl Detector for YamlDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_YAML,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        let value = parent.value.as_str();
        if !looks_like_yaml(value) {
            return None;
        }
        Some(DetectedPayload::new(TYPE_YAML, value))
    }
}
