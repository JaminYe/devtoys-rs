use crate::slot::ToolView;
use crate::ui;

use super::{convert, Conversion};

pub struct HtmlView {
    input: String,
    output: String,
    conversion: Conversion,
    error: Option<String>,
}

impl HtmlView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            conversion: Conversion::Encode,
            error: None,
        }
    }

    fn recompute(&mut self) {
        if self.input.is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match convert(&self.input, self.conversion) {
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

impl ToolView for HtmlView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("编码", Conversion::Encode), ("解码", Conversion::Decode)] {
                if ui::toggle(ui, self.conversion == value, label).clicked() {
                    self.conversion = value;
                    self.recompute();
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
                input_changed =
                    ui::labeled_code(ui, "输入", "html-in", &mut self.input, "粘贴文本", true);
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    "输出",
                    "html-out",
                    &mut self.output,
                    "编解码结果",
                    false,
                );
            },
        );
        if input_changed {
            self.recompute();
        }
    }

    fn on_data_received(&mut self, _payload: &str) {}
}
