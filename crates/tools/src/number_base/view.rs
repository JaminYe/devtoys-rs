use crate::slot::ToolView;
use crate::ui;

use super::{
    convert_custom, convert_rfc4648, BasicBaseFields, NumberBase, Rfc4648Encoding, Signedness, ID,
};

/// Settings JSON encodings: `base16` | `base32` | `base32_hex` | `base64` | `base64_url`.
/// Unknown / missing → advanced=false, thousands=false, signedness=signed, from=base64, to=base16.
/// Custom alphabet is a dictionary; `use_custom` is also not persisted (would restore checked with an empty table).
const DEFAULT_ADVANCED: bool = false;
const DEFAULT_THOUSANDS: bool = false;
const DEFAULT_SIGNEDNESS: Signedness = Signedness::Signed;
const DEFAULT_FROM_ENCODING: Rfc4648Encoding = Rfc4648Encoding::Base64;
const DEFAULT_TO_ENCODING: Rfc4648Encoding = Rfc4648Encoding::Base16;

pub struct NumberBaseView {
    basic: BasicBaseFields,
    advanced_input: String,
    advanced_output: String,
    custom_alphabet: String,
    advanced: bool,
    thousands: bool,
    signedness: Signedness,
    last_from: NumberBase,
    from_encoding: Rfc4648Encoding,
    to_encoding: Rfc4648Encoding,
    use_custom: bool,
    syncing: bool,
}

impl NumberBaseView {
    pub fn new() -> Self {
        Self {
            basic: BasicBaseFields::default(),
            advanced_input: String::new(),
            advanced_output: String::new(),
            custom_alphabet: String::new(),
            advanced: false,
            thousands: false,
            signedness: DEFAULT_SIGNEDNESS,
            last_from: NumberBase::Decimal,
            from_encoding: Rfc4648Encoding::Base64,
            to_encoding: Rfc4648Encoding::Base16,
            use_custom: false,
            syncing: false,
        }
    }

    fn field_mut(&mut self, base: NumberBase) -> &mut String {
        self.basic.field_mut(base)
    }

    fn sync_from(&mut self, from: NumberBase) {
        if self.syncing || self.advanced {
            return;
        }
        self.last_from = from;
        self.basic
            .apply_input(from, self.thousands, self.signedness);
    }

    fn reconvert_advanced(&mut self) {
        if !self.advanced {
            return;
        }
        if self.advanced_input.trim().is_empty() {
            self.basic.error = None;
            self.advanced_output.clear();
            return;
        }
        let result = if self.use_custom {
            convert_custom(
                &self.advanced_input,
                &self.custom_alphabet,
                self.to_encoding.dictionary(),
            )
        } else {
            convert_rfc4648(&self.advanced_input, self.from_encoding, self.to_encoding)
        };
        match result {
            Ok(text) => {
                self.basic.error = None;
                self.advanced_output = text;
            }
            Err(err) => {
                self.basic.error = Some(err.to_string());
                self.advanced_output.clear();
            }
        }
    }

    fn encoding_row(&mut self, ui: &mut egui::Ui, is_from: bool) {
        for (label, encoding) in [
            ("Base16", Rfc4648Encoding::Base16),
            ("Base32", Rfc4648Encoding::Base32),
            ("Base32Hex", Rfc4648Encoding::Base32Hex),
            ("Base64", Rfc4648Encoding::Base64),
            ("Base64URL", Rfc4648Encoding::Base64Url),
        ] {
            let selected = if is_from {
                self.from_encoding == encoding
            } else {
                self.to_encoding == encoding
            };
            if ui::toggle(ui, selected, label).clicked() {
                if is_from {
                    self.from_encoding = encoding;
                    self.use_custom = false;
                } else {
                    self.to_encoding = encoding;
                }
                self.reconvert_advanced();
            }
        }
    }

