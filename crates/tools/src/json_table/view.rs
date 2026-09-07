use crate::slot::ToolView;
use crate::ui;

use super::{json_to_table, TableFormat};

pub struct JsonTableView {
    input: String,
    output: String,
    format: TableFormat,
    error: Option<String>,
}

impl JsonTableView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            format: TableFormat::Csv,
            error: None,
        }
    }

    fn reconvert(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match json_to_table(&self.input, self.format) {
            Ok(table) => {
                self.error = None;
                self.output = table;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for JsonTableView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("CSV", TableFormat::Csv),
                ("TSV", TableFormat::Tsv),
                ("分号", TableFormat::Fsv),
            ] {
                if ui::toggle(ui, self.format == value, label).clicked() {
                    self.format = value;
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
                    "json-table-in",
                    &mut self.input,
                    "粘贴 JSON 对象数组",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    "输出",
                    "json-table-out",
                    &mut self.output,
                    "表格",
                    false,
                );
            },
        );
        if input_changed {
            self.reconvert();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = payload.to_string();
        self.reconvert();
    }
}
