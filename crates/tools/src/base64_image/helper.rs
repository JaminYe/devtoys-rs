use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

const IMAGE_FILE_EXTENSIONS: &[&str] = &["bmp", "gif", "ico", "jpeg", "jpg", "png", "svg", "webp"];
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

pub fn is_image_file_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            IMAGE_FILE_EXTENSIONS
                .iter()
                .any(|want| ext.eq_ignore_ascii_case(want))
        })
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Base64ImageError {
    #[error("非法图片")]
    InvalidImage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    pub mime: &'static str,
    pub label: &'static str,
    pub byte_len: usize,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl ImageInfo {
    pub fn summary(&self) -> String {
        match (self.width, self.height) {
            (Some(w), Some(h)) => format!("{} {}×{} · {} 字节", self.label, w, h, self.byte_len),
            _ => format!("{} · {} 字节", self.label, self.byte_len),
        }
    }
}

pub fn encode_bytes(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn decode_base64(input: &str) -> Result<Vec<u8>, Base64ImageError> {
    let payload = strip_data_uri(input);
    let cleaned: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| Base64ImageError::InvalidImage)?;
    if sniff_mime(&bytes).is_none() && image::load_from_memory(&bytes).is_err() {
        return Err(Base64ImageError::InvalidImage);
    }
    Ok(bytes)
}

pub fn inspect_image(bytes: &[u8]) -> ImageInfo {
    let (mime, label) = sniff_mime(bytes).unwrap_or(("application/octet-stream", "图像"));
    let (width, height) = image::load_from_memory(bytes)
        .ok()
        .map(|img| (Some(img.width()), Some(img.height())))
        .unwrap_or((None, None));
    ImageInfo {
        mime,
        label,
        byte_len: bytes.len(),
        width,
        height,
    }
}

fn strip_data_uri(input: &str) -> &str {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("data:image/") {
        if let Some(idx) = lower.find(";base64,") {
            return trimmed[idx + ";base64,".len()..].trim();
        }
    }
    trimmed
}

fn sniff_mime(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.len() >= 4
        && bytes[0] == 0x89
        && bytes[1] == 0x50
        && bytes[2] == 0x4E
        && bytes[3] == 0x47
    {
        return Some(("image/png", "PNG"));
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some(("image/jpeg", "JPEG"));
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(("image/gif", "GIF"));
    }
    if bytes.starts_with(b"BM") {
        return Some(("image/bmp", "BMP"));
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some(("image/webp", "WEBP"));
    }
    if bytes.len() >= 4 && bytes[0] == 0 && bytes[1] == 0 && bytes[2] == 1 && bytes[3] == 0 {
        return Some(("image/x-icon", "ICO"));
    }
    if is_svg(bytes) {
        return Some(("image/svg+xml", "SVG"));
    }
    None
}

fn is_svg(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if svg_tag_at(lower.as_bytes(), 0) {
        return true;
    }
    if !lower.starts_with("<?xml") {
        return false;
    }
    let haystack = lower.as_bytes();
    let mut i = 0;
    while i + 4 <= haystack.len() {
        if svg_tag_at(haystack, i) {
            return true;
        }
        i += 1;
    }
    false
}

fn svg_tag_at(haystack: &[u8], i: usize) -> bool {
    if i + 4 > haystack.len() || &haystack[i..i + 4] != b"<svg" {
        return false;
    }
    match haystack.get(i + 4) {
        None => true,
        Some(b) => matches!(*b, b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_png_magic() {
        let got = encode_bytes(&[0x89, 0x50, 0x4E, 0x47]);
        assert_eq!(got, "iVBORw==");
    }

    #[test]
    fn decode_png_magic() {
        let got = decode_base64("iVBORw==").unwrap();
        assert_eq!(got, vec![0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn decode_strips_data_uri_prefix() {
        let got = decode_base64("data:image/png;base64,iVBORw==").unwrap();
        assert_eq!(got, vec![0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn invalid_is_err_without_input() {
        let err = decode_base64("not-an-image!!!").unwrap_err();
        assert_eq!(err, Base64ImageError::InvalidImage);
        let message = err.to_string();
        assert!(
            !message.contains("not-an-image"),
            "error must not include user input"
        );
    }

    #[test]
    fn image_file_extensions_match_known_types_including_svg() {
        for path in [
            "a.png", "a.PNG", "a.jpg", "a.jpeg", "a.gif", "a.bmp", "a.webp", "a.ico", "a.svg",
            "a.SVG",
        ] {
            assert!(is_image_file_path(Path::new(path)), "{path}");
        }
        for path in ["a.txt", "a.tga", "a.tiff", "a", "a.png.txt"] {
            assert!(!is_image_file_path(Path::new(path)), "{path}");
        }
    }

    const MINIMAL_SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";

    #[test]
    fn svg_encode_decode_roundtrip() {
        let encoded = encode_bytes(MINIMAL_SVG);
        let got = decode_base64(&encoded).unwrap();
        assert_eq!(got, MINIMAL_SVG);
    }

    #[test]
    fn svg_data_uri_decodes_to_original_bytes() {
        let encoded = encode_bytes(MINIMAL_SVG);
        let uri = format!("data:image/svg+xml;base64,{encoded}");
        let got = decode_base64(&uri).unwrap();
        assert_eq!(got, MINIMAL_SVG);
    }

    #[test]
    fn svg_file_path_is_image() {
        assert!(is_image_file_path(Path::new("a.svg")));
    }

    #[test]
    fn inspect_svg_labels_without_raster_dimensions() {
        let info = inspect_image(MINIMAL_SVG);
        assert_eq!(info.mime, "image/svg+xml");
        assert_eq!(info.label, "SVG");
        assert_eq!(info.width, None);
        assert_eq!(info.height, None);
        assert_eq!(info.summary(), format!("SVG · {} 字节", MINIMAL_SVG.len()));
    }

    #[test]
    fn xml_declaration_svg_decodes() {
        let svg = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let got = decode_base64(&encode_bytes(svg)).unwrap();
        assert_eq!(got, svg);
    }

    #[test]
    fn bom_prefixed_svg_decodes() {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice(MINIMAL_SVG);
        let got = decode_base64(&encode_bytes(&bytes)).unwrap();
        assert_eq!(got, bytes);
    }

    #[test]
    fn arbitrary_xml_is_invalid_image() {
        let xml = b"<?xml version=\"1.0\"?><root/>";
        let err = decode_base64(&encode_bytes(xml)).unwrap_err();
        assert_eq!(err, Base64ImageError::InvalidImage);
        assert_eq!(err.to_string(), "非法图片");
    }
}
