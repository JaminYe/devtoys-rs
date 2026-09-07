use std::path::Path;

use crate::slot::ToolView;
use crate::ui;

use super::{decode_image_path, encode_svg, is_existing_image_file};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Conversion {
    #[default]
    Encode,
    Decode,
}

pub struct QrcodeView {
    input: String,
    output: String,
    conversion: Conversion,
    error: Option<String>,
}

impl QrcodeView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            conversion: Conversion::Encode,
            error: None,
        }
    }

    fn recompute(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        let result = match self.conversion {
            Conversion::Encode => encode_svg(&self.input),
            Conversion::Decode => decode_image_path(Path::new(self.input.trim())),
        };
        match result {
            Ok(text) => {
                self.error = None;
                self.output = text;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for QrcodeView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("编码", Conversion::Encode), ("解码", Conversion::Decode)] {
                if ui::toggle(ui, self.conversion == value, label).clicked() {
                    self.conversion = value;
                    self.recompute();
                }
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui::error_label(ui, self.error.as_deref());
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed =
                    ui::labeled_code(ui, "输入", "qr-in", &mut self.input, "文本或图像路径", true);
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    "输出",
                    "qr-out",
                    &mut self.output,
                    "SVG 或解码文本",
                    false,
                );
            },
        );
        if input_changed {
            self.recompute();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.conversion = if is_existing_image_file(payload) {
            Conversion::Decode
        } else {
            Conversion::Encode
        };
        self.input = payload.to_string();
        self.recompute();
    }
}
