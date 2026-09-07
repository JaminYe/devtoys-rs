use std::fs;
use std::path::Path;

use crate::slot::ToolView;
use crate::ui;

use super::{decode_base64, encode_bytes, inspect_image};

pub struct Base64ImageView {
    input: String,
    preview: Option<String>,
    decoded: Option<Vec<u8>>,
    tex: Option<egui::TextureHandle>,
    error: Option<String>,
}

impl Base64ImageView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            preview: None,
            decoded: None,
            tex: None,
            error: None,
        }
    }

    fn refresh(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.preview = None;
            self.decoded = None;
            self.tex = None;
            return;
        }
        match decode_base64(&self.input) {
            Ok(bytes) => {
                self.error = None;
                self.preview = Some(inspect_image(&bytes).summary());
                self.decoded = Some(bytes);
                self.tex = None;
            }
            Err(err) => {
                self.preview = None;
                self.decoded = None;
                self.tex = None;
                self.error = Some(err.to_string());
            }
        }
    }
}

impl ToolView for Base64ImageView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui::primary_button(ui, "复制").clicked() {
                ui::copy_text(ui, &self.input);
            }
        });
        ui::error_label(ui, self.error.as_deref());
        if let Some(summary) = &self.preview {
            ui.label(format!("预览：{summary}"));
        }

        let mut changed = false;
        ui::split_2(
            ui,
            |ui| {
                changed = ui::labeled_code(
                    ui,
                    "Base64",
                    "b64-in",
                    &mut self.input,
                    "粘贴 Base64 或 data URI",
                    true,
                );
            },
            |ui| {
                if let Some(bytes) = self.decoded.as_deref() {
                    show_png(ui, "b64-preview", bytes, &mut self.tex);
                }
            },
        );
        if changed {
            self.refresh();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = if Path::new(payload).is_file() {
            match fs::read(payload) {
                Ok(bytes) => encode_bytes(&bytes),
                Err(_) => payload.to_string(),
            }
        } else {
            payload.to_string()
        };
        self.refresh();
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
