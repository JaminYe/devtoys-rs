use crate::slot::ToolView;
use crate::ui;

use super::{convert, looks_like_base64, Charset, Conversion, ID};

/// Settings JSON `conversion`: `encode` | `decode`. `charset`: `utf8` | `ascii`.
/// Unknown / missing → Encode, Utf8, multiline=false.
const DEFAULT_CONVERSION: Conversion = Conversion::Encode;
const DEFAULT_CHARSET: Charset = Charset::Utf8;
const DEFAULT_MULTILINE: bool = false;

pub struct Base64TextView {
    input: String,
    output: String,
    conversion: Conversion,
    charset: Charset,
    multiline: bool,
    error: Option<String>,
}

impl Base64TextView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            conversion: Conversion::Encode,
            charset: Charset::Utf8,
            multiline: false,
            error: None,
        }
    }

    fn reconvert(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match convert(&self.input, self.conversion, self.charset, self.multiline) {
            Ok(result) => {
                self.error = None;
                self.output = result;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for Base64TextView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                (ui::t(ui, "base64_text.encode"), Conversion::Encode),
                (ui::t(ui, "base64_text.decode"), Conversion::Decode),
            ] {
                if ui::toggle(ui, self.conversion == value, label).clicked() {
                    self.conversion = value;
                    self.reconvert();
                }
            }
            for (label, value) in [("UTF-8", Charset::Utf8), ("ASCII", Charset::Ascii)] {
                if ui::toggle(ui, self.charset == value, label).clicked() {
                    self.charset = value;
                    self.reconvert();
                }
            }
            if ui.checkbox(&mut self.multiline, ui::t(ui, "base64_text.multiline")).changed() {
                self.reconvert();
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        let err_text = self.error.as_deref().map(|e| match e {
            "非法 Base64" => ui::t(ui, "base64_text.invalid"),
            "非 ASCII 文本" => ui::t(ui, "base64_text.not_ascii"),
            _ => e,
        });
        ui::error_label(ui, err_text);
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed = ui::labeled_code(
                    ui,
                    ui::t(ui, "common.input"),
                    "b64-in",
                    &mut self.input,
                    "粘贴文本或 Base64",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    ui::t(ui, "common.output"),
                    "b64-out",
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
        self.conversion = if looks_like_base64(payload) {
            Conversion::Decode
        } else {
            Conversion::Encode
        };
        self.input = payload.to_string();
        self.reconvert();
    }

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "conversion": conversion_to_settings(self.conversion),
                "charset": charset_to_settings(self.charset),
                "multiline": self.multiline,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.conversion =
            conversion_from_settings(value.get("conversion").and_then(|v| v.as_str()));
        self.charset = charset_from_settings(value.get("charset").and_then(|v| v.as_str()));
        self.multiline = value
            .get("multiline")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_MULTILINE);
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

fn charset_to_settings(charset: Charset) -> &'static str {
    match charset {
        Charset::Utf8 => "utf8",
        Charset::Ascii => "ascii",
    }
}

fn charset_from_settings(value: Option<&str>) -> Charset {
    match value {
        Some("utf8") => Charset::Utf8,
        Some("ascii") => Charset::Ascii,
        _ => DEFAULT_CHARSET,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_mode_only() {
        let mut view = Base64TextView::new();
        view.conversion = Conversion::Decode;
        view.charset = Charset::Ascii;
        view.multiline = true;
        view.input = "hello".into();
        view.output = "aGVsbG8=".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["conversion"], "decode");
        assert_eq!(value["charset"], "ascii");
        assert_eq!(value["multiline"], true);
        assert!(value.get("input").is_none());
        assert!(value.get("output").is_none());
        assert!(!value.to_string().contains("hello"));
    }

    #[test]
    fn restore_decode_multiline_then_converts() {
        let mut view = Base64TextView::new();
        view.restore_options(&serde_json::json!({
            "conversion": "decode",
            "charset": "utf8",
            "multiline": true
        }));
        assert_eq!(view.conversion, Conversion::Decode);
        assert!(view.multiline);
        view.input = "aGk=\nYQ==".into();
        view.reconvert();
        assert_eq!(view.output, "hi\na");
        assert!(view.error.is_none());
    }

    #[test]
    fn missing_and_illegal_use_defaults() {
        let mut view = Base64TextView::new();
        view.conversion = Conversion::Decode;
        view.charset = Charset::Ascii;
        view.multiline = true;
        view.restore_options(&serde_json::json!({
            "conversion": "rot13",
            "charset": "latin1"
        }));
        assert_eq!(view.conversion, Conversion::Encode);
        assert_eq!(view.charset, Charset::Utf8);
        assert!(!view.multiline);
        view.input = "hi".into();
        view.reconvert();
        assert_eq!(view.output, "aGk=");
    }

    #[test]
    fn test_base64_text_view_ui_localization() {
        let mut view = Base64TextView::new();
        view.conversion = Conversion::Decode;
        view.input = "invalid base64!!!".into();
        view.reconvert();
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
        assert!(zh_texts.iter().any(|t| t == "多行"), "ZhCn should contain '多行'");
        assert!(zh_texts.iter().any(|t| t == "非法 Base64"), "ZhCn should contain '非法 Base64'");

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
        assert!(en_texts.iter().any(|t| t == "Invalid Base64"), "EnUs should contain 'Invalid Base64'");
    }
}
