use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_DATE, TYPE_TEXT};

use super::looks_like_date;

#[derive(Debug, Default, Clone, Copy)]
pub struct DateDetector;

impl Detector for DateDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_DATE,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        let value = parent.value.as_str();
        if !looks_like_date(value) {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_DATE.to_string(),
            value: value.to_string(),
        })
    }
}
