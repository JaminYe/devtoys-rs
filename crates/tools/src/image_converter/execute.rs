use std::path::PathBuf;

use super::{convert_image, ImageConvertError, ImageTargetFormat};

/// One successfully converted input: the original path plus its converted bytes.
#[derive(Debug)]
pub struct SuccessfulConversion {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

/// One failed conversion: the original path plus a human-readable reason.
#[derive(Debug)]
pub struct FailedConversion {
    pub path: PathBuf,
    pub error: String,
}

/// Aggregated result of converting a batch of paths.
#[derive(Debug, Default)]
pub struct ConversionBatch {
    pub successes: Vec<SuccessfulConversion>,
    pub failures: Vec<FailedConversion>,
}

impl ConversionBatch {
    /// Number of paths that converted successfully.
    pub fn succeeded(&self) -> usize {
        self.successes.len()
    }

    /// Number of paths that failed to convert.
    pub fn failed(&self) -> usize {
        self.failures.len()
    }

    /// Concatenated failure messages for display; `None` when nothing failed.
    pub fn error_message(&self) -> Option<String> {
        if self.failures.is_empty() {
            return None;
        }
        Some(
            self.failures
                .iter()
                .map(|f| format!("{}：{}", f.path.display(), f.error))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

/// Split a multi-line input into non-empty, trimmed path strings.
pub fn parse_paths(input: &str) -> Vec<String> {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Read + convert each path in order. Never panics; an empty input yields an
/// empty batch (`succeeded` and `failed` are both zero).
pub fn convert_paths(paths: &[String], format: ImageTargetFormat) -> ConversionBatch {
    let mut batch = ConversionBatch::default();
    for path in paths {
        let result = match std::fs::read(path) {
            Ok(bytes) => match convert_image(&bytes, format) {
                Ok(converted) => Ok(converted),
                Err(err) => Err(err),
            },
            Err(_) => Err(ImageConvertError::Read),
        };
        match result {
            Ok(bytes) => batch.successes.push(SuccessfulConversion {
                path: PathBuf::from(path),
                bytes,
            }),
            Err(err) => batch.failures.push(FailedConversion {
                path: PathBuf::from(path),
                error: err.to_string(),
            }),
        }
    }
    batch
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;

    use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

    use super::*;

    fn png_bytes() -> Vec<u8> {
        let img = RgbImage::from_pixel(1, 1, Rgb([10, 20, 30]));
        let mut buf = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        buf
    }

    /// Write `bytes` into a uniquely named temp file and return its path.
    fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("imgconv-exec-test-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_file(&dir);
        std::fs::write(&dir, bytes).unwrap();
        dir
    }

    #[test]
    fn parse_paths_trims_and_skips_blank_lines() {
        let parsed = parse_paths("  a.png \n\nb.jpeg\n\tc.png");
        assert_eq!(
            parsed,
            vec![
                "a.png".to_string(),
                "b.jpeg".to_string(),
                "c.png".to_string()
            ]
        );
        assert!(parse_paths("").is_empty());
        assert!(parse_paths("\n  \n").is_empty());
    }

    #[test]
    fn two_valid_paths_both_succeed() {
        let a = temp_file("a.png", &png_bytes());
        let b = temp_file("b.png", &png_bytes());
        let paths = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let batch = convert_paths(&paths, ImageTargetFormat::Png);

        assert_eq!(batch.succeeded(), 2);
        assert_eq!(batch.failed(), 0);
        assert!(batch.successes.iter().all(|s| !s.bytes.is_empty()));
        assert!(batch.error_message().is_none());

        let _ = std::fs::remove_file(&a);
        let _ = std::fs::remove_file(&b);
    }

    #[test]
    fn one_valid_one_unconvertible_succeeds_once_fails_once() {
        let valid = temp_file("valid.png", &png_bytes());
        let bad = temp_file("bad.png", b"not an image");
        let paths = vec![
            valid.to_string_lossy().into_owned(),
            bad.to_string_lossy().into_owned(),
        ];
        let batch = convert_paths(&paths, ImageTargetFormat::Png);

        assert_eq!(batch.succeeded(), 1);
        assert_eq!(batch.failed(), 1);
        assert!(batch
            .failures
            .iter()
            .any(|f| f.path == bad && !f.error.is_empty()));
        assert!(batch.error_message().is_some());

        let _ = std::fs::remove_file(&valid);
        let _ = std::fs::remove_file(&bad);
    }

    #[test]
    fn nonexistent_path_is_reported_as_failure() {
        let missing = std::env::temp_dir().join(format!(
            "imgconv-exec-test-{}-missing.png",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&missing);
        let paths = vec![missing.to_string_lossy().into_owned()];
        let batch = convert_paths(&paths, ImageTargetFormat::Png);

        assert_eq!(batch.succeeded(), 0);
        assert_eq!(batch.failed(), 1);
        assert!(batch.failures[0].path == missing);
        assert!(!batch.failures[0].error.is_empty());
    }

    #[test]
    fn empty_paths_yield_empty_batch() {
        let batch = convert_paths(&[], ImageTargetFormat::Png);

        assert_eq!(batch.succeeded(), 0);
        assert_eq!(batch.failed(), 0);
        assert!(batch.successes.is_empty());
        assert!(batch.failures.is_empty());
        assert!(batch.error_message().is_none());
    }
}
