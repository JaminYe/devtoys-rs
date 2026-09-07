use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_FILES};

use crate::color_blindness::{is_static_image_path, split_paths};

pub const TYPE_STATIC_IMAGE_FILES: &str = "StaticImageFiles";

#[derive(Debug, Default, Clone, Copy)]
pub struct StaticImageFilesDetector;

impl Detector for StaticImageFilesDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_STATIC_IMAGE_FILES,
            parent: Some(TYPE_FILES),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        let paths = split_paths(&parent.value);
        if paths.is_empty() {
            return None;
        }
        if !paths.iter().all(|path| is_static_image_path(path)) {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_STATIC_IMAGE_FILES.to_string(),
            value: parent.value.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files_parent(value: &str) -> DetectedPayload {
        DetectedPayload {
            type_name: TYPE_FILES.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn all_static_images_match() {
        let detector = StaticImageFilesDetector;
        let hit = detector
            .detect(
                &RawData::Files(vec![]),
                Some(&files_parent("a.png\nb.jpeg")),
            )
            .unwrap();
        assert_eq!(hit.type_name, TYPE_STATIC_IMAGE_FILES);
    }

    #[test]
    fn mixed_paths_do_not_match() {
        let detector = StaticImageFilesDetector;
        assert!(detector
            .detect(&RawData::Files(vec![]), Some(&files_parent("a.png\nb.txt")),)
            .is_none());
        assert!(detector.detect(&RawData::Files(vec![]), None).is_none());
    }
}
