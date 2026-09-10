use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{convert_image_from, ImageConvertError, ImageTargetFormat};

/// One successfully converted input: the original path plus its converted bytes.
#[derive(Debug, Clone)]
pub struct SuccessfulConversion {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

/// One failed conversion: the original path plus a human-readable reason.
#[derive(Debug, Clone)]
pub struct FailedConversion {
    pub path: PathBuf,
    pub error: String,
}

/// Aggregated result of converting a batch of paths.
#[derive(Debug, Default, Clone)]
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

    /// Remove a successful conversion by index. Out-of-range returns `None`.
    pub fn remove_success(&mut self, index: usize) -> Option<SuccessfulConversion> {
        if index >= self.successes.len() {
            return None;
        }
        Some(self.successes.remove(index))
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

/// Convert in-memory image bytes (clipboard / smart paste). `source` is only
/// used as the save-name stem; nothing is read from disk.
pub fn convert_memory(
    bytes: &[u8],
    format: ImageTargetFormat,
    source: impl AsRef<Path>,
) -> ConversionBatch {
    let path = source.as_ref().to_path_buf();
    match convert_image_from(bytes, &path, format) {
        Ok(converted) => ConversionBatch {
            successes: vec![SuccessfulConversion {
                path,
                bytes: converted,
            }],
            failures: vec![],
        },
        Err(err) => ConversionBatch {
            successes: vec![],
            failures: vec![FailedConversion {
                path,
                error: err.to_string(),
            }],
        },
    }
}

/// Read + convert each path in order. Never panics; an empty input yields an
/// empty batch (`succeeded` and `failed` are both zero).
pub fn convert_paths(paths: &[String], format: ImageTargetFormat) -> ConversionBatch {
    let mut batch = ConversionBatch::default();
    for path in paths {
        let result = match std::fs::read(path) {
            Ok(bytes) => match convert_image_from(&bytes, Path::new(path), format) {
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

/// `{stem}.{extension}` for `source` in the given target format.
pub fn output_filename(source: &Path, format: ImageTargetFormat) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("image");
    PathBuf::from(format!("{stem}.{}", format.extension()))
}

/// Per-item outcome of saving a converted image to a destination directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStatus {
    Saved,
    SkippedConflict,
    WriteFailed,
}

/// One save attempt for a successful conversion.
#[derive(Debug, Clone)]
pub struct SavedItem {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub status: SaveStatus,
    pub message: Option<String>,
}

/// Aggregated result of saving a conversion batch to a directory.
#[derive(Debug, Default, Clone)]
pub struct SaveReport {
    pub items: Vec<SavedItem>,
}

impl SaveReport {
    pub fn saved(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == SaveStatus::Saved)
            .count()
    }

    pub fn skipped(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == SaveStatus::SkippedConflict)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == SaveStatus::WriteFailed)
            .count()
    }
}

/// Write this conversion's bytes to `dest`. Overwrites if `dest` already exists
/// (explicit Save As). Does not create parent directories.
pub fn save_one(item: &SuccessfulConversion, dest: &Path) -> Result<(), ImageConvertError> {
    std::fs::write(dest, &item.bytes).map_err(|_| ImageConvertError::Write)
}

/// Save converted bytes into `dest_dir`. Never overwrites: existing files and
/// same-batch duplicate output names are skipped as conflicts.
pub fn save_batch(
    successes: &[SuccessfulConversion],
    dest_dir: &Path,
    format: ImageTargetFormat,
) -> SaveReport {
    if !dest_dir.is_dir() {
        return SaveReport {
            items: successes
                .iter()
                .map(|item| SavedItem {
                    source: item.path.clone(),
                    dest: dest_dir.join(output_filename(&item.path, format)),
                    status: SaveStatus::WriteFailed,
                    message: Some("目标不是目录".to_string()),
                })
                .collect(),
        };
    }

    let mut report = SaveReport::default();
    let mut claimed = HashSet::new();
    for item in successes {
        let filename = output_filename(&item.path, format);
        let dest = dest_dir.join(&filename);
        let key = filename.to_string_lossy().into_owned();
        if !claimed.insert(key) {
            report.items.push(SavedItem {
                source: item.path.clone(),
                dest,
                status: SaveStatus::SkippedConflict,
                message: Some("同批次输出重名".to_string()),
            });
            continue;
        }
        if dest.is_file() {
            report.items.push(SavedItem {
                source: item.path.clone(),
                dest,
                status: SaveStatus::SkippedConflict,
                message: Some("目标文件已存在".to_string()),
            });
            continue;
        }
        match std::fs::write(&dest, &item.bytes) {
            Ok(()) => report.items.push(SavedItem {
                source: item.path.clone(),
                dest,
                status: SaveStatus::Saved,
                message: None,
            }),
            Err(_) => report.items.push(SavedItem {
                source: item.path.clone(),
                dest,
                status: SaveStatus::WriteFailed,
                message: Some("无法写入文件".to_string()),
            }),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

    use super::*;

    static SAVE_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

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

    fn unique_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "imgconv-save-{}-{}-{}",
            std::process::id(),
            SAVE_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
            label
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_png_named(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, png_bytes()).unwrap();
        path
    }

    fn cleanup(paths: &[&Path]) {
        for path in paths {
            if path.is_dir() {
                let _ = std::fs::remove_dir_all(path);
            } else {
                let _ = std::fs::remove_file(path);
            }
        }
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

    #[test]
    fn two_conversions_save_with_matching_encoding() {
        let src = unique_dir("src-two");
        let dest = unique_dir("dest-two");
        let a = write_png_named(&src, "a.png");
        let b = write_png_named(&src, "b.png");
        let paths = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let batch = convert_paths(&paths, ImageTargetFormat::Jpeg);
        assert_eq!(batch.succeeded(), 2);

        let report = save_batch(&batch.successes, &dest, ImageTargetFormat::Jpeg);
        assert_eq!(report.saved(), 2);
        assert_eq!(report.skipped(), 0);
        assert_eq!(report.failed(), 0);

        for name in ["a.jpg", "b.jpg"] {
            let path = dest.join(name);
            assert_eq!(path.extension().and_then(|e| e.to_str()), Some("jpg"));
            let bytes = std::fs::read(&path).unwrap();
            assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Jpeg);
            assert!(image::load_from_memory(&bytes).is_ok());
        }

        cleanup(&[&src, &dest]);
    }

    #[test]
    fn existing_dest_file_is_skipped_not_overwritten() {
        let src = unique_dir("src-exists");
        let dest = unique_dir("dest-exists");
        let a = write_png_named(&src, "photo.png");
        let existing = dest.join("photo.jpg");
        std::fs::write(&existing, b"keep-me").unwrap();

        let batch = convert_paths(&[a.to_string_lossy().into_owned()], ImageTargetFormat::Jpeg);
        assert_eq!(batch.succeeded(), 1);
        let report = save_batch(&batch.successes, &dest, ImageTargetFormat::Jpeg);

        assert_eq!(report.saved(), 0);
        assert_eq!(report.skipped(), 1);
        assert_eq!(report.items[0].status, SaveStatus::SkippedConflict);
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep-me");

        cleanup(&[&src, &dest]);
    }

    #[test]
    fn duplicate_output_names_skip_the_second() {
        let src_a = unique_dir("src-dup-a");
        let src_b = unique_dir("src-dup-b");
        let dest = unique_dir("dest-dup");
        let a = write_png_named(&src_a, "same.png");
        let b = write_png_named(&src_b, "same.png");
        let paths = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let batch = convert_paths(&paths, ImageTargetFormat::Png);
        assert_eq!(batch.succeeded(), 2);

        let report = save_batch(&batch.successes, &dest, ImageTargetFormat::Png);
        assert_eq!(report.saved(), 1);
        assert_eq!(report.skipped(), 1);
        assert_eq!(report.items[0].status, SaveStatus::Saved);
        assert_eq!(report.items[1].status, SaveStatus::SkippedConflict);
        assert_eq!(
            std::fs::read(dest.join("same.png")).unwrap(),
            batch.successes[0].bytes
        );

        cleanup(&[&src_a, &src_b, &dest]);
    }

    #[test]
    fn partial_convert_failure_still_saves_successes() {
        let src = unique_dir("src-partial");
        let dest = unique_dir("dest-partial");
        let valid = write_png_named(&src, "ok.png");
        let bad = src.join("bad.png");
        std::fs::write(&bad, b"not an image").unwrap();
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

        let report = save_batch(&batch.successes, &dest, ImageTargetFormat::Png);
        assert_eq!(report.saved(), 1);
        assert_eq!(report.failed(), 0);
        let saved = dest.join("ok.png");
        assert!(saved.is_file());
        assert!(image::load_from_memory(&std::fs::read(&saved).unwrap()).is_ok());
        assert_eq!(batch.failed(), 1);

        cleanup(&[&src, &dest]);
    }

    #[test]
    fn write_failure_is_reported_without_dropping_bytes() {
        let src = unique_dir("src-write");
        let dest = unique_dir("dest-write");
        let a = write_png_named(&src, "a.png");
        let b = write_png_named(&src, "b.png");
        std::fs::create_dir(dest.join("a.png")).unwrap();
        let paths = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let batch = convert_paths(&paths, ImageTargetFormat::Png);
        assert_eq!(batch.succeeded(), 2);

        let report = save_batch(&batch.successes, &dest, ImageTargetFormat::Png);
        assert_eq!(report.failed(), 1);
        assert_eq!(report.saved(), 1);
        assert_eq!(report.items[0].status, SaveStatus::WriteFailed);
        assert_eq!(report.items[1].status, SaveStatus::Saved);
        assert!(batch.successes.iter().all(|s| !s.bytes.is_empty()));
        assert!(dest.join("b.png").is_file());

        let not_dir = src.join("not-a-dir");
        std::fs::write(&not_dir, b"file").unwrap();
        let failed = save_batch(&batch.successes, &not_dir, ImageTargetFormat::Png);
        assert_eq!(failed.failed(), 2);
        assert!(failed
            .items
            .iter()
            .all(|i| i.status == SaveStatus::WriteFailed));
        assert!(batch.successes.iter().all(|s| !s.bytes.is_empty()));

        cleanup(&[&src, &dest]);
    }

    fn known_rgba_png() -> (Vec<u8>, [[u8; 4]; 4]) {
        let pixels = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
        ];
        let mut img = image::RgbaImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgba(pixels[0]));
        img.put_pixel(1, 0, image::Rgba(pixels[1]));
        img.put_pixel(0, 1, image::Rgba(pixels[2]));
        img.put_pixel(1, 1, image::Rgba(pixels[3]));
        let mut buf = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        (buf, pixels)
    }

