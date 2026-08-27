use devtoys_api::{
    DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_BASE64_IMAGE, TYPE_BASE64_TEXT,
    TYPE_DATE, TYPE_FILE, TYPE_FILES, TYPE_GZIP, TYPE_IMAGE, TYPE_IMAGE_FILE, TYPE_JSON,
    TYPE_JSON_ARRAY, TYPE_TEXT, TYPE_XML, TYPE_XSD,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct TextDetector;

impl Detector for TextDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_TEXT,
            parent: None,
        }
    }

    fn detect(&self, raw: &RawData, _parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let value = raw.as_text()?;
        if value.is_empty() {
            return None;
        }
        Some(payload(TYPE_TEXT, value))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct JsonDetector;

impl Detector for JsonDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_JSON,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let value = parent_value(parent)?;
        // C# JsonDataTypeDetector rejects long.TryParse integers so clipboard numbers are not JSON.
        if looks_like_i64(value) {
            return None;
        }
        serde_json::from_str::<serde_json::Value>(value).ok()?;
        Some(payload(TYPE_JSON, value))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct JsonArrayDetector;

impl Detector for JsonArrayDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_JSON_ARRAY,
            parent: Some(TYPE_JSON),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let value = parent_value(parent)?;
        let parsed: serde_json::Value = serde_json::from_str(value).ok()?;
        let arr = parsed.as_array()?;
        if arr.is_empty() || !arr.iter().all(|item| item.is_object()) {
            return None;
        }
        Some(payload(TYPE_JSON_ARRAY, value))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct XmlDetector;

impl Detector for XmlDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_XML,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let value = parent_value(parent)?;
        if !looks_like_xml(value) {
            return None;
        }
        Some(payload(TYPE_XML, value))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct XsdDetector;

impl Detector for XsdDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_XSD,
            parent: Some(TYPE_XML),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let value = parent_value(parent)?;
        if !looks_like_xsd(value) {
            return None;
        }
        Some(payload(TYPE_XSD, value))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Base64TextDetector;

impl Detector for Base64TextDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_BASE64_TEXT,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let value = parent_value(parent)?;
        if looks_like_i64(value) {
            return None;
        }
        let trimmed = value.trim();
        if trimmed.len() < 16 || decode_base64_std(trimmed).is_none() {
            return None;
        }
        Some(payload(TYPE_BASE64_TEXT, value))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Base64ImageDetector;

impl Detector for Base64ImageDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_BASE64_IMAGE,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let value = parent_value(parent)?;
        if looks_like_i64(value) {
            return None;
        }
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        if is_image_data_uri(trimmed) {
            return Some(payload(TYPE_BASE64_IMAGE, value));
        }
        let bytes = decode_base64_std(trimmed)?;
        if !is_image_magic(&bytes) {
            return None;
        }
        Some(payload(TYPE_BASE64_IMAGE, value))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GzipDetector;

impl Detector for GzipDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_GZIP,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let value = parent_value(parent)?;
        if looks_like_i64(value) {
            return None;
        }
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        // 1F 8B 08 (gzip + deflate) encodes to the C# prefix "H4sI".
        if trimmed.starts_with("H4sI") {
            return Some(payload(TYPE_GZIP, value));
        }
        let bytes = decode_base64_std(trimmed)?;
        if bytes.len() < 2 || bytes[0] != 0x1F || bytes[1] != 0x8B {
            return None;
        }
        Some(payload(TYPE_GZIP, value))
    }
}

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
        let value = parent_value(parent)?;
        if !looks_like_date(value) {
            return None;
        }
        Some(payload(TYPE_DATE, value))
    }
}

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
        let mime = match raw {
            RawData::Image { mime, .. } => mime.clone().unwrap_or_default(),
            _ => String::new(),
        };
        Some(payload(TYPE_IMAGE, mime))
    }
}

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
        Some(payload(TYPE_FILES, files.join("\n")))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileDetector;

impl Detector for FileDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_FILE,
            parent: Some(TYPE_FILES),
        }
    }

    fn detect(&self, raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        parent?;
        let files = raw.as_files()?;
        if files.len() != 1 {
            return None;
        }
        Some(payload(TYPE_FILE, files[0].as_str()))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ImageFileDetector;

impl Detector for ImageFileDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_IMAGE_FILE,
            parent: Some(TYPE_FILE),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let path = parent_value(parent)?;
        if !is_image_path(path) {
            return None;
        }
        Some(payload(TYPE_IMAGE_FILE, path))
    }
}

fn payload(type_name: &'static str, value: impl Into<String>) -> DetectedPayload {
    DetectedPayload {
        type_name: type_name.to_string(),
        value: value.into(),
    }
}

fn parent_value(parent: Option<&DetectedPayload>) -> Option<&str> {
    parent.map(|p| p.value.as_str())
}

