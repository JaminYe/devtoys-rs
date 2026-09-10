use crate::indent::Indentation;
use crate::slot::ToolView;
use crate::ui;

use super::helper::coerce_json_yaml_indent;
use super::{convert_json_yaml, looks_like_json, looks_like_yaml, Conversion, ID};

/// Settings JSON `direction`: `json_to_yaml` | `yaml_to_json`.
/// `indent`: `two_spaces` | `four_spaces` | `one_tab` | `minified`.
/// Unknown / missing → JsonToYaml + TwoSpaces. JSON→YAML illegal indent is coerced.
const DEFAULT_DIRECTION: Conversion = Conversion::JsonToYaml;
const DEFAULT_INDENT: Indentation = Indentation::TwoSpaces;

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
                (ui::t("json_yaml.json_to_yaml"), Conversion::JsonToYaml),
                (ui::t("json_yaml.yaml_to_json"), Conversion::YamlToJson),
            ] {
                if ui::toggle(ui, self.direction == value, label).clicked() {
                    self.direction = value;
                    self.indent = coerce_json_yaml_indent(self.direction, self.indent);
                    self.reconvert();
                }
            }
            let indent_choices: &[(&str, Indentation)] = match self.direction {
                Conversion::JsonToYaml => &[
                    (ui::t("common.two_spaces"), Indentation::TwoSpaces),
                    (ui::t("common.four_spaces"), Indentation::FourSpaces),
                ],
                Conversion::YamlToJson => &[
                    (ui::t("common.two_spaces"), Indentation::TwoSpaces),
                    (ui::t("common.four_spaces"), Indentation::FourSpaces),
                    (ui::t("common.one_tab"), Indentation::OneTab),
                    (ui::t("common.minified"), Indentation::Minified),
                ],
            };
            for (label, value) in indent_choices {
                if ui::toggle(ui, self.indent == *value, label).clicked() {
                    self.indent = *value;
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
                    ui::t("common.input"),
                    "json-yaml-in",
                    &mut self.input,
                    "粘贴 JSON 或 YAML",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    ui::t("common.output"),
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
        self.indent = coerce_json_yaml_indent(self.direction, self.indent);
        self.input = payload.to_string();
        self.reconvert();
    }

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "direction": direction_to_settings(self.direction),
                "indent": indent_to_settings(self.indent),
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.direction = direction_from_settings(value.get("direction").and_then(|v| v.as_str()));
        self.indent = indent_from_settings(value.get("indent").and_then(|v| v.as_str()));
        self.indent = coerce_json_yaml_indent(self.direction, self.indent);
        self.reconvert();
    }
}

fn direction_to_settings(direction: Conversion) -> &'static str {
    match direction {
        Conversion::JsonToYaml => "json_to_yaml",
        Conversion::YamlToJson => "yaml_to_json",
    }
}

fn direction_from_settings(value: Option<&str>) -> Conversion {
    match value {
        Some("json_to_yaml") => Conversion::JsonToYaml,
        Some("yaml_to_json") => Conversion::YamlToJson,
        _ => DEFAULT_DIRECTION,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_direction_and_indent_only() {
        let mut view = JsonYamlView::new();
        view.direction = Conversion::YamlToJson;
        view.indent = Indentation::Minified;
        view.input = "a: 1".into();
        view.output = r#"{"a":1}"#.into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["direction"], "yaml_to_json");
        assert_eq!(value["indent"], "minified");
        assert!(value.get("input").is_none());
        assert!(value.get("output").is_none());
        assert!(!value.to_string().contains("a: 1"));
    }

    #[test]
    fn restore_yaml_to_json_minified_then_converts() {
        let mut view = JsonYamlView::new();
        view.restore_options(&serde_json::json!({
            "direction": "yaml_to_json",
            "indent": "minified"
        }));
        assert_eq!(view.direction, Conversion::YamlToJson);
        assert_eq!(view.indent, Indentation::Minified);
        view.input = "a:\n  b: 1".into();
        view.reconvert();
        assert_eq!(view.output, r#"{"a":{"b":1}}"#);
    }

    #[test]
    fn restore_json_to_yaml_four_spaces_then_converts() {
        let mut view = JsonYamlView::new();
        view.restore_options(&serde_json::json!({
            "direction": "json_to_yaml",
            "indent": "four_spaces"
        }));
        view.input = r#"{"a":{"b":1}}"#.into();
        view.reconvert();
        assert!(view.output.contains("\n    b:"), "{}", view.output);
        assert!(!view.output.contains('\t'));
    }

    #[test]
    fn json_to_yaml_illegal_indent_coerced_to_two_spaces() {
        let mut view = JsonYamlView::new();
        view.restore_options(&serde_json::json!({
            "direction": "json_to_yaml",
            "indent": "minified"
        }));
        assert_eq!(view.direction, Conversion::JsonToYaml);
        assert_eq!(view.indent, Indentation::TwoSpaces);
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.direction, Conversion::JsonToYaml);
        assert_eq!(view.indent, Indentation::TwoSpaces);
    }
}