    fn ui_basic(&mut self, ui: &mut egui::Ui) {
        let hint = match self.signedness {
            Signedness::Signed => {
                "有符号：64 位补码（与原版一致）；超出 Int64 的正整数仍按无符号大整数转换。"
            }
            Signedness::Unsigned => "无符号：64 位无符号整数（0 至 18446744073709551615）。",
        };
        ui.label(egui::RichText::new(hint).small().weak());
        ui.label(ui::t(ui, "number_base.decimal"));
        if ui::singleline(ui, "nb-dec", &mut self.basic.decimal, "十进制") {
            self.sync_from(NumberBase::Decimal);
        }
        ui.label(ui::t(ui, "number_base.hexadecimal"));
        if ui::singleline(ui, "nb-hex", &mut self.basic.hexadecimal, "十六进制") {
            self.sync_from(NumberBase::Hexadecimal);
        }
        ui.label(ui::t(ui, "number_base.octal"));
        if ui::singleline(ui, "nb-oct", &mut self.basic.octal, "八进制") {
            self.sync_from(NumberBase::Octal);
        }
        ui.label(ui::t(ui, "number_base.binary"));
        if ui::singleline(ui, "nb-bin", &mut self.basic.binary, "二进制") {
            self.sync_from(NumberBase::Binary);
        }
    }

    fn ui_advanced(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("高级模式按无符号 64 位整数换基，不是字节编码。")
                .small()
                .weak(),
        );
        ui.label("输入进制");
        ui.horizontal_wrapped(|ui| {
            self.encoding_row(ui, true);
        });
        ui.label("输出进制");
        ui.horizontal_wrapped(|ui| {
            self.encoding_row(ui, false);
            if ui.checkbox(&mut self.use_custom, "自定义字符表").changed() {
                self.reconvert_advanced();
            }
        });
        if self.use_custom
            && ui::singleline(ui, "nb-alpha", &mut self.custom_alphabet, "自定义字符表")
        {
            self.reconvert_advanced();
        }
        let spacing = 12.0;
        let total = ui.available_size();
        let w = ((total.x - spacing) / 2.0).max(80.0);
        ui.horizontal(|ui| {
            ui.set_min_height(total.y);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                if ui::labeled_code(
                    ui,
                    ui::t(ui, "common.input"),
                    "nb-adv-in",
                    &mut self.advanced_input,
                    "高级输入",
                    true,
                ) {
                    self.reconvert_advanced();
                }
            });
            ui.add_space(spacing);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui::labeled_code(
                    ui,
                    ui::t(ui, "common.output"),
                    "nb-adv-out",
                    &mut self.advanced_output,
                    "高级输出",
                    false,
                );
            });
        });
    }
}

impl ToolView for NumberBaseView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui::toggle(ui, !self.advanced, "基础").clicked() {
                self.advanced = false;
                self.basic.error = None;
            }
            if ui::toggle(ui, self.advanced, "高级").clicked() {
                self.advanced = true;
                self.reconvert_advanced();
            }
            if !self.advanced {
                if ui::toggle(ui, self.signedness == Signedness::Signed, ui::t(ui, "number_base.signed")).clicked() {
                    self.signedness = Signedness::Signed;
                    self.sync_from(self.last_from);
                }
                if ui::toggle(ui, self.signedness == Signedness::Unsigned, ui::t(ui, "number_base.unsigned")).clicked() {
                    self.signedness = Signedness::Unsigned;
                    self.sync_from(self.last_from);
                }
            }
            if ui.checkbox(&mut self.thousands, ui::t(ui, "number_base.format_thousands")).changed() {
                self.sync_from(NumberBase::Decimal);
            }
            ui::copy_button(
                ui,
                self.basic.error.is_none().then_some(if self.advanced {
                    self.advanced_output.as_str()
                } else {
                    self.basic.hexadecimal.as_str()
                }),
            );
        });
        ui::error_label(ui, self.basic.error.as_deref());
        if self.advanced {
            self.ui_advanced(ui);
        } else {
            self.ui_basic(ui);
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.advanced = false;
        let trimmed = payload.trim();
        let (base, value) = if let Some(rest) = strip_prefix_ci(trimmed, "0x") {
            (NumberBase::Hexadecimal, rest)
        } else if let Some(rest) = strip_prefix_ci(trimmed, "0b") {
            (NumberBase::Binary, rest)
        } else if let Some(rest) = strip_prefix_ci(trimmed, "0o") {
            (NumberBase::Octal, rest)
        } else {
            (NumberBase::Decimal, trimmed)
        };
        self.syncing = true;
        *self.field_mut(base) = value.to_string();
        self.syncing = false;
        self.sync_from(base);
    }

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "advanced": self.advanced,
                "thousands": self.thousands,
                "signedness": signedness_to_settings(self.signedness),
                "from_encoding": encoding_to_settings(self.from_encoding),
                "to_encoding": encoding_to_settings(self.to_encoding),
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.advanced = value
            .get("advanced")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_ADVANCED);
        self.thousands = value
            .get("thousands")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_THOUSANDS);
        self.signedness =
            signedness_from_settings(value.get("signedness").and_then(|v| v.as_str()))
                .unwrap_or(DEFAULT_SIGNEDNESS);
        self.from_encoding =
            encoding_from_settings(value.get("from_encoding").and_then(|v| v.as_str()))
                .unwrap_or(DEFAULT_FROM_ENCODING);
        self.to_encoding =
            encoding_from_settings(value.get("to_encoding").and_then(|v| v.as_str()))
                .unwrap_or(DEFAULT_TO_ENCODING);
        if self.advanced {
            self.reconvert_advanced();
        }
    }
}

