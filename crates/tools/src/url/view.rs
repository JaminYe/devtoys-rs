use crate::slot::ToolView;
use crate::ui;

use super::{convert, Conversion};

pub struct UrlView {
    input: String,
    output: String,
    conversion: Conversion,
    multiline: bool,
    error: Option<String>,
}

impl UrlView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            conversion: Conversion::Encode,
            multiline: false,
            error: None,
        }
    }

    fn recompute(&mut self) {
        if self.input.is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match convert(&self.input, self.conversion, self.multiline) {
            Ok(formatted) => {
                self.error = None;
                self.output = formatted;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for UrlView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("编码", Conversion::Encode), ("解码", Conversion::Decode)] {
                if ui::toggle(ui, self.conversion == value, label).clicked() {
                    self.conversion = value;
                    self.recompute();
                }
            }
            if ui.checkbox(&mut self.multiline, "多行").changed() {
                self.recompute();
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui::error_label(ui, self.error.as_deref());
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed =
                    ui::labeled_code(ui, "输入", "url-in", &mut self.input, "粘贴文本", true);
            },
            |ui| {
                ui::labeled_code(ui, "输出", "url-out", &mut self.output, "编解码结果", false);
            },
        );
        if input_changed {
            self.recompute();
        }
    }

    fn on_data_received(&mut self, _payload: &str) {}
}
