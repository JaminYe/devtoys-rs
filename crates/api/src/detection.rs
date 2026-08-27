/// Host generic type names. Domain types (Yaml, Certificate, …) live on tools.
pub const TYPE_TEXT: &str = "text";
pub const TYPE_JSON: &str = "json";
pub const TYPE_JSON_ARRAY: &str = "jsonArray";
pub const TYPE_XML: &str = "xml";
pub const TYPE_XSD: &str = "xsd";
pub const TYPE_BASE64_TEXT: &str = "base64Text";
pub const TYPE_BASE64_IMAGE: &str = "base64Image";
pub const TYPE_GZIP: &str = "gzip";
pub const TYPE_DATE: &str = "date";
pub const TYPE_IMAGE: &str = "image";
pub const TYPE_FILES: &str = "files";
pub const TYPE_FILE: &str = "file";
pub const TYPE_IMAGE_FILE: &str = "imageFile";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataTypeSpec {
    pub name: &'static str,
    pub parent: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawData {
    Text(String),
    Image { bytes: Vec<u8>, mime: Option<String> },
    Files(Vec<String>),
}

impl RawData {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_image(&self) -> Option<&[u8]> {
        match self {
            Self::Image { bytes, .. } => Some(bytes),
            _ => None,
        }
    }

    pub fn as_files(&self) -> Option<&[String]> {
        match self {
            Self::Files(paths) => Some(paths),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedPayload {
    pub type_name: String,
    pub value: String,
}

/// Pure detector. The engine walks the parent-type tree; implementations must
/// not touch the window or clipboard.
pub trait Detector: Send + Sync {
    fn data_type(&self) -> DataTypeSpec;

    /// `parent` is the payload produced by the parent type, when this type has one.
    fn detect(&self, raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload>;
}