fn signedness_to_settings(signedness: Signedness) -> &'static str {
    match signedness {
        Signedness::Signed => "signed",
        Signedness::Unsigned => "unsigned",
    }
}

fn signedness_from_settings(value: Option<&str>) -> Option<Signedness> {
    match value {
        Some("signed") => Some(Signedness::Signed),
        Some("unsigned") => Some(Signedness::Unsigned),
        _ => None,
    }
}

fn encoding_to_settings(encoding: Rfc4648Encoding) -> &'static str {
    match encoding {
        Rfc4648Encoding::Base16 => "base16",
        Rfc4648Encoding::Base32 => "base32",
        Rfc4648Encoding::Base32Hex => "base32_hex",
        Rfc4648Encoding::Base64 => "base64",
        Rfc4648Encoding::Base64Url => "base64_url",
    }
}

fn encoding_from_settings(value: Option<&str>) -> Option<Rfc4648Encoding> {
    match value {
        Some("base16") => Some(Rfc4648Encoding::Base16),
        Some("base32") => Some(Rfc4648Encoding::Base32),
        Some("base32_hex") => Some(Rfc4648Encoding::Base32Hex),
        Some("base64") => Some(Rfc4648Encoding::Base64),
        Some("base64_url") => Some(Rfc4648Encoding::Base64Url),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_exclude_content_and_alphabet() {
        let mut view = NumberBaseView::new();
        view.advanced = true;
        view.thousands = true;
        view.from_encoding = Rfc4648Encoding::Base32;
        view.to_encoding = Rfc4648Encoding::Base64Url;
        view.use_custom = true;
        view.custom_alphabet = "01".into();
        view.advanced_input = "secret-nb".into();
        view.basic.decimal = "1000".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["advanced"], true);
        assert_eq!(value["thousands"], true);
        assert_eq!(value["signedness"], "signed");
        assert_eq!(value["from_encoding"], "base32");
        assert_eq!(value["to_encoding"], "base64_url");
        assert!(value.get("use_custom").is_none());
        assert!(value.get("custom_alphabet").is_none());
        assert!(value.get("decimal").is_none());
        assert!(value.get("advanced_input").is_none());
        let dumped = value.to_string();
        assert!(!dumped.contains("secret-nb"));
        assert!(!dumped.contains("1000"));
    }

    #[test]
    fn restore_thousands_then_formats_basic_output() {
        let mut view = NumberBaseView::new();
        view.restore_options(&serde_json::json!({ "thousands": true }));
        assert!(view.thousands);
        assert!(!view.advanced);
        view.basic.decimal = "1000000".into();
        view.sync_from(NumberBase::Decimal);
        assert_eq!(view.basic.hexadecimal, "F4,240");
        assert!(view.basic.binary.contains(','), "{}", view.basic.binary);
    }

    #[test]
    fn restore_advanced_encodings_then_converts() {
        let mut view = NumberBaseView::new();
        view.restore_options(&serde_json::json!({
            "advanced": true,
            "from_encoding": "base64",
            "to_encoding": "base16"
        }));
        assert!(view.advanced);
        assert_eq!(view.from_encoding, Rfc4648Encoding::Base64);
        assert_eq!(view.to_encoding, Rfc4648Encoding::Base16);
        view.advanced_input = "D/".into();
        view.reconvert_advanced();
        assert_eq!(view.advanced_output, "FF");
        assert!(view.basic.error.is_none());

        view.from_encoding = Rfc4648Encoding::Base16;
        view.to_encoding = Rfc4648Encoding::Base64;
        view.advanced_input = "FF".into();
        view.reconvert_advanced();
        assert_eq!(view.advanced_output, "D/");
        assert!(view.basic.error.is_none());
    }

    #[test]
    fn advanced_custom_binary_alphabet_to_hex() {
        let mut view = NumberBaseView::new();
        view.advanced = true;
        view.use_custom = true;
        view.custom_alphabet = "01".into();
        view.to_encoding = Rfc4648Encoding::Base16;
        view.advanced_input = "1111".into();
        view.reconvert_advanced();
        assert_eq!(view.advanced_output, "F");
        assert!(view.basic.error.is_none());
    }

    #[test]
    fn missing_and_illegal_encodings_use_defaults() {
        let mut view = NumberBaseView::new();
        view.advanced = true;
        view.thousands = true;
        view.from_encoding = Rfc4648Encoding::Base32;
        view.to_encoding = Rfc4648Encoding::Base32Hex;
        view.restore_options(&serde_json::json!({
            "from_encoding": "base85",
            "to_encoding": "rot13",
            "use_custom": true
        }));
        assert!(!view.advanced);
        assert!(!view.thousands);
        assert_eq!(view.from_encoding, Rfc4648Encoding::Base64);
        assert_eq!(view.to_encoding, Rfc4648Encoding::Base16);
        assert!(
            !view.use_custom,
            "stale use_custom in old settings must not enable custom mode without an alphabet"
        );
        view.restore_options(&serde_json::json!({}));
        assert!(!view.advanced);
        assert_eq!(view.from_encoding, Rfc4648Encoding::Base64);
        assert_eq!(view.to_encoding, Rfc4648Encoding::Base16);
        assert_eq!(view.signedness, Signedness::Signed);
    }

    #[test]
    fn unsigned_hex_all_f_is_u64_max_not_minus_one() {
        let mut view = NumberBaseView::new();
        view.signedness = Signedness::Unsigned;
        view.basic.hexadecimal = "FFFFFFFFFFFFFFFF".into();
        view.sync_from(NumberBase::Hexadecimal);
        assert_eq!(view.basic.decimal, "18446744073709551615");
        assert_eq!(view.basic.hexadecimal, "FFFFFFFFFFFFFFFF");
        assert!(!view.basic.decimal.contains('-'));
        assert!(!view.basic.hexadecimal.contains('-'));
        assert!(view.basic.error.is_none());

        view.basic.decimal = "18446744073709551615".into();
        view.sync_from(NumberBase::Decimal);
        assert_eq!(view.basic.hexadecimal, "FFFFFFFFFFFFFFFF");
        assert!(!view.basic.hexadecimal.contains('-'));

        view.signedness = Signedness::Signed;
        view.basic.decimal = "-1".into();
        view.sync_from(NumberBase::Decimal);
        assert_eq!(view.basic.hexadecimal, "FFFFFFFFFFFFFFFF");
        assert_eq!(view.basic.decimal, "-1");
    }

    #[test]
    fn restore_unsigned_then_converts_and_rfc_path_ignores_switch() {
        let mut view = NumberBaseView::new();
        view.restore_options(&serde_json::json!({ "signedness": "unsigned" }));
        assert_eq!(view.signedness, Signedness::Unsigned);
        assert!(!view.advanced);
        view.basic.hexadecimal = "FFFFFFFFFFFFFFFF".into();
        view.sync_from(NumberBase::Hexadecimal);
        assert_eq!(view.basic.decimal, "18446744073709551615");

        view.restore_options(&serde_json::json!({ "signedness": "twos" }));
        assert_eq!(view.signedness, Signedness::Signed);

        view.advanced = true;
        view.signedness = Signedness::Unsigned;
        view.from_encoding = Rfc4648Encoding::Base16;
        view.to_encoding = Rfc4648Encoding::Base64;
        view.advanced_input = "FF".into();
        view.reconvert_advanced();
        assert_eq!(view.advanced_output, "D/");
        assert!(view.basic.error.is_none());
    }
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        text.get(prefix.len()..)
    } else {
        None
    }
}
