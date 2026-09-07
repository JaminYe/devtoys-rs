use std::sync::{Arc, Mutex};
use devtoys_api::RawData;

/// Trait abstracting clipboard content reading for Smart Detection.
pub trait ClipboardSource: Send + Sync {
    /// Reads raw data (text or image) from the clipboard source.
    ///
    /// Returns `None` if the clipboard is empty, cannot be accessed,
    /// or contains unsupported content.
    fn read_raw(&self) -> Option<RawData>;
}

/// Converts raw RGBA pixels into PNG-encoded bytes.
///
/// Returns `None` if:
/// - width or height is zero
/// - pixel buffer length does not match `width * height * 4`
/// - the buffer cannot be formatted or encoded into PNG
pub fn rgba_to_png(width: u32, height: u32, bytes: &[u8]) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let expected_len = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    if bytes.len() != expected_len {
        return None;
    }

    let pixels: Vec<u8> = bytes
        .chunks(4)
        .flat_map(|px| {
            let r = px.first().copied().unwrap_or(0);
            let g = px.get(1).copied().unwrap_or(0);
            let b = px.get(2).copied().unwrap_or(0);
            let a = px.get(3).copied().unwrap_or(255);
            [r, g, b, a]
        })
        .collect();

    let buffer = image::RgbaImage::from_raw(width, height, pixels)?;
    let mut encoded = Vec::new();
    image::DynamicImage::ImageRgba8(buffer)
        .write_to(
            &mut std::io::Cursor::new(&mut encoded),
            image::ImageFormat::Png,
        )
        .ok()?;
    Some(encoded)
}

/// System clipboard implementation backed by `arboard::Clipboard`.
pub struct SystemClipboard {
    clipboard: Mutex<Option<arboard::Clipboard>>,
}

impl Default for SystemClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemClipboard {
    /// Creates a new `SystemClipboard` instance.
    pub fn new() -> Self {
        Self {
            clipboard: Mutex::new(arboard::Clipboard::new().ok()),
        }
    }
}

impl ClipboardSource for SystemClipboard {
    fn read_raw(&self) -> Option<RawData> {
        let mut guard = match self.clipboard.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        if guard.is_none() {
            *guard = arboard::Clipboard::new().ok();
        }

        let clipboard = guard.as_mut()?;

        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                return Some(RawData::text(text));
            }
        }

        if let Ok(image) = clipboard.get_image() {
            let width = image.width as u32;
            let height = image.height as u32;
            if let Some(bytes) = rgba_to_png(width, height, &image.bytes) {
                return Some(RawData::Image {
                    bytes,
                    mime: Some("image/png".to_string()),
                });
            }
        }

        None
    }
}

/// In-memory clipboard source for testing and headless environments.
#[derive(Clone, Default)]
pub struct InMemoryClipboard {
    data: Arc<Mutex<Option<RawData>>>,
}

impl InMemoryClipboard {
    /// Creates an empty `InMemoryClipboard`.
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets or clears the current clipboard content.
    pub fn set(&self, raw: Option<RawData>) {
        let mut guard = match self.data.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = raw;
    }

    /// Sets the clipboard content to text.
    pub fn set_text(&self, text: impl Into<String>) {
        self.set(Some(RawData::text(text)));
    }

    /// Sets the clipboard content to raw image bytes with an optional MIME type.
    pub fn set_image(&self, bytes: Vec<u8>, mime: Option<String>) {
        self.set(Some(RawData::Image { bytes, mime }));
    }

    /// Clears the clipboard content.
    pub fn clear(&self) {
        self.set(None);
    }
}

impl ClipboardSource for InMemoryClipboard {
    fn read_raw(&self) -> Option<RawData> {
        let guard = match self.data.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_to_png_valid() {
        // 2x2 image = 4 pixels * 4 = 16 bytes
        let pixels = vec![
            255, 0, 0, 255,   // red
            0, 255, 0, 255,   // green
            0, 0, 255, 255,   // blue
            255, 255, 255, 255, // white
        ];
        let png = rgba_to_png(2, 2, &pixels);
        assert!(png.is_some());
        let png_bytes = png.unwrap();
        // PNG magic number: \x89PNG\r\n\x1a\n
        assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn test_rgba_to_png_zero_dimensions() {
        assert!(rgba_to_png(0, 10, &[0; 40]).is_none());
        assert!(rgba_to_png(10, 0, &[0; 40]).is_none());
        assert!(rgba_to_png(0, 0, &[]).is_none());
    }

    #[test]
    fn test_rgba_to_png_dirty_truncated_data() {
        // Expected 2x2 * 4 = 16 bytes, but provided only 10 bytes
        let truncated = vec![255; 10];
        assert!(rgba_to_png(2, 2, &truncated).is_none());
    }

    #[test]
    fn test_rgba_to_png_dirty_excess_data() {
        // Expected 1x1 * 4 = 4 bytes, but provided 16 bytes
        let excess = vec![255; 16];
        assert!(rgba_to_png(1, 1, &excess).is_none());
    }
}
