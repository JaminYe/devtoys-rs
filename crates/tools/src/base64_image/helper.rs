use base64::engine::general_purpose::STANDARD;
use base64::Engine;

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
    None
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
}
