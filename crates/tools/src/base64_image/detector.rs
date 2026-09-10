use std::path::Path;

use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_FILE};

use super::TYPE_BASE64_IMAGE_FILE;

/// Matches upstream `Base64ImageFileDataTypeDetector.SupportedFileTypes`.
const EXTENSIONS: &[&str] = &["bmp", "gif", "ico", "jpg", "jpeg", "png", "svg", "webp"];

pub fn is_base64_image_file_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EXTENSIONS.iter().any(|want| ext.eq_ignore_ascii_case(want)))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Base64ImageFileDetector;

impl Detector for Base64ImageFileDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_BASE64_IMAGE_FILE,
            parent: Some(TYPE_FILE),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        if !is_base64_image_file_path(&parent.value) {
            return None;
        }
        Some(DetectedPayload::new(
            TYPE_BASE64_IMAGE_FILE,
            parent.value.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_parent(path: &str) -> DetectedPayload {
        DetectedPayload::new(TYPE_FILE, path)
    }

    #[test]
    fn accepts_image_extensions_including_spaces_and_chinese() {
        let detector = Base64ImageFileDetector;
        let path = r"C:\Users\图片\foo bar.PNG";
        let hit = detector
            .detect(&RawData::Files(vec![path.into()]), Some(&file_parent(path)))
            .unwrap();
        assert_eq!(hit.type_name, TYPE_BASE64_IMAGE_FILE);
        assert_eq!(hit.value, path);
        assert!(hit.image_bytes().is_none());
    }

    #[test]
    fn rejects_non_image_missing_parent_and_mixed_payloads() {
        let detector = Base64ImageFileDetector;
        assert!(detector
            .detect(
                &RawData::Files(vec![r"C:\a.txt".into()]),
                Some(&file_parent(r"C:\a.txt")),
            )
            .is_none());
        assert!(detector
            .detect(&RawData::Files(vec![r"C:\a.png".into()]), None)
            .is_none());
        assert!(detector
            .detect(&RawData::text(r"C:\a.png"), Some(&file_parent(r"C:\a.png")))
            .is_some());
    }

    #[test]
    fn accepts_svg_extension() {
        let detector = Base64ImageFileDetector;
        let path = r"C:\icons\logo.svg";
        let hit = detector
            .detect(&RawData::Files(vec![path.into()]), Some(&file_parent(path)))
            .unwrap();
        assert_eq!(hit.type_name, TYPE_BASE64_IMAGE_FILE);
        assert_eq!(hit.value, path);
    }

    #[test]
    fn missing_file_still_matches_by_extension() {
        let detector = Base64ImageFileDetector;
        let path = r"C:\does not exist\gone.webp";
        assert!(detector
            .detect(&RawData::Files(vec![path.into()]), Some(&file_parent(path)))
            .is_some());
    }
}
