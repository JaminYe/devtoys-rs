use crate::slot::ToolView;
use crate::ui;

use super::{format_xml, Indentation};

pub struct XmlFormatterView {
    input: String,
    output: String,
    indent: Indentation,
    new_line_on_attributes: bool,
    error: Option<String>,
}

impl XmlFormatterView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            indent: Indentation::TwoSpaces,
            new_line_on_attributes: false,
            error: None,
        }
    }

    fn reformat(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match format_xml(&self.input, self.indent, self.new_line_on_attributes) {
            Ok(formatted) => {
                self.error = None;
                self.output = formatted;
            }
            Err(_) => {
                self.error = Some("非法 XML".into());
                self.output.clear();
            }
        }
    }
}

impl ToolView for XmlFormatterView {
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
                .checkbox(&mut self.new_line_on_attributes, "属性换行")
                .changed()
            {
                self.reformat();
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui::error_label(ui, self.error.as_deref());
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed =
                    ui::labeled_code(ui, "输入", "xml-in", &mut self.input, "粘贴 XML", true);
            },
            |ui| {
                ui::labeled_code(ui, "输出", "xml-out", &mut self.output, "格式化结果", false);
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
