use crate::slot::ToolView;
use crate::ui;

use super::{convert, looks_like_base64, Charset, Conversion};

pub struct Base64TextView {
    input: String,
    output: String,
    conversion: Conversion,
    charset: Charset,
    multiline: bool,
    error: Option<String>,
}

impl Base64TextView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            conversion: Conversion::Encode,
            charset: Charset::Utf8,
            multiline: false,
            error: None,
        }
    }

    fn reconvert(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match convert(&self.input, self.conversion, self.charset, self.multiline) {
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

impl ToolView for Base64TextView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("编码", Conversion::Encode), ("解码", Conversion::Decode)] {
                if ui::toggle(ui, self.conversion == value, label).clicked() {
                    self.conversion = value;
                    self.reconvert();
                }
            }
            for (label, value) in [("UTF-8", Charset::Utf8), ("ASCII", Charset::Ascii)] {
                if ui::toggle(ui, self.charset == value, label).clicked() {
                    self.charset = value;
                    self.reconvert();
                }
            }
            if ui.checkbox(&mut self.multiline, "多行").changed() {
                self.reconvert();
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui::error_label(ui, self.error.as_deref());
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed = ui::labeled_code(
                    ui,
                    "输入",
                    "b64-in",
                    &mut self.input,
                    "粘贴文本或 Base64",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(ui, "输出", "b64-out", &mut self.output, "转换结果", false);
            },
        );
        if input_changed {
            self.reconvert();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.conversion = if looks_like_base64(payload) {
            Conversion::Decode
        } else {
            Conversion::Encode
        };
        self.input = payload.to_string();
        self.reconvert();
    }
}
