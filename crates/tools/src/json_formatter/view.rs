use crate::slot::ToolView;
use crate::ui;

use super::{format_json, Indentation};

pub struct JsonFormatterView {
    input: String,
    output: String,
    indent: Indentation,
    sort_properties: bool,
    error: Option<String>,
}

impl JsonFormatterView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            indent: Indentation::TwoSpaces,
            sort_properties: false,
            error: None,
        }
    }

    fn reformat(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match format_json(&self.input, self.indent, self.sort_properties) {
            Ok(formatted) => {
                self.error = None;
                self.output = formatted;
            }
            Err(_) => {
                self.error = Some("非法 JSON".into());
                self.output.clear();
            }
        }
    }
}

impl ToolView for JsonFormatterView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("两空格", Indentation::TwoSpaces),
                ("四空格", Indentation::FourSpaces),
                ("Tab", Indentation::OneTab),
                ("压缩", Indentation::Minified),
            ] {
                if ui::toggle(ui, self.indent == value, label).clicked() {
                    self.indent = value;
                    self.reformat();
                }
            }
            if ui
                .checkbox(&mut self.sort_properties, "按属性名排序")
                .changed()
            {
                self.reformat();
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
                    ui::labeled_code(ui, "输入", "json-in", &mut self.input, "粘贴 JSON", true);
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    "输出",
                    "json-out",
                    &mut self.output,
                    "格式化结果",
                    false,
                );
            },
        );
        if input_changed {
            self.reformat();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = payload.to_string();
        self.reformat();
    }
}
