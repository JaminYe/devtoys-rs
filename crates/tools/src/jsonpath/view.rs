use crate::slot::ToolView;
use crate::ui;

use super::eval_jsonpath;
use super::helper::CHEAT_SHEET;

pub struct JsonPathView {
    json: String,
    path: String,
    output: String,
    error: Option<String>,
}

impl JsonPathView {
    pub fn new() -> Self {
        Self {
            json: String::new(),
            path: String::new(),
            output: String::new(),
            error: None,
        }
    }

    fn reeval(&mut self) {
        if self.json.trim().is_empty() || self.path.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match eval_jsonpath(&self.json, &self.path) {
            Ok(formatted) => {
                self.error = None;
                self.output = formatted;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for JsonPathView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui::error_label(ui, self.error.as_deref());
        let avail = ui.available_size();
        let bottom = 72.0;
        let mut json_changed = false;
        let mut path_changed = false;
        ui.allocate_ui(egui::vec2(avail.x, (avail.y - bottom).max(80.0)), |ui| {
            ui::split_2(
                ui,
                |ui| {
                    json_changed = ui::labeled_code(
                        ui,
                        ui::t(ui, "common.input"),
                        "jsonpath-json",
                        &mut self.json,
                        ui::t(ui, "common.paste"),
                        true,
                    );
                },
                |ui| {
                    ui.vertical(|ui| {
                        ui.label(ui::t(ui, "jsonpath.expression"));
                        path_changed =
                            ui::singleline(ui, "jsonpath-path", &mut self.path, "$.path");
                        ui.label(ui::t(ui, "common.output"));
                        ui::fill_code(ui, "jsonpath-out", &mut self.output, ui::t(ui, "common.output"), false);
                    });
                },
            );
        });
        if json_changed || path_changed {
            self.reeval();
        }
        ui.label("速查表");
        ui.horizontal_wrapped(|ui| {
            for (syntax, desc) in CHEAT_SHEET {
                ui.label(format!("{syntax}  {desc}"));
            }
        });
    }

    fn on_data_received(&mut self, payload: &str) {
        self.json = payload.to_string();
        self.reeval();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_none() {
        let mut view = JsonPathView::new();
        view.json = r#"{"a":1}"#.into();
        view.path = "$.a".into();
        view.output = "1".into();
        assert!(view.persistable_options().is_none());
        view.restore_options(&serde_json::json!({ "path": "$.stolen", "json": "{}" }));
        assert_eq!(view.json, r#"{"a":1}"#);
        assert_eq!(view.path, "$.a");
        assert!(view.persistable_options().is_none());
    }

    #[test]
    fn test_i18n_keys() {
        assert_eq!(
            devtoys_api::t("jsonpath.expression", devtoys_api::Language::ZhCn),
            "JSONPath 表达式"
        );
        assert_eq!(
            devtoys_api::t("jsonpath.expression", devtoys_api::Language::EnUs),
            "JSONPath expression"
        );
    }
}
