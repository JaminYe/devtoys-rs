use crate::slot::ToolView;
use crate::ui;

use super::{format_sql, Indentation, SqlLanguage, ID};

/// Settings JSON `indent`: `two_spaces` | `four_spaces` | `one_tab` | `minified`.
/// Unknown / missing → TwoSpaces. `leading_comma` missing → false.
const DEFAULT_INDENT: Indentation = Indentation::TwoSpaces;
const DEFAULT_LEADING_COMMA: bool = false;

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
                (ui::t("common.two_spaces"), Indentation::TwoSpaces),
                (ui::t("common.four_spaces"), Indentation::FourSpaces),
                (ui::t("common.one_tab"), Indentation::OneTab),
                (ui::t("common.minified"), Indentation::Minified),
            ] {
                if ui::toggle(ui, self.indent == value, label).clicked() {
                    self.indent = value;
                    self.reformat();
                }
            }
            if ui
                .checkbox(&mut self.leading_comma, ui::t("sql.leading_comma"))
                .changed()
            {
                self.reformat();
            }
            let mut lang_changed = false;
            ui.label(ui::t("sql.language"));
            egui::ComboBox::from_id_salt("sql-lang")
                .selected_text(self.language.display_name())
                .show_ui(ui, |ui| {
                    for lang in SqlLanguage::ALL {
                        if ui
                            .selectable_label(self.language == lang, lang.display_name())
                            .clicked()
                        {
                            self.language = lang;
                            lang_changed = true;
                        }
                    }
                });
            if lang_changed {
                self.reformat();
            }
            ui::copy_button(ui, Some(self.output.as_str()));
        });
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed = ui::labeled_code_editor(
                    ui,
                    ui::t("common.input"),
                    "sql-in",
                    &mut self.input,
                    "粘贴 SQL",
                    true,
                    Some("sql"),
                );
            },
            |ui| {
                ui::labeled_code_editor(
                    ui,
                    ui::t("common.output"),
                    "sql-out",
                    &mut self.output,
                    "格式化结果",
                    false,
                    Some("sql"),
                );
            },
        );
        if input_changed {
            self.reformat();
        }
    }

    fn on_data_received(&mut self, _payload: &str) {}

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "indent": indent_to_settings(self.indent),
                "language": self.language.as_str(),
                "leading_comma": self.leading_comma,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.indent = indent_from_settings(value.get("indent").and_then(|v| v.as_str()));
        self.language = value
            .get("language")
            .and_then(|v| v.as_str())
            .and_then(SqlLanguage::parse)
            .unwrap_or(SqlLanguage::Sql);
        self.leading_comma = value
            .get("leading_comma")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_LEADING_COMMA);
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
    fn persistable_options_are_indent_language_and_leading_comma() {
        let mut view = SqlFormatterView::new();
        view.indent = Indentation::FourSpaces;
        view.language = SqlLanguage::Tsql;
        view.leading_comma = true;
        view.input = "select a, b from t".into();
        view.output = "SELECT".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["indent"], "four_spaces");
        assert_eq!(value["language"], "Tsql");
        assert_eq!(value["leading_comma"], true);
        assert!(value.get("input").is_none());
        assert!(!value.to_string().contains("select"));
    }

    #[test]
    fn restore_four_spaces_leading_comma_then_formats() {
        let mut view = SqlFormatterView::new();
        view.restore_options(&serde_json::json!({
            "indent": "four_spaces",
            "language": "PostgreSql",
            "leading_comma": true
        }));
        assert_eq!(view.indent, Indentation::FourSpaces);
        assert_eq!(view.language, SqlLanguage::PostgreSql);
        assert!(view.leading_comma);
        view.input = "select a, b, c from t".into();
        view.reformat();
        let lines: Vec<_> = view.output.lines().map(str::trim).collect();
        assert!(lines.iter().any(|l| *l == ", b"), "{}", view.output);
        assert!(lines.iter().any(|l| *l == ", c"), "{}", view.output);
        assert!(view.output.contains("    "), "{}", view.output);
    }

    #[test]
    fn missing_and_illegal_indent_use_defaults() {
        let mut view = SqlFormatterView::new();
        view.indent = Indentation::Minified;
        view.leading_comma = true;
        view.restore_options(&serde_json::json!({ "indent": "eight_spaces" }));
        assert_eq!(view.indent, Indentation::TwoSpaces);
        assert!(!view.leading_comma);
        view.input = "select a, b from t".into();
        view.reformat();
        let lines: Vec<_> = view.output.lines().map(str::trim).collect();
        assert!(lines.iter().any(|l| *l == "a,"), "{}", view.output);
    }
}
