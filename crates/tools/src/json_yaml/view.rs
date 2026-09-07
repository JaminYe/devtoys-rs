use crate::indent::Indentation;
use crate::slot::ToolView;
use crate::ui;

use super::{convert_json_yaml, looks_like_json, looks_like_yaml, Conversion};

pub struct JsonYamlView {
    input: String,
    output: String,
    direction: Conversion,
    indent: Indentation,
    error: Option<String>,
}

impl JsonYamlView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            direction: Conversion::JsonToYaml,
            indent: Indentation::TwoSpaces,
            error: None,
        }
    }

    fn reconvert(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match convert_json_yaml(&self.input, self.direction, self.indent) {
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

impl ToolView for JsonYamlView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("JSON → YAML", Conversion::JsonToYaml),
                ("YAML → JSON", Conversion::YamlToJson),
            ] {
                if ui::toggle(ui, self.direction == value, label).clicked() {
                    self.direction = value;
                    self.reconvert();
                }
            }
            for (label, value) in [
                ("两空格", Indentation::TwoSpaces),
                ("四空格", Indentation::FourSpaces),
                ("Tab", Indentation::OneTab),
                ("压缩", Indentation::Minified),
            ] {
                if ui::toggle(ui, self.indent == value, label).clicked() {
                    self.indent = value;
                    self.reconvert();
                }
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
                    "json-yaml-in",
                    &mut self.input,
                    "粘贴 JSON 或 YAML",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    "输出",
                    "json-yaml-out",
                    &mut self.output,
                    "转换结果",
                    false,
                );
            },
        );
        if input_changed {
            self.reconvert();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        if looks_like_json(payload) {
            self.direction = Conversion::JsonToYaml;
        } else if looks_like_yaml(payload) {
            self.direction = Conversion::YamlToJson;
        }
        self.input = payload.to_string();
        self.reconvert();
    }
}
