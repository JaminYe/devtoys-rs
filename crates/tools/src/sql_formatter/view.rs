use crate::slot::ToolView;
use crate::ui;

use super::{format_sql, Indentation, SqlLanguage};

pub struct SqlFormatterView {
    input: String,
    output: String,
    indent: Indentation,
    language: SqlLanguage,
    leading_comma: bool,
}

impl SqlFormatterView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            indent: Indentation::TwoSpaces,
            language: SqlLanguage::Sql,
            leading_comma: false,
        }
    }

    fn reformat(&mut self) {
        if self.input.trim().is_empty() {
            self.output.clear();
            return;
        }
        self.output = format_sql(&self.input, self.indent, self.language, self.leading_comma);
    }
}

impl ToolView for SqlFormatterView {
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
            if ui.checkbox(&mut self.leading_comma, "前导逗号").changed() {
                self.reformat();
            }
            if ui::primary_button(ui, "复制").clicked() {
                ui::copy_text(ui, &self.output);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("方言");
            for lang in SqlLanguage::ALL {
                if ui::toggle(ui, self.language == lang, lang.as_str()).clicked() {
                    self.language = lang;
                    self.reformat();
                }
            }
        });
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed =
                    ui::labeled_code(ui, "输入", "sql-in", &mut self.input, "粘贴 SQL", true);
            },
            |ui| {
                ui::labeled_code(ui, "输出", "sql-out", &mut self.output, "格式化结果", false);
            },
        );
        if input_changed {
            self.reformat();
        }
    }

    fn on_data_received(&mut self, _payload: &str) {}
}
