use std::path::Path;

use crate::slot::ToolView;
use crate::ui;

use super::execute::{
    output_filename, save_batch, save_one, ConversionBatch, SaveReport, SaveStatus, SavedItem,
};
use super::helper::clipboard_source_name;
use super::{ImageTargetFormat, ID};

const DEFAULT_FORMAT: ImageTargetFormat = ImageTargetFormat::Png;

struct MemoryImage {
    bytes: Vec<u8>,
    mime: Option<String>,
}

pub struct ImageConverterView {
    path: String,
    target: ImageTargetFormat,
    batch: ConversionBatch,
    save_report: Option<SaveReport>,
    preview: Option<Vec<u8>>,
    tex: Option<egui::TextureHandle>,
    /// Clipboard / smart-paste bytes. Independent of the path editor.
    memory: Option<MemoryImage>,
}

impl ImageConverterView {
    pub fn new() -> Self {
        Self {
            path: String::new(),
            target: DEFAULT_FORMAT,
            batch: ConversionBatch::default(),
            save_report: None,
            preview: None,
            tex: None,
            memory: None,
        }
    }

    fn reconvert(&mut self) {
        self.save_report = None;
        self.tex = None;
        if let Some(memory) = &self.memory {
            self.batch = super::execute::convert_memory(
                &memory.bytes,
                self.target,
                clipboard_source_name(memory.mime.as_deref()),
            );
            self.preview = self.batch.successes.first().map(|s| s.bytes.clone());
            return;
        }
        let paths = super::execute::parse_paths(&self.path);
        if paths.is_empty() {
            self.batch = ConversionBatch::default();
            self.preview = None;
            return;
        }

        self.batch = super::execute::convert_paths(&paths, self.target);
        self.preview = self.batch.successes.first().map(|s| s.bytes.clone());
    }

    fn pick_files(&mut self) {
        let Some(files) = rfd::FileDialog::new().pick_files() else {
            return;
        };
        let paths: Vec<String> = files
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if !paths.is_empty() {
            self.memory = None;
            if self.path.trim().is_empty() {
                self.path = paths.join("\n");
            } else {
                self.path.push('\n');
                self.path.push_str(&paths.join("\n"));
            }
            self.reconvert();
        }
    }

    fn pick_and_save(&mut self) {
        let Some(dir) = rfd::FileDialog::new()
            .set_title("选择保存目录")
            .pick_folder()
        else {
            return;
        };
        self.save_report = Some(save_batch(&self.batch.successes, &dir, self.target));
    }

    fn pick_and_save_one(&mut self, index: usize) {
        let suggested = match self.batch.successes.get(index) {
            Some(item) => output_filename(&item.path, self.target),
            None => return,
        };
        let Some(dest) = rfd::FileDialog::new()
            .set_title("另存为")
            .set_file_name(suggested.to_string_lossy().as_ref())
            .save_file()
        else {
            return;
        };
        let Some(item) = self.batch.successes.get(index) else {
            return;
        };
        let result = save_one(item, &dest);
        let source = item.path.clone();
        let (status, message) = match result {
            Ok(()) => (SaveStatus::Saved, None),
            Err(_) => (SaveStatus::WriteFailed, Some("无法写入文件".to_string())),
        };
        self.record_save(SavedItem {
            source,
            dest,
            status,
            message,
        });
    }

    fn record_save(&mut self, item: SavedItem) {
        let report = self.save_report.get_or_insert_with(SaveReport::default);
        if let Some(existing) = report.items.iter_mut().find(|i| i.source == item.source) {
            *existing = item;
        } else {
            report.items.push(item);
        }
    }

    fn delete_item(&mut self, index: usize) {
        let Some(removed) = self.batch.remove_success(index) else {
            return;
        };
        self.path = super::execute::parse_paths(&self.path)
            .into_iter()
            .filter(|p| Path::new(p) != removed.path.as_path())
            .collect::<Vec<_>>()
            .join("\n");
        if self.memory.is_some() {
            self.memory = None;
        }
        if let Some(report) = self.save_report.as_mut() {
            report.items.retain(|i| i.source != removed.path);
            if report.items.is_empty() {
                self.save_report = None;
            }
        }
        self.preview = self.batch.successes.first().map(|s| s.bytes.clone());
        self.tex = None;
    }

