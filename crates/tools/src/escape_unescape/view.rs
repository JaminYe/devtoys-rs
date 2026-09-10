use crate::slot::ToolView;
use crate::ui;

use super::{convert, Conversion, ID};

/// Settings JSON `conversion`: `encode` | `decode`. Unknown / missing → Encode.
const DEFAULT_CONVERSION: Conversion = Conversion::Encode;

pub struct EscapeUnescapeView {
    input: String,
    output: String,
    conversion: Conversion,
    error: Option<String>,
}

impl EscapeUnescapeView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            conversion: Conversion::Encode,
            error: None,
        }
    }

    fn reconvert(&mut self) {
        match convert(&self.input, self.conversion) {
            Ok(result) => {
                self.error = None;
                self.output = result;
            }
            Err(_) => {
                self.error = Some("非法转义序列".into());
                self.output.clear();
            }
        }
    }
}

impl ToolView for EscapeUnescapeView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                (ui::t("escape.escape"), Conversion::Encode),
                (ui::t("escape.unescape"), Conversion::Decode),
            ] {
                if ui::toggle(ui, self.conversion == value, label).clicked() {
                    self.conversion = value;
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
                    "escape-in",
                    &mut self.input,
                    ui::t("common.paste"),
                    true,
                );
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    ui::t("common.output"),
                    "escape-out",
                    &mut self.output,
                    ui::t("common.output"),
                    false,
                );
            },
        );
        if input_changed {
            self.reconvert();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.conversion = Conversion::Decode;
        self.input = payload.to_string();
        self.reconvert();
    }

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "conversion": conversion_to_settings(self.conversion),
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.conversion =
            conversion_from_settings(value.get("conversion").and_then(|v| v.as_str()));
        self.reconvert();
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
    fn persistable_options_are_conversion_only() {
        let mut view = EscapeUnescapeView::new();
        view.conversion = Conversion::Decode;
        view.input = "a\\nb".into();
        view.output = "a\nb".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["conversion"], "decode");
        assert!(value.get("input").is_none());
        assert!(!value.to_string().contains("a\\nb"));
    }

    #[test]
    fn restore_decode_then_unescapes_backspace_and_form_feed() {
        let mut view = EscapeUnescapeView::new();
        view.restore_options(&serde_json::json!({ "conversion": "decode" }));
        assert_eq!(view.conversion, Conversion::Decode);
        view.input = "a\\nb\\tc\\\\d\\b\\f".into();
        view.reconvert();
        assert_eq!(view.output, "a\nb\tc\\d\u{8}\u{c}");
        assert!(view.error.is_none());
    }

    #[test]
    fn missing_and_illegal_use_encode() {
        let mut view = EscapeUnescapeView::new();
        view.conversion = Conversion::Decode;
        view.restore_options(&serde_json::json!({ "conversion": "html" }));
        assert_eq!(view.conversion, Conversion::Encode);
        view.input = "a\nb".into();
        view.reconvert();
        assert_eq!(view.output, "a\\nb");
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.conversion, Conversion::Encode);
    }

    #[test]
    fn test_escape_i18n_keys() {
        assert_eq!(devtoys_api::t("escape.escape"), "转义");
        assert_eq!(devtoys_api::t("escape.unescape"), "反转义");
    }

    #[test]
    fn on_data_received_switches_to_decode_and_unescapes() {
        let mut view = EscapeUnescapeView::new();
        assert_eq!(view.conversion, Conversion::Encode);
        view.on_data_received("hello\\r\\n\\t\\b\\f\\\"\\'\\\\\\u0041");
        assert_eq!(view.conversion, Conversion::Decode);
        assert!(view.error.is_none());
        assert_eq!(view.output, "hello\r\n\t\u{8}\u{c}\"'\\A");
    }

    #[test]
    fn on_data_received_preserves_unknown_sequence_and_trailing_backslash() {
        let mut view = EscapeUnescapeView::new();
        view.on_data_received("hello\\qworld\\");
        assert_eq!(view.conversion, Conversion::Decode);
        assert!(view.error.is_none());
        assert_eq!(view.output, "hello\\qworld\\");
    }

    #[test]
    fn on_data_received_invalid_u_shows_error() {
        let mut view = EscapeUnescapeView::new();
        view.on_data_received("hello\\uZZZZ");
        assert_eq!(view.conversion, Conversion::Decode);
        assert_eq!(view.error.as_deref(), Some("非法转义序列"));
        assert!(view.output.is_empty());
    }
}
