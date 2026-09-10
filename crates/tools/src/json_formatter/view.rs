use crate::slot::ToolView;
use crate::ui;

use super::{format_json, Indentation, ID};

/// Settings JSON `indent` values: `two_spaces` | `four_spaces` | `one_tab` | `minified`.
/// Unknown / missing → [`Indentation::TwoSpaces`]. `sort_properties` missing → false.
const DEFAULT_INDENT: Indentation = Indentation::TwoSpaces;
const DEFAULT_SORT_PROPERTIES: bool = false;

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
            indent: DEFAULT_INDENT,
            sort_properties: DEFAULT_SORT_PROPERTIES,
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

fn indent_to_settings(indent: Indentation) -> &'static str {
    match indent {
        Indentation::TwoSpaces => "two_spaces",
        Indentation::FourSpaces => "four_spaces",
        Indentation::OneTab => "one_tab",
        Indentation::Minified => "minified",
    }
}

fn indent_from_settings(value: Option<&str>) -> Indentation {
    match value {
        Some("two_spaces") => Indentation::TwoSpaces,
        Some("four_spaces") => Indentation::FourSpaces,
        Some("one_tab") => Indentation::OneTab,
        Some("minified") => Indentation::Minified,
        _ => DEFAULT_INDENT,
    }
}

