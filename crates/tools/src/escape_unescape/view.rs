use crate::slot::ToolView;
use crate::ui;

use super::{convert, Conversion};

pub struct EscapeUnescapeView {
    input: String,
    output: String,
    conversion: Conversion,
    error: Option<String>,
}

impl EscapeUnescapeView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            conversion: Conversion::Encode,
            error: None,
        }
    }

    fn reconvert(&mut self) {
        match convert(&self.input, self.conversion) {
            Ok(result) => {
                self.error = None;
                self.output = result;
            }
            Err(_) => {
                self.error = Some("非法转义序列".into());
                self.output.clear();
            }
        }
    }
}

impl ToolView for EscapeUnescapeView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("转义", Conversion::Encode), ("反转义", Conversion::Decode)]
            {
                if ui::toggle(ui, self.conversion == value, label).clicked() {
                    self.conversion = value;
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
                input_changed =
                    ui::labeled_code(ui, "输入", "escape-in", &mut self.input, "粘贴文本", true);
            },
            |ui| {
                ui::labeled_code(ui, "输出", "escape-out", &mut self.output, "结果", false);
            },
        );
        if input_changed {
            self.reconvert();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.conversion = Conversion::Decode;
        self.input = payload.to_string();
        self.reconvert();
    }
}
