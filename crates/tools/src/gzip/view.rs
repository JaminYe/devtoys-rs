use crate::slot::ToolView;
use crate::ui;

use super::{convert, GzipMode};

pub struct GzipView {
    input: String,
    output: String,
    mode: GzipMode,
    error: Option<String>,
}

impl GzipView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            mode: GzipMode::Compress,
            error: None,
        }
    }

    fn reconvert(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match convert(&self.input, self.mode) {
            Ok(result) => {
                self.error = None;
                self.output = result;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for GzipView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("压缩", GzipMode::Compress), ("解压", GzipMode::Decompress)]
            {
                if ui::toggle(ui, self.mode == value, label).clicked() {
                    self.mode = value;
                    self.reconvert();
                }
            }
            if ui::primary_button(ui, "复制").clicked() && self.error.is_none() {
                ui::copy_text(ui, &self.output);
            }
        });
        ui::error_label(ui, self.error.as_deref());
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed = ui::labeled_code(
                    ui,
                    "输入",
                    "gzip-in",
                    &mut self.input,
                    "粘贴文本或 Base64 GZip",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(ui, "输出", "gzip-out", &mut self.output, "转换结果", false);
            },
        );
        if input_changed {
            self.reconvert();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.mode = GzipMode::Decompress;
        self.input = payload.to_string();
        self.reconvert();
    }
}
