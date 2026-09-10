use crate::slot::ToolView;
use crate::ui;

use super::{json_to_table, TableFormat, ID};

/// Settings JSON `format` values: `csv` | `tsv` | `fsv`. Unknown / missing → [`TableFormat::Csv`].
const DEFAULT_FORMAT: TableFormat = TableFormat::Csv;

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
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui::error_label(ui, self.error.as_deref());
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed = ui::labeled_code(
                    ui,
                    ui::t(ui, "common.input"),
                    "json-table-in",
                    &mut self.input,
                    "粘贴 JSON 对象数组",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    ui::t(ui, "common.output"),
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

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "format": format_to_settings(self.format),
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.format = format_from_settings(value.get("format").and_then(|v| v.as_str()));
        self.reconvert();
    }
}

fn format_to_settings(format: TableFormat) -> &'static str {
    match format {
        TableFormat::Csv => "csv",
        TableFormat::Tsv => "tsv",
        TableFormat::Fsv => "fsv",
    }
}

fn format_from_settings(value: Option<&str>) -> TableFormat {
    match value {
        Some("csv") => TableFormat::Csv,
        Some("tsv") => TableFormat::Tsv,
        Some("fsv") => TableFormat::Fsv,
        _ => DEFAULT_FORMAT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    const SAMPLE: &str = r#"[{"name":"Ada","id":1},{"name":"Bob","id":2}]"#;

    #[test]
    fn persistable_options_are_format_only() {
        let mut view = JsonTableView::new();
        view.format = TableFormat::Tsv;
        view.on_data_received(SAMPLE);
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["format"], "tsv");
        assert!(value.get("input").is_none());
        assert!(value.get("output").is_none());
        assert!(!value.to_string().contains("Ada"));
        assert_eq!(view.input, SAMPLE);
    }

    #[test]
    fn restore_fsv_then_converts_independently() {
        let mut view = JsonTableView::new();
        view.restore_options(&serde_json::json!({ "format": "fsv" }));
        assert_eq!(view.format, TableFormat::Fsv);
        view.on_data_received(SAMPLE);
        assert_eq!(view.output, "name;id\nAda;1\nBob;2");
    }

    #[test]
    fn missing_and_illegal_format_use_csv() {
        let mut view = JsonTableView::new();
        view.format = TableFormat::Tsv;
        view.restore_options(&serde_json::json!({ "format": "xlsx" }));
        assert_eq!(view.format, TableFormat::Csv);
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.format, TableFormat::Csv);
        view.on_data_received(SAMPLE);
        assert_eq!(view.output, "name,id\nAda,1\nBob,2");
    }
}
