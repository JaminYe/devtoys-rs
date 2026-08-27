use std::path::Path;

use devtoys_api::{
    DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_FILE, TYPE_FILES, TYPE_IMAGE,
};

pub const TYPE_STATIC_IMAGE_FILE: &str = "StaticImageFile";

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

/// Host generic `files`. Issue 27 lands the parent so `file` / StaticImageFile can attach.
#[derive(Debug, Default, Clone, Copy)]
pub struct FilesDetector;

impl Detector for FilesDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_FILES,
            parent: None,
        }
    }

    fn detect(&self, raw: &RawData, _parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let files = raw.as_files()?;
        if files.is_empty() {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_FILES.to_string(),
            value: files.join("\n"),
        })
    }
}

/// Host generic `file` (single path), parent `files`.
#[derive(Debug, Default, Clone, Copy)]
pub struct FileDetector;

impl Detector for FileDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_FILE,
            parent: Some(TYPE_FILES),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        let paths = split_paths(&parent.value);
        if paths.len() != 1 {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_FILE.to_string(),
            value: paths[0].to_string(),
        })
    }
}

/// Host generic `image` (clipboard / in-memory bytes).
#[derive(Debug, Default, Clone, Copy)]
pub struct ImageDetector;

impl Detector for ImageDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_IMAGE,
            parent: None,
        }
    }

    fn detect(&self, raw: &RawData, _parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let bytes = raw.as_image()?;
        if bytes.is_empty() {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_IMAGE.to_string(),
            value: String::new(),
        })
    }
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
        Some(DetectedPayload {
            type_name: TYPE_STATIC_IMAGE_FILE.to_string(),
            value: parent.value.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files_parent(paths: &str) -> DetectedPayload {
        DetectedPayload {
            type_name: TYPE_FILES.to_string(),
            value: paths.to_string(),
        }
    }

    fn file_parent(path: &str) -> DetectedPayload {
        DetectedPayload {
            type_name: TYPE_FILE.to_string(),
            value: path.to_string(),
        }
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
    fn file_detector_requires_single_path() {
        let detector = FileDetector;
        assert!(detector
            .detect(&RawData::Files(vec![]), Some(&files_parent("a.png\nb.png")))
            .is_none());
        let one = detector
            .detect(&RawData::Files(vec![]), Some(&files_parent("a.png")))
            .unwrap();
        assert_eq!(one.value, "a.png");
    }
}