fn looks_like_i64(value: &str) -> bool {
    value.trim().parse::<i64>().is_ok()
}

fn looks_like_xml(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.len() < 3
        || !trimmed.starts_with('<')
        || !trimmed.contains('>')
        || !trimmed.ends_with('>')
    {
        return false;
    }
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return false;
    }
    let mut chars = trimmed[1..].chars().peekable();
    if matches!(chars.peek(), Some('/' | '?' | '!')) {
        chars.next();
    }
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
}

fn looks_like_xsd(input: &str) -> bool {
    input.contains("xs:schema")
        || input.contains("xsd:schema")
        || input.contains("http://www.w3.org/2001/XMLSchema")
}

fn looks_like_date(input: &str) -> bool {
    let trimmed = input.trim();
    is_unix_seconds(trimmed) || is_unix_millis(trimmed) || is_iso_8601(trimmed)
}

fn is_unix_seconds(s: &str) -> bool {
    s.len() == 10 && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_unix_millis(s: &str) -> bool {
    s.len() == 13 && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_iso_8601(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 10 {
        return false;
    }
    if !is_digits(&b[0..4])
        || b[4] != b'-'
        || !is_digits(&b[5..7])
        || b[7] != b'-'
        || !is_digits(&b[8..10])
    {
        return false;
    }
    let month = parse_u16(&b[5..7]);
    let day = parse_u16(&b[8..10]);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return false;
    }
    if b.len() == 10 {
        return true;
    }
    if b[10] != b'T' && b[10] != b' ' {
        return false;
    }
    if b.len() < 16 || !is_digits(&b[11..13]) || b[13] != b':' || !is_digits(&b[14..16]) {
        return false;
    }
    let hour = parse_u16(&b[11..13]);
    let minute = parse_u16(&b[14..16]);
    if hour > 23 || minute > 59 {
        return false;
    }
    let mut i = 16;
    if i < b.len() && b[i] == b':' {
        if i + 3 > b.len() || !is_digits(&b[i + 1..i + 3]) {
            return false;
        }
        let second = parse_u16(&b[i + 1..i + 3]);
        if second > 60 {
            return false;
        }
        i += 3;
        if i < b.len() && b[i] == b'.' {
            i += 1;
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i == start {
                return false;
            }
        }
    }
    if i == b.len() {
        return true;
    }
    match b[i] {
        b'Z' | b'z' => i + 1 == b.len(),
        b'+' | b'-' => is_iso_offset(&b[i + 1..]),
        _ => false,
    }
}

fn is_iso_offset(b: &[u8]) -> bool {
    match b.len() {
        2 if is_digits(b) => parse_u16(b) <= 23,
        4 if is_digits(b) => parse_u16(&b[0..2]) <= 23 && parse_u16(&b[2..4]) <= 59,
        5 if is_digits(&b[0..2]) && b[2] == b':' && is_digits(&b[3..5]) => {
            parse_u16(&b[0..2]) <= 23 && parse_u16(&b[3..5]) <= 59
        }
        _ => false,
    }
}

fn is_digits(b: &[u8]) -> bool {
    !b.is_empty() && b.iter().all(|c| c.is_ascii_digit())
}

fn parse_u16(b: &[u8]) -> u16 {
    b.iter().fold(0, |acc, &c| acc * 10 + u16::from(c - b'0'))
}

fn is_image_data_uri(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.starts_with("data:image/") && lower.contains(";base64,")
}

fn is_image_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
        || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP")
}

fn is_image_path(path: &str) -> bool {
    let Some((_, ext)) = path.rsplit_once('.') else {
        return false;
    };
    if ext.contains('/') || ext.contains('\\') {
        return false;
    }
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "bmp" | "jpeg" | "jpg" | "pbm" | "png" | "tiff" | "tga" | "webp"
    )
}

/// Strict RFC 4648 standard-alphabet decode. Rejects whitespace and inner padding.
fn decode_base64_std(input: &str) -> Option<Vec<u8>> {
    if input.is_empty() || input.len() % 4 != 0 {
        return None;
    }
    let bytes = input.as_bytes();
    let pad = match (bytes[bytes.len() - 2], bytes[bytes.len() - 1]) {
        (b'=', b'=') => 2,
        (_, b'=') => 1,
        (_, _) => 0,
    };
    if bytes[..bytes.len() - pad].contains(&b'=') {
        return None;
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    for chunk in bytes.chunks_exact(4) {
        let a = base64_val(chunk[0])?;
        let b = base64_val(chunk[1])?;
        let c = if chunk[2] == b'=' {
            0
        } else {
            base64_val(chunk[2])?
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            base64_val(chunk[3])?
        };
        if chunk[2] == b'=' && chunk[3] != b'=' {
            return None;
        }
        out.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            out.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            out.push((c << 6) | d);
        }
    }
    Some(out)
}

fn base64_val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
