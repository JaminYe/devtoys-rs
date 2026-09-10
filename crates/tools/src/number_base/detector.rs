use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_TEXT};

use super::looks_like_number_base;

pub const TYPE_NUMBER_BASE: &str = "NumberBase";

#[derive(Debug, Default, Clone, Copy)]
pub struct NumberBaseDetector;

impl Detector for NumberBaseDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_NUMBER_BASE,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        let value = parent.value.as_str();
        if !looks_like_number_base(value) {
            return None;
        }
        Some(DetectedPayload::new(TYPE_NUMBER_BASE, value))
    }
}