    #[test]
    fn memory_png_keeps_known_pixels_and_saves() {
        let (png, pixels) = known_rgba_png();
        let batch = convert_memory(&png, ImageTargetFormat::Png, "clipboard.png");
        assert_eq!(batch.succeeded(), 1);
        assert_eq!(batch.failed(), 0);
        let converted = &batch.successes[0].bytes;
        assert_ne!(converted.as_slice(), b"image/png");
        let loaded = image::load_from_memory(converted).unwrap().to_rgba8();
        assert_eq!(loaded.dimensions(), (2, 2));
        assert_eq!(loaded.get_pixel(0, 0).0, pixels[0]);
        assert_eq!(loaded.get_pixel(1, 0).0, pixels[1]);
        assert_eq!(loaded.get_pixel(0, 1).0, pixels[2]);
        assert_eq!(loaded.get_pixel(1, 1).0, pixels[3]);

        let dest = unique_dir("mem-save");
        let report = save_batch(&batch.successes, &dest, ImageTargetFormat::Png);
        assert_eq!(report.saved(), 1);
        let saved = dest.join("clipboard.png");
        assert!(saved.is_file());
        let round = image::load_from_memory(&std::fs::read(&saved).unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(round.get_pixel(0, 0).0, pixels[0]);

        let unused = unique_dir("mem-cancel");
        assert!(std::fs::read_dir(&unused).unwrap().next().is_none());
        cleanup(&[&dest, &unused]);
    }

    #[test]
    fn memory_invalid_bytes_are_a_failure() {
        let batch = convert_memory(b"image/png", ImageTargetFormat::Png, "clipboard.png");
        assert_eq!(batch.succeeded(), 0);
        assert_eq!(batch.failed(), 1);
    }

    #[test]
    fn save_uses_the_batch_that_was_passed() {
        let src = unique_dir("src-indep");
        let dest = unique_dir("dest-indep");
        let a = write_png_named(&src, "first.png");
        let b = write_png_named(&src, "second.png");
        let batch_jpeg =
            convert_paths(&[a.to_string_lossy().into_owned()], ImageTargetFormat::Jpeg);
        let batch_png = convert_paths(&[b.to_string_lossy().into_owned()], ImageTargetFormat::Png);

        let jpeg_report = save_batch(&batch_jpeg.successes, &dest, ImageTargetFormat::Jpeg);
        assert_eq!(jpeg_report.saved(), 1);
        let jpeg_path = dest.join("first.jpg");
        assert!(jpeg_path.is_file());
        assert!(!dest.join("second.png").exists());
        assert_eq!(
            image::guess_format(&std::fs::read(&jpeg_path).unwrap()).unwrap(),
            ImageFormat::Jpeg
        );

        let png_report = save_batch(&batch_png.successes, &dest, ImageTargetFormat::Png);
        assert_eq!(png_report.saved(), 1);
        let png_path = dest.join("second.png");
        assert!(png_path.is_file());
        assert!(jpeg_path.is_file());
        assert_eq!(
            image::guess_format(&std::fs::read(&png_path).unwrap()).unwrap(),
            ImageFormat::Png
        );

        cleanup(&[&src, &dest]);
    }

    fn three_png_successes(label: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf, ConversionBatch) {
        let src = unique_dir(&format!("src-{label}"));
        let a = write_png_named(&src, "a.png");
        let b = write_png_named(&src, "b.png");
        let c = write_png_named(&src, "c.png");
        let paths = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
            c.to_string_lossy().into_owned(),
        ];
        let batch = convert_paths(&paths, ImageTargetFormat::Png);
        assert_eq!(batch.succeeded(), 3);
        assert_eq!(batch.failed(), 0);
        (src, a, b, c, batch)
    }

