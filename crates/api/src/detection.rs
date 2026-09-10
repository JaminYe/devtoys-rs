/// Host generic type names. Domain types (Yaml, Certificate, …) live on tools.
pub const TYPE_TEXT: &str = "text";
pub const TYPE_JSON: &str = "json";
pub const TYPE_JSON_ARRAY: &str = "jsonArray";
pub const TYPE_XML: &str = "xml";
pub const TYPE_BASE64_TEXT: &str = "base64Text";
pub const TYPE_BASE64_IMAGE: &str = "base64Image";
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
    Image {
        bytes: Vec<u8>,
        mime: Option<String>,
    },
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

/// Result of a single detector. `value` stays a string so text tools keep
/// compiling; image/binary detections also fill `bytes` / `mime`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DetectedPayload {
    pub type_name: String,
    pub value: String,
    /// Encoded image (or other binary) bytes. Text detections leave this `None`.
    pub bytes: Option<Vec<u8>>,
    /// MIME type accompanying [`Self::bytes`], e.g. `image/png`.
    pub mime: Option<String>,
}

impl DetectedPayload {
    /// Text-only payload. Binary fields default to `None`.
    pub fn new(type_name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            value: value.into(),
            bytes: None,
            mime: None,
        }
    }

    /// Attach encoded bytes and optional MIME. `value` is unchanged.
    pub fn with_bytes(mut self, bytes: Vec<u8>, mime: Option<String>) -> Self {
        self.bytes = Some(bytes);
        self.mime = mime;
        self
    }

    /// Non-empty image/binary bytes, if present.
    pub fn image_bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref().filter(|b| !b.is_empty())
    }
}

/// Pure detector. The engine walks the parent-type tree; implementations must
/// not touch the window or clipboard.
pub trait Detector: Send + Sync {
    fn data_type(&self) -> DataTypeSpec;

    /// `parent` is the payload produced by the parent type, when this type has one.
    fn detect(&self, raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload>;
}
