use crate::slot::ToolView;
use crate::ui;

use super::{format_xml, Indentation, ID};

/// Settings JSON `indent`: `two_spaces` | `four_spaces` | `one_tab` | `minified`.
/// Unknown / missing → TwoSpaces. `new_line_on_attributes` missing → false.
const DEFAULT_INDENT: Indentation = Indentation::TwoSpaces;
const DEFAULT_NEW_LINE_ON_ATTRIBUTES: bool = false;

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
                .checkbox(
                    &mut self.new_line_on_attributes,
                    ui::t(ui, "xml.attributes_on_new_lines"),
                )
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
                input_changed = ui::labeled_code_editor(
                    ui,
                    ui::t(ui, "common.input"),
                    "xml-in",
                    &mut self.input,
                    "粘贴 XML",
                    true,
                    Some("xml"),
                );
            },
            |ui| {
                ui::labeled_code_editor(
                    ui,
                    ui::t(ui, "common.output"),
                    "xml-out",
                    &mut self.output,
                    "格式化结果",
                    false,
                    Some("xml"),
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
                "new_line_on_attributes": self.new_line_on_attributes,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.indent = indent_from_settings(value.get("indent").and_then(|v| v.as_str()));
        self.new_line_on_attributes = value
            .get("new_line_on_attributes")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_NEW_LINE_ON_ATTRIBUTES);
        self.reformat();
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
    fn persistable_options_are_indent_and_attr_wrap_only() {
        let mut view = XmlFormatterView::new();
        view.indent = Indentation::FourSpaces;
        view.new_line_on_attributes = true;
        view.input = r#"<root a="1"/>"#.into();
        view.output = "<root/>".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["indent"], "four_spaces");
        assert_eq!(value["new_line_on_attributes"], true);
        assert!(value.get("input").is_none());
        assert!(!value.to_string().contains("<root"));
    }

    #[test]
    fn restore_attr_wrap_then_formats() {
        let mut view = XmlFormatterView::new();
        view.restore_options(&serde_json::json!({
            "indent": "two_spaces",
            "new_line_on_attributes": true
        }));
        assert!(view.new_line_on_attributes);
        view.on_data_received(r#"<root a="1" b="2"/>"#);
        assert_eq!(view.output, "<root\n  a=\"1\"\n  b=\"2\"/>");
    }

    #[test]
    fn missing_and_illegal_indent_use_defaults() {
        let mut view = XmlFormatterView::new();
        view.indent = Indentation::Minified;
        view.new_line_on_attributes = true;
        view.restore_options(&serde_json::json!({ "indent": "eight_spaces" }));
        assert_eq!(view.indent, Indentation::TwoSpaces);
        assert!(!view.new_line_on_attributes);
        view.on_data_received("<root><a>1</a></root>");
        assert_eq!(view.output, "<root>\n  <a>1</a>\n</root>");
    }
}