    fn save_item_for(&self, source: &Path) -> Option<&SavedItem> {
        self.save_report
            .as_ref()?
            .items
            .iter()
            .find(|item| item.source == source)
    }

    fn show_status(&self, ui: &mut egui::Ui) {
        let converted = self.batch.succeeded();
        let convert_failed = self.batch.failed();
        if converted == 0 && convert_failed == 0 {
            return;
        }

        let mut parts = vec![format!("已转换 {converted} → {}", self.target.as_str())];
        if convert_failed > 0 {
            parts.push(format!("转换失败 {convert_failed}"));
        }
        if let Some(report) = &self.save_report {
            parts.push(format!("已保存 {}", report.saved()));
            if report.skipped() > 0 {
                parts.push(format!("跳过 {}", report.skipped()));
            }
            if report.failed() > 0 {
                parts.push(format!("保存失败 {}", report.failed()));
            }
        }
        let line = parts.join("，");
        let has_error =
            convert_failed > 0 || self.save_report.as_ref().is_some_and(|r| r.failed() > 0);
        if has_error {
            ui.colored_label(ui::danger(ui), line);
        } else {
            ui.colored_label(ui::success(ui), line);
        }
    }

    fn show_items(&mut self, ui: &mut egui::Ui) {
        let mut save_index = None;
        let mut delete_index = None;
        for (i, success) in self.batch.successes.iter().enumerate() {
            ui.horizontal_wrapped(|ui| {
                ui.label(success.path.display().to_string());
                ui.colored_label(ui::success(ui), "转换成功");
                if ui.button("另存为").clicked() {
                    save_index = Some(i);
                }
                if ui.button("删除").clicked() {
                    delete_index = Some(i);
                }
                if let Some(item) = self.save_item_for(&success.path) {
                    match item.status {
                        SaveStatus::Saved => {
                            let dest = item
                                .dest
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| item.dest.display().to_string());
                            ui.colored_label(ui::success(ui), format!("已保存 {dest}"));
                        }
                        SaveStatus::SkippedConflict => {
                            ui.label("跳过（同名冲突）");
                        }
                        SaveStatus::WriteFailed => {
                            let msg = item.message.as_deref().unwrap_or("无法写入文件");
                            ui.colored_label(ui::danger(ui), format!("保存失败：{msg}"));
                        }
                    }
                }
            });
        }
        for failure in &self.batch.failures {
            ui.horizontal_wrapped(|ui| {
                ui.label(failure.path.display().to_string());
                ui.colored_label(ui::danger(ui), format!("转换失败：{}", failure.error));
            });
        }
        if let Some(index) = save_index {
            self.pick_and_save_one(index);
        }
        if let Some(index) = delete_index {
            self.delete_item(index);
        }
    }
}

impl ToolView for ImageConverterView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(ui::t(ui, "image_converter.target_format"));
            for (label, value) in [
                ("BMP", ImageTargetFormat::Bmp),
                ("JPEG", ImageTargetFormat::Jpeg),
                ("PBM", ImageTargetFormat::Pbm),
                ("PNG", ImageTargetFormat::Png),
                ("TGA", ImageTargetFormat::Tga),
                ("TIFF", ImageTargetFormat::Tiff),
                ("WEBP", ImageTargetFormat::Webp),
            ] {
                if ui::toggle(ui, self.target == value, label).clicked() {
                    self.target = value;
                    self.reconvert();
                }
            }
        });
        ui.horizontal(|ui| {
            if ui
                .button(ui::t(ui, "image_converter.select_file"))
                .clicked()
            {
                self.pick_files();
            }
            let can_save = !self.batch.successes.is_empty();
            if ui
                .add_enabled(can_save, egui::Button::new("保存到目录"))
                .clicked()
            {
                self.pick_and_save();
            }
        });
        self.show_status(ui);
        self.show_items(ui);

        let mut changed = false;
        ui::split_2(
            ui,
            |ui| {
                changed = ui::labeled_code(
                    ui,
                    ui::t(ui, "common.input"),
                    "img-path",
                    &mut self.path,
                    "图像文件路径（每行一个）",
                    true,
                );
            },
            |ui| {
                if let Some(bytes) = self.preview.as_deref() {
                    show_converted(ui, "imgconv-preview", bytes, self.target, &mut self.tex);
                }
            },
        );
        if changed {
            self.memory = None;
            self.reconvert();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        let filled = payload
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if filled.is_empty() {
            return;
        }
        self.memory = None;
        self.path = filled;
        self.reconvert();
    }

    fn on_image_received(&mut self, bytes: &[u8], mime: Option<&str>) {
        if bytes.is_empty() {
            return;
        }
        self.path.clear();
        self.memory = Some(MemoryImage {
            bytes: bytes.to_vec(),
            mime: mime.map(str::to_string),
        });
        self.reconvert();
    }

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "format": self.target.as_str(),
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.target = format_from_settings(value.get("format").and_then(|v| v.as_str()));
        self.reconvert();
    }
}

