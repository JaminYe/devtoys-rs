use crate::slot::ToolView;
use crate::ui;

use super::{convert, Conversion, ID};

/// Settings JSON `conversion`: `encode` | `decode`. Unknown / missing → Encode, multiline=false.
const DEFAULT_CONVERSION: Conversion = Conversion::Encode;
const DEFAULT_MULTILINE: bool = false;

pub struct UrlView {
    input: String,
    output: String,
    conversion: Conversion,
    multiline: bool,
    error: Option<String>,
}

impl UrlView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            conversion: Conversion::Encode,
            multiline: false,
            error: None,
        }
    }

    fn recompute(&mut self) {
        if self.input.is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match convert(&self.input, self.conversion, self.multiline) {
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

impl ToolView for UrlView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                (ui::t("url.encode"), Conversion::Encode),
                (ui::t("url.decode"), Conversion::Decode),
            ] {
                if ui::toggle(ui, self.conversion == value, label).clicked() {
                    self.conversion = value;
                    self.recompute();
                }
            }
            if ui.checkbox(&mut self.multiline, "多行").changed() {
                self.recompute();
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
                    "url-in",
                    &mut self.input,
                    "粘贴文本",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    ui::t("common.output"),
                    "url-out",
                    &mut self.output,
                    "编解码结果",
                    false,
                );
            },
        );
        if input_changed {
            self.recompute();
        }
    }

    fn on_data_received(&mut self, _payload: &str) {}

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "conversion": conversion_to_settings(self.conversion),
                "multiline": self.multiline,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.conversion =
            conversion_from_settings(value.get("conversion").and_then(|v| v.as_str()));
        self.multiline = value
            .get("multiline")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_MULTILINE);
        self.recompute();
    }
}

fn conversion_to_settings(conversion: Conversion) -> &'static str {
    match conversion {
        Conversion::Encode => "encode",
        Conversion::Decode => "decode",
    }
}

fn conversion_from_settings(value: Option<&str>) -> Conversion {
    match value {
        Some("encode") => Conversion::Encode,
        Some("decode") => Conversion::Decode,
        _ => DEFAULT_CONVERSION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_mode_only() {
        let mut view = UrlView::new();
        view.conversion = Conversion::Decode;
        view.multiline = true;
        view.input = "a b".into();
        view.output = "a%20b".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["conversion"], "decode");
        assert_eq!(value["multiline"], true);
        assert!(value.get("input").is_none());
        assert!(!value.to_string().contains("a b"));
    }

    #[test]
    fn restore_decode_multiline_then_converts() {
        let mut view = UrlView::new();
        view.restore_options(&serde_json::json!({
            "conversion": "decode",
            "multiline": true
        }));
        assert_eq!(view.conversion, Conversion::Decode);
        assert!(view.multiline);
        view.input = "a%20b\nc%20d".into();
        view.recompute();
        assert_eq!(view.output, "a b\nc d");
        assert!(view.error.is_none());
    }

    #[test]
    fn missing_and_illegal_use_defaults() {
        let mut view = UrlView::new();
        view.conversion = Conversion::Decode;
        view.multiline = true;
        view.restore_options(&serde_json::json!({ "conversion": "rot13" }));
        assert_eq!(view.conversion, Conversion::Encode);
        assert!(!view.multiline);
        view.input = "a b".into();
        view.recompute();
        assert_eq!(view.output, "a%20b");
    }
}