    #[test]
    fn save_one_writes_only_the_middle_item() {
        let (src, a, b, c, batch) = three_png_successes("one");
        let dest = unique_dir("dest-one");
        let picked = dest.join("middle-picked.png");

        save_one(&batch.successes[1], &picked).unwrap();

        assert!(picked.is_file());
        assert_eq!(std::fs::read(&picked).unwrap(), batch.successes[1].bytes);
        assert!(!dest.join("a.png").exists());
        assert!(!dest.join("b.png").exists());
        assert!(!dest.join("c.png").exists());
        assert!(!dest
            .join(output_filename(&a, ImageTargetFormat::Png))
            .exists());
        assert_ne!(
            picked,
            dest.join(output_filename(&b, ImageTargetFormat::Png))
        );
        assert!(!dest
            .join(output_filename(&c, ImageTargetFormat::Png))
            .exists());
        let written: Vec<_> = std::fs::read_dir(&dest).unwrap().collect();
        assert_eq!(written.len(), 1);

        cleanup(&[&src, &dest]);
    }

    #[test]
    fn remove_success_then_save_batch_writes_remaining() {
        let (src, a, b, c, mut batch) = three_png_successes("rm");
        let dest = unique_dir("dest-rm");

        let removed = batch.remove_success(1).expect("middle item");
        assert_eq!(removed.path, b);
        assert_eq!(batch.succeeded(), 2);
        assert!(batch.successes.iter().all(|s| s.path != b));
        assert_eq!(batch.successes[0].path, a);
        assert_eq!(batch.successes[1].path, c);

        let report = save_batch(&batch.successes, &dest, ImageTargetFormat::Png);
        assert_eq!(report.saved(), 2);
        assert_eq!(report.skipped(), 0);
        assert_eq!(report.failed(), 0);
        assert!(dest.join("a.png").is_file());
        assert!(!dest.join("b.png").exists());
        assert!(dest.join("c.png").is_file());
        assert_eq!(
            std::fs::read(dest.join("a.png")).unwrap(),
            batch.successes[0].bytes
        );
        assert_eq!(
            std::fs::read(dest.join("c.png")).unwrap(),
            batch.successes[1].bytes
        );

        cleanup(&[&src, &dest]);
    }