impl ToolView for JsonFormatterView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                (ui::t(ui, "common.two_spaces"), Indentation::TwoSpaces),
                (ui::t(ui, "common.four_spaces"), Indentation::FourSpaces),
                (ui::t(ui, "common.one_tab"), Indentation::OneTab),
                (ui::t(ui, "common.minified"), Indentation::Minified),
            ] {
                if ui::toggle(ui, self.indent == value, label).clicked() {
                    self.indent = value;
                    self.reformat();
                }
            }
            if ui
                .checkbox(&mut self.sort_properties, ui::t(ui, "json.sort_properties"))
                .changed()
            {
                self.reformat();
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        let err_text = self.error.as_deref().map(|e| {
            if e == "非法 JSON" {
                ui::t(ui, "json.invalid")
            } else {
                e
            }
        });
        ui::error_label(ui, err_text);
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed = ui::labeled_code_editor(
                    ui,
                    ui::t(ui, "common.input"),
                    "json-in",
                    &mut self.input,
                    "粘贴 JSON",
                    true,
                    Some("json"),
                );
            },
            |ui| {
                ui::labeled_code_editor(
                    ui,
                    ui::t(ui, "common.output"),
                    "json-out",
                    &mut self.output,
                    "格式化结果",
                    false,
                    Some("json"),
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

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "indent": indent_to_settings(self.indent),
                "sort_properties": self.sort_properties,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.indent = indent_from_settings(value.get("indent").and_then(|v| v.as_str()));
        self.sort_properties = value
            .get("sort_properties")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_SORT_PROPERTIES);
        self.reformat();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    const SAMPLE: &str = r#"{"z":1,"a":2}"#;

    #[test]
    fn persistable_options_are_indent_and_sort_only() {
        let mut view = JsonFormatterView::new();
        view.indent = Indentation::FourSpaces;
        view.sort_properties = true;
        view.on_data_received(SAMPLE);
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["indent"], "four_spaces");
        assert_eq!(value["sort_properties"], true);
        assert!(value.get("input").is_none());
        assert!(value.get("output").is_none());
        let dumped = value.to_string();
        assert!(!dumped.contains("secret") && !dumped.contains(SAMPLE));
        assert_eq!(view.input, SAMPLE);
    }

    #[test]
    fn restore_four_spaces_sorted_then_formats_independently() {
        let mut view = JsonFormatterView::new();
        view.restore_options(&serde_json::json!({
            "indent": "four_spaces",
            "sort_properties": true
        }));
        view.on_data_received(SAMPLE);
        assert_eq!(view.indent, Indentation::FourSpaces);
        assert!(view.sort_properties);
        assert_eq!(view.output, "{\n    \"a\": 2,\n    \"z\": 1\n}");
    }

    #[test]
    fn restore_minified_sorted_then_formats_independently() {
        let mut view = JsonFormatterView::new();
        view.restore_options(&serde_json::json!({
            "indent": "minified",
            "sort_properties": true
        }));
        view.on_data_received(SAMPLE);
        assert_eq!(view.output, r#"{"a":2,"z":1}"#);
    }

    #[test]
    fn restore_one_tab_unsorted_then_formats_independently() {
        let mut view = JsonFormatterView::new();
        view.restore_options(&serde_json::json!({
            "indent": "one_tab",
            "sort_properties": false
        }));
        view.on_data_received(SAMPLE);
        assert_eq!(view.output, "{\n\t\"z\": 1,\n\t\"a\": 2\n}");
    }

    #[test]
    fn missing_fields_use_documented_defaults() {
        let mut view = JsonFormatterView::new();
        view.indent = Indentation::Minified;
        view.sort_properties = true;
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.indent, Indentation::TwoSpaces);
        assert!(!view.sort_properties);
        view.on_data_received(SAMPLE);
        assert_eq!(view.output, "{\n  \"z\": 1,\n  \"a\": 2\n}");
    }

    #[test]
    fn illegal_indent_defaults_but_keeps_valid_sort() {
        let mut view = JsonFormatterView::new();
        view.restore_options(&serde_json::json!({
            "indent": "eight_spaces",
            "sort_properties": true
        }));
        assert_eq!(view.indent, Indentation::TwoSpaces);
        assert!(view.sort_properties);
        view.on_data_received(SAMPLE);
        assert_eq!(view.output, "{\n  \"a\": 2,\n  \"z\": 1\n}");
    }

    #[test]
    fn reconstruct_from_serialized_settings_object() {
        let mut first = JsonFormatterView::new();
        first.indent = Indentation::FourSpaces;
        first.sort_properties = true;
        let (id, value) = first.persistable_options().unwrap();
        assert_eq!(id, "JsonFormatter");

        let stored = serde_json::json!({
            "theme": "dark",
            "tool_options": { id: value }
        });
        let mut second = JsonFormatterView::new();
        second.restore_options(&stored["tool_options"]["JsonFormatter"]);
        second.on_data_received(r#"{"z":{"b":1,"a":2},"a":0}"#);
        assert_eq!(
            second.output,
            "{\n    \"a\": 0,\n    \"z\": {\n        \"a\": 2,\n        \"b\": 1\n    }\n}"
        );
        assert_eq!(second.input, r#"{"z":{"b":1,"a":2},"a":0}"#);
        let persisted = second.persistable_options().unwrap().1;
        assert!(persisted.get("input").is_none());
        assert!(!persisted.to_string().contains("\"z\""));
    }

    #[test]
    fn test_json_formatter_view_ui_localization() {
        let mut view = JsonFormatterView::new();
        view.input = "{invalid json}".into();
        view.reformat();
        assert!(view.error.is_some());

        // ZhCn UI
        let ctx_zh = egui::Context::default();
        let mut out_zh = ctx_zh.run_ui(egui::RawInput::default(), |ui| {
            view.ui(ui);
        });
        out_zh.textures_delta.clear();
        let zh_texts: Vec<String> = out_zh.shapes.iter().filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        }).collect();
        assert!(zh_texts.iter().any(|t| t == "非法 JSON"), "ZhCn should contain '非法 JSON'");

        // EnUs UI
        let ctx_en = egui::Context::default();
        ctx_en.data_mut(|d| {
            d.insert_temp(egui::Id::new("app_language"), devtoys_api::Language::EnUs);
        });
        let mut out_en = ctx_en.run_ui(egui::RawInput::default(), |ui| {
            view.ui(ui);
        });
        out_en.textures_delta.clear();
        let en_texts: Vec<String> = out_en.shapes.iter().filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        }).collect();
        assert!(en_texts.iter().any(|t| t == "Invalid JSON"), "EnUs should contain 'Invalid JSON'");
    }
}