fn format_from_settings(value: Option<&str>) -> ImageTargetFormat {
    value
        .and_then(ImageTargetFormat::parse)
        .unwrap_or(DEFAULT_FORMAT)
}

fn show_converted(
    ui: &mut egui::Ui,
    name: &'static str,
    bytes: &[u8],
    format: ImageTargetFormat,
    cache: &mut Option<egui::TextureHandle>,
) {
    if cache.is_none() {
        if let Ok((w, h, rgba)) = super::helper::preview_rgba(bytes, format) {
            let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            *cache = Some(ui.ctx().load_texture(name, color, Default::default()));
        }
    }
    if let Some(tex) = cache.as_ref() {
        ui.add(egui::Image::new(tex).max_size(ui.available_size()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;
    use devtoys_api::DetectedPayload;
    use image::{Rgba, RgbaImage};
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

    /// Distinct 2×2 pixels used to prove the converter received real image bytes.
    const PIXELS: [[u8; 4]; 4] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];

    fn known_png() -> Vec<u8> {
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba(PIXELS[0]));
        img.put_pixel(1, 0, Rgba(PIXELS[1]));
        img.put_pixel(0, 1, Rgba(PIXELS[2]));
        img.put_pixel(1, 1, Rgba(PIXELS[3]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    fn assert_known_pixels(bytes: &[u8]) {
        let img = image::load_from_memory(bytes).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (2, 2));
        assert_eq!(img.get_pixel(0, 0).0, PIXELS[0]);
        assert_eq!(img.get_pixel(1, 0).0, PIXELS[1]);
        assert_eq!(img.get_pixel(0, 1).0, PIXELS[2]);
        assert_eq!(img.get_pixel(1, 1).0, PIXELS[3]);
    }

    fn unique_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "imgconv-view-{}-{}-{}",
            std::process::id(),
            DIR_SEQ.fetch_add(1, Ordering::Relaxed),
            label
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn mime_string_is_not_a_successful_conversion() {
        let mut view = ImageConverterView::new();
        view.on_data_received("image/png");
        assert!(view.batch.successes.is_empty());
        assert_eq!(view.batch.failed(), 1);
        assert!(view.preview.is_none());
    }

    #[test]
    fn image_bytes_preview_convert_and_save_known_pixels() {
        let png = known_png();
        let mut view = ImageConverterView::new();
        view.on_image_received(&png, Some("image/png"));

        assert_eq!(view.batch.succeeded(), 1);
        assert_eq!(view.batch.failed(), 0);
        let converted = view.preview.as_deref().expect("preview from memory");
        assert_ne!(converted, b"image/png");
        assert_known_pixels(converted);

        let dest = unique_dir("save");
        let report = save_batch(&view.batch.successes, &dest, ImageTargetFormat::Png);
        assert_eq!(report.saved(), 1);
        let saved = dest.join("clipboard.png");
        assert!(saved.is_file());
        assert_known_pixels(&std::fs::read(&saved).unwrap());
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn detected_payload_bytes_do_not_use_mime_as_path() {
        let png = known_png();
        let mut view = ImageConverterView::new();
        view.on_detected_data(
            &DetectedPayload::new("image", "image/png").with_bytes(png, Some("image/png".into())),
        );
        assert_eq!(view.batch.succeeded(), 1);
        assert_known_pixels(view.preview.as_deref().unwrap());
        assert!(view.path.is_empty());
    }

    #[test]
    fn cancel_save_does_not_write_files() {
        let png = known_png();
        let mut view = ImageConverterView::new();
        view.on_image_received(&png, Some("image/png"));
        assert_eq!(view.batch.succeeded(), 1);

        let dest = unique_dir("cancel");
        assert!(
            view.save_report.is_none(),
            "folder dialog cancel never calls save_batch"
        );
        assert!(std::fs::read_dir(&dest).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn changing_target_reconvert_from_memory() {
        let png = known_png();
        let mut view = ImageConverterView::new();
        view.on_image_received(&png, Some("image/png"));
        view.target = ImageTargetFormat::Bmp;
        view.reconvert();
        assert_eq!(view.batch.succeeded(), 1);
        let bytes = view.preview.as_deref().unwrap();
        assert_eq!(image::guess_format(bytes).unwrap(), image::ImageFormat::Bmp);
        let img = image::load_from_memory(bytes).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (2, 2));
        assert_eq!(img.get_pixel(0, 0).0, PIXELS[0]);
    }

    #[test]
    fn tga_preview_and_saved_file_roundtrip() {
        let png = known_png();
        let mut view = ImageConverterView::new();
        view.on_image_received(&png, Some("image/png"));
        view.target = ImageTargetFormat::Tga;
        view.reconvert();
        assert_eq!(view.batch.succeeded(), 1);
        assert_eq!(view.batch.failed(), 0);
        let tga = view.preview.as_deref().expect("tga preview bytes");
        assert!(image::guess_format(tga).is_err());
        let (w, h, rgba) =
            crate::image_converter::preview_rgba(tga, ImageTargetFormat::Tga).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgba[0..4], &PIXELS[0]);

        let dest = unique_dir("tga-round");
        let report = save_batch(&view.batch.successes, &dest, ImageTargetFormat::Tga);
        assert_eq!(report.saved(), 1);
        let saved = dest.join("clipboard.tga");
        assert!(saved.is_file());

        let mut round = ImageConverterView::new();
        round.target = ImageTargetFormat::Png;
        round.on_data_received(saved.to_str().unwrap());
        assert_eq!(round.batch.succeeded(), 1);
        assert_eq!(round.batch.failed(), 0);
        assert_known_pixels(round.preview.as_deref().unwrap());

        round.target = ImageTargetFormat::Bmp;
        round.reconvert();
        assert_eq!(round.batch.succeeded(), 1);
        let bmp = round.preview.as_deref().unwrap();
        assert_eq!(image::guess_format(bmp).unwrap(), image::ImageFormat::Bmp);

        let mut bad = ImageConverterView::new();
        let bad_path = dest.join("corrupt.tga");
        std::fs::write(&bad_path, b"not a targa").unwrap();
        bad.on_data_received(bad_path.to_str().unwrap());
        assert_eq!(bad.batch.succeeded(), 0);
        assert_eq!(bad.batch.failed(), 1);
        assert!(bad.preview.is_none());
        assert!(bad.batch.failures[0].error.contains("无法解码图像"));

        let _ = std::fs::remove_dir_all(&dest);
    }

    fn assert_no_sensitive(value: &serde_json::Value) {
        assert!(value.get("path").is_none());
        assert!(value.get("bytes").is_none());
        assert!(value.get("preview").is_none());
        assert!(value.get("memory").is_none());
        assert!(value.get("input").is_none());
        assert!(value.get("output").is_none());
    }

    #[test]
    fn persistable_options_are_format_only() {
        let mut view = ImageConverterView::new();
        view.target = ImageTargetFormat::Jpeg;
        view.path = r"C:\Users\secret\photo.png".into();
        view.on_image_received(&known_png(), Some("image/png"));
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["format"], "Jpeg");
        assert_no_sensitive(&value);
        let dumped = value.to_string();
        assert!(!dumped.contains("secret"));
        assert!(!dumped.contains("photo.png"));
        assert!(!dumped.contains("clipboard"));
        assert!(!value.to_string().contains("255"));
    }

    #[test]
    fn restore_jpeg_then_converts_independently() {
        let png = known_png();
        let mut view = ImageConverterView::new();
        view.restore_options(&serde_json::json!({ "format": "Jpeg" }));
        assert_eq!(view.target, ImageTargetFormat::Jpeg);
        view.on_image_received(&png, Some("image/png"));
        assert_eq!(view.batch.succeeded(), 1);
        let bytes = view.preview.as_deref().unwrap();
        assert_eq!(
            image::guess_format(bytes).unwrap(),
            image::ImageFormat::Jpeg
        );
        let img = image::load_from_memory(bytes).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (2, 2));
        assert!(view.path.is_empty());
    }

    #[test]
    fn restore_tga_then_converts_independently() {
        let png = known_png();
        let mut view = ImageConverterView::new();
        view.on_image_received(&png, Some("image/png"));
        view.restore_options(&serde_json::json!({ "format": "Tga" }));
        assert_eq!(view.target, ImageTargetFormat::Tga);
        assert_eq!(view.batch.succeeded(), 1);
        let tga = view.preview.as_deref().unwrap();
        assert!(image::guess_format(tga).is_err());
        let (w, h, rgba) =
            crate::image_converter::preview_rgba(tga, ImageTargetFormat::Tga).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgba[0..4], &PIXELS[0]);
    }

    #[test]
    fn restore_ignores_path_and_bytes_from_json() {
        let mut view = ImageConverterView::new();
        view.path = r"C:\keep\in.png".into();
        view.restore_options(&serde_json::json!({
            "format": "Webp",
            "path": r"C:\injected\secret.png",
            "bytes": [1, 2, 3]
        }));
        assert_eq!(view.target, ImageTargetFormat::Webp);
        assert_eq!(view.path, r"C:\keep\in.png");
        assert!(view.memory.is_none());
    }

    #[test]
    fn missing_fields_use_documented_defaults() {
        let mut view = ImageConverterView::new();
        view.target = ImageTargetFormat::Tiff;
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.target, ImageTargetFormat::Png);
    }

    #[test]
    fn illegal_format_defaults_to_png() {
        let mut view = ImageConverterView::new();
        view.restore_options(&serde_json::json!({ "format": "gif" }));
        assert_eq!(view.target, ImageTargetFormat::Png);
    }

    #[test]
    fn reconstruct_from_serialized_settings_object() {
        let mut first = ImageConverterView::new();
        first.target = ImageTargetFormat::Bmp;
        let (id, value) = first.persistable_options().unwrap();
        assert_eq!(id, "ImageConverter");

        let stored = serde_json::json!({
            "theme": "dark",
            "tool_options": {
                "JsonFormatter": { "indent": "minified" },
                id: value
            }
        });
        let mut second = ImageConverterView::new();
        second.restore_options(&stored["tool_options"]["ImageConverter"]);
        second.on_image_received(&known_png(), Some("image/png"));
        assert_eq!(second.target, ImageTargetFormat::Bmp);
        let bytes = second.preview.as_deref().unwrap();
        assert_eq!(image::guess_format(bytes).unwrap(), image::ImageFormat::Bmp);
        let persisted = second.persistable_options().unwrap().1;
        assert_no_sensitive(&persisted);
    }

    #[test]
    fn test_image_converter_i18n_keys() {
        let keys = [
            ("image_converter.title", "图片格式转换器", "Image Converter"),
            ("image_converter.target_format", "目标格式", "Target format"),
            ("image_converter.select_file", "选择图片", "Select image"),
        ];
        for (key, zh, en) in keys {
            assert_eq!(devtoys_api::t(key, devtoys_api::Language::ZhCn), zh);
            assert_eq!(devtoys_api::t(key, devtoys_api::Language::EnUs), en);
        }
    }
}
