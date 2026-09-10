use std::path::Path;

use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_FILE, TYPE_FILES};

pub const TYPE_STATIC_IMAGE_FILE: &str = "StaticImageFile";
pub const TYPE_STATIC_IMAGE_FILES: &str = "StaticImageFiles";

pub const STATIC_IMAGE_EXTENSIONS: &[&str] = &[
    ".bmp", ".jpeg", ".jpg", ".pbm", ".png", ".tiff", ".tga", ".webp",
];

pub fn is_static_image_path(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_ascii_lowercase()));
    ext.is_some_and(|e| STATIC_IMAGE_EXTENSIONS.iter().any(|want| *want == e))
}

pub fn split_paths(value: &str) -> Vec<&str> {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StaticImageFileDetector;

impl Detector for StaticImageFileDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_STATIC_IMAGE_FILE,
            parent: Some(TYPE_FILE),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        if !is_static_image_path(&parent.value) {
            return None;
        }
        Some(DetectedPayload::new(
            TYPE_STATIC_IMAGE_FILE,
            parent.value.clone(),
        ))
    }
}

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
        Some(DetectedPayload::new(
            TYPE_STATIC_IMAGE_FILES,
            parent.value.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files_parent(value: &str) -> DetectedPayload {
        DetectedPayload::new(TYPE_FILES, value)
    }
    fn file_parent(path: &str) -> DetectedPayload {
        DetectedPayload::new(TYPE_FILE, path)
    }

    #[test]
    fn static_image_file_accepts_png_rejects_txt() {
        let detector = StaticImageFileDetector;
        let png = detector
            .detect(&RawData::Files(vec![]), Some(&file_parent(r"C:\a.PNG")))
            .unwrap();
        assert_eq!(png.type_name, TYPE_STATIC_IMAGE_FILE);
        assert!(detector
            .detect(&RawData::Files(vec![]), Some(&file_parent(r"C:\a.txt")))
            .is_none());
        assert!(detector.detect(&RawData::Files(vec![]), None).is_none());
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