    #[test]
    fn not_calling_save_one_writes_nothing() {
        let (src, _, _, _, batch) = three_png_successes("nosave");
        let dest = unique_dir("dest-nosave");
        let picked = dest.join("cancelled.png");

        assert_eq!(batch.succeeded(), 3);
        assert!(!picked.exists());
        assert!(std::fs::read_dir(&dest).unwrap().next().is_none());

        cleanup(&[&src, &dest]);
    }

    #[test]
    fn save_one_overwrites_existing_dest() {
        let (src, _, _, _, batch) = three_png_successes("ow");
        let dest = unique_dir("dest-ow");
        let picked = dest.join("explicit.png");
        std::fs::write(&picked, b"keep-me").unwrap();

        save_one(&batch.successes[1], &picked).unwrap();

        assert_eq!(std::fs::read(&picked).unwrap(), batch.successes[1].bytes);
        assert_ne!(std::fs::read(&picked).unwrap().as_slice(), b"keep-me");

        cleanup(&[&src, &dest]);
    }

    #[test]
    fn save_batch_still_skips_existing_conflict() {
        let (src, _, _, _, batch) = three_png_successes("skip");
        let dest = unique_dir("dest-skip");
        let existing = dest.join("b.png");
        std::fs::write(&existing, b"keep-me").unwrap();

        let report = save_batch(&batch.successes, &dest, ImageTargetFormat::Png);
        assert_eq!(report.saved(), 2);
        assert_eq!(report.skipped(), 1);
        assert_eq!(report.items[1].status, SaveStatus::SkippedConflict);
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep-me");
        assert!(dest.join("a.png").is_file());
        assert!(dest.join("c.png").is_file());

        cleanup(&[&src, &dest]);
    }
}
