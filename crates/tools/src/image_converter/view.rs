use crate::slot::ToolView;
use crate::ui;

use super::{convert_image, ImageTargetFormat};

pub struct ImageConverterView {
    path: String,
    /// Last chosen target; in-memory only (no global settings).
    target: ImageTargetFormat,
    status: Option<String>,
    error: Option<String>,
    preview: Option<Vec<u8>>,
    tex: Option<egui::TextureHandle>,
}

impl ImageConverterView {
    pub fn new() -> Self {
        Self {
            path: String::new(),
            target: ImageTargetFormat::Png,
            status: None,
            error: None,
            preview: None,
            tex: None,
        }
    }

    fn reconvert(&mut self) {
        let paths: Vec<&str> = self
            .path
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        if paths.is_empty() {
            self.error = None;
            self.status = None;
            self.preview = None;
            self.tex = None;
            return;
        }

        let n = paths.len();
        let first = paths[0].to_string();
        match std::fs::read(&first) {
            Ok(bytes) => match convert_image(&bytes, self.target) {
                Ok(converted) => {
                    self.error = None;
                    self.preview = Some(converted);
                    self.tex = None;
                    self.status = Some(format!("已转换 {n} 个路径 → {}", self.target.as_str()));
                }
                Err(_) => {
                    self.preview = None;
                    self.tex = None;
                    self.status = None;
                    self.error = Some("无法转换图像".into());
                }
            },
            Err(_) => {
                self.preview = None;
                self.tex = None;
                self.status = None;
                self.error = Some("无法读取图像".into());
            }
        }
    }
}

impl ToolView for ImageConverterView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
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
        ui::error_label(ui, self.error.as_deref());
        if let Some(status) = &self.status {
            ui.label(status);
        }
        if let Some(preview) = &self.preview {
            ui.label(format!("预览已生成（{} 字节）", preview.len()));
        }

        let mut changed = false;
        ui::split_2(
            ui,
            |ui| {
                changed = ui::labeled_code(
                    ui,
                    "选中路径",
                    "img-path",
                    &mut self.path,
                    "图像文件或目录（多行路径）",
                    true,
                );
            },
            |ui| {
                if let Some(bytes) = self.preview.as_deref() {
                    show_png(ui, "imgconv-preview", bytes, &mut self.tex);
                }
            },
        );
        if changed {
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
        self.path = filled;
        self.reconvert();
    }
}

fn show_png(
    ui: &mut egui::Ui,
    name: &'static str,
    bytes: &[u8],
    cache: &mut Option<egui::TextureHandle>,
) {
    if cache.is_none() {
        if let Some(color) = ui::png_image(bytes) {
            *cache = Some(ui.ctx().load_texture(name, color, Default::default()));
        }
    }
    if let Some(tex) = cache.as_ref() {
        ui.add(egui::Image::new(tex).max_size(ui.available_size()));
    }
}
