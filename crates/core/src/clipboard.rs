use devtoys_api::RawData;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Windows `DROPFILES` / `CF_HDROP` header size (`pFiles`, `pt`, `fNC`, `fWide`).
const DROPFILES_HEADER_LEN: usize = 20;

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
    let expected_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
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

        // File objects first. Explorer/Finder also place a path as text, which
        // must stay `RawData::Text` only when CF_HDROP / file-list is absent.
        // arboard 3.6.1 `Get::file_list` reads CF_HDROP on Windows, file URIs
        // on Linux, and NSFilenamesPboardType on macOS. OS E2E is not claimed
        // by unit tests; they inject `RawData::Files` or parse mocked CF_HDROP.
        if let Ok(paths) = clipboard.get().file_list() {
            if let Some(raw) = files_from_os_paths(paths) {
                return Some(raw);
            }
        }

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

/// Turns OS file-list paths into [`RawData::Files`].
///
/// Empty paths are dropped. Missing or unreadable files are kept so callers
/// can surface the original path; an empty result is `None`.
pub fn files_from_os_paths<I, P>(paths: I) -> Option<RawData>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let files: Vec<String> = paths
        .into_iter()
        .map(|p| p.as_ref().to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .collect();
    if files.is_empty() {
        None
    } else {
        Some(RawData::Files(files))
    }
}

/// Parses a Windows `CF_HDROP` / `DROPFILES` payload into file paths.
///
/// `fWide != 0` is UTF-16LE; otherwise bytes are treated as Latin-1 (ACP is
/// not available in tests). Double-NUL terminates the list. Paths with spaces
/// or non-ASCII are kept intact; empty entries are skipped.
pub fn parse_cf_hdrop(bytes: &[u8]) -> Option<Vec<String>> {
    if bytes.len() < DROPFILES_HEADER_LEN {
        return None;
    }
    let p_files = u32::from_le_bytes(bytes[0..4].try_into().ok()?) as usize;
    let f_wide = i32::from_le_bytes(bytes[16..20].try_into().ok()?) != 0;
    if p_files < DROPFILES_HEADER_LEN || p_files > bytes.len() {
        return None;
    }
    let list = &bytes[p_files..];
    let paths = if f_wide {
        parse_wide_zstring_list(list)?
    } else {
        parse_ansi_zstring_list(list)?
    };
    let paths: Vec<String> = paths.into_iter().filter(|s| !s.is_empty()).collect();
    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}

fn parse_wide_zstring_list(bytes: &[u8]) -> Option<Vec<String>> {
    if bytes.len() < 2 {
        return None;
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let mut paths = Vec::new();
    let mut start = 0;
    let mut saw_terminator = false;
    for (i, unit) in units.iter().copied().enumerate() {
        if unit != 0 {
            continue;
        }
        if i == start {
            saw_terminator = true;
            break;
        }
        paths.push(String::from_utf16_lossy(&units[start..i]));
        start = i + 1;
    }
    if !saw_terminator {
        return None;
    }
    Some(paths)
}

fn parse_ansi_zstring_list(bytes: &[u8]) -> Option<Vec<String>> {
    if bytes.is_empty() {
        return None;
    }
    let mut paths = Vec::new();
    let mut start = 0;
    let mut saw_terminator = false;
    for (i, b) in bytes.iter().copied().enumerate() {
        if b != 0 {
            continue;
        }
        if i == start {
            saw_terminator = true;
            break;
        }
        paths.push(bytes[start..i].iter().map(|&c| c as char).collect());
        start = i + 1;
    }
    if !saw_terminator {
        return None;
    }
    Some(paths)
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

    /// Sets the clipboard content to a file-object list (not path text).
    pub fn set_files(&self, files: impl Into<Vec<String>>) {
        self.set(Some(RawData::Files(files.into())));
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
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
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

    fn dropfiles_wide(paths: &[&str]) -> Vec<u8> {
        let mut list = Vec::new();
        for path in paths {
            for unit in path.encode_utf16() {
                list.extend_from_slice(&unit.to_le_bytes());
            }
            list.extend_from_slice(&0u16.to_le_bytes());
        }
        list.extend_from_slice(&0u16.to_le_bytes());
        let mut bytes = vec![0u8; DROPFILES_HEADER_LEN];
        bytes[0..4].copy_from_slice(&(DROPFILES_HEADER_LEN as u32).to_le_bytes());
        bytes[16..20].copy_from_slice(&1i32.to_le_bytes());
        bytes.extend(list);
        bytes
    }

    fn dropfiles_ansi(paths: &[&str]) -> Vec<u8> {
        let mut list = Vec::new();
        for path in paths {
            list.extend(path.bytes());
            list.push(0);
        }
        list.push(0);
        let mut bytes = vec![0u8; DROPFILES_HEADER_LEN];
        bytes[0..4].copy_from_slice(&(DROPFILES_HEADER_LEN as u32).to_le_bytes());
        bytes.extend(list);
        bytes
    }

    #[test]
    fn parse_cf_hdrop_keeps_spaces_and_chinese() {
        let bytes = dropfiles_wide(&[r"C:\Users\图片\foo bar.png"]);
        assert_eq!(
            parse_cf_hdrop(&bytes).as_deref(),
            Some(&[r"C:\Users\图片\foo bar.png".to_string()][..])
        );
    }

    #[test]
    fn parse_cf_hdrop_multiple_and_ansi() {
        let wide = dropfiles_wide(&[r"C:\a.png", r"D:\b 中文.jpg"]);
        assert_eq!(
            parse_cf_hdrop(&wide).unwrap(),
            vec![r"C:\a.png".to_string(), r"D:\b 中文.jpg".to_string()]
        );
        let ansi = dropfiles_ansi(&[r"C:\temp\file name.txt"]);
        assert_eq!(
            parse_cf_hdrop(&ansi).as_deref(),
            Some(&[r"C:\temp\file name.txt".to_string()][..])
        );
    }

    #[test]
    fn parse_cf_hdrop_rejects_empty_and_truncated() {
        assert!(parse_cf_hdrop(&[]).is_none());
        assert!(parse_cf_hdrop(&[0; 10]).is_none());
        // Double-NUL with no paths; a leading empty entry is the terminator.
        assert!(parse_cf_hdrop(&dropfiles_wide(&[])).is_none());
        assert!(parse_cf_hdrop(&dropfiles_wide(&["", r"C:\keep.png"])).is_none());
        let truncated = dropfiles_wide(&[r"C:\a.png"]);
        assert!(parse_cf_hdrop(&truncated[..truncated.len() - 2]).is_none());
    }

    #[test]
    fn files_from_os_paths_drops_empty_keeps_missing() {
        assert!(files_from_os_paths(Vec::<String>::new()).is_none());
        assert!(files_from_os_paths([""]).is_none());
        let missing = std::path::PathBuf::from(r"C:\no such file 中文.png");
        match files_from_os_paths([&missing]) {
            Some(RawData::Files(files)) => {
                assert_eq!(files, vec![r"C:\no such file 中文.png"]);
            }
            other => panic!("expected Files, got {other:?}"),
        }
    }
}
