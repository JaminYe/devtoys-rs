use crate::slot::ToolView;
use crate::ui;

use super::{
    add_thousands_separators, convert_base, convert_rfc4648, decode_custom, encode_custom,
    encode_rfc4648, NumberBase, Rfc4648Encoding,
};

pub struct NumberBaseView {
    decimal: String,
    hexadecimal: String,
    octal: String,
    binary: String,
    advanced_input: String,
    advanced_output: String,
    custom_alphabet: String,
    advanced: bool,
    thousands: bool,
    from_encoding: Rfc4648Encoding,
    to_encoding: Rfc4648Encoding,
    use_custom: bool,
    syncing: bool,
    error: Option<String>,
}

impl NumberBaseView {
    pub fn new() -> Self {
        Self {
            decimal: String::new(),
            hexadecimal: String::new(),
            octal: String::new(),
            binary: String::new(),
            advanced_input: String::new(),
            advanced_output: String::new(),
            custom_alphabet: String::new(),
            advanced: false,
            thousands: false,
            from_encoding: Rfc4648Encoding::Base64,
            to_encoding: Rfc4648Encoding::Base16,
            use_custom: false,
            syncing: false,
            error: None,
        }
    }

    fn field_mut(&mut self, base: NumberBase) -> &mut String {
        match base {
            NumberBase::Decimal => &mut self.decimal,
            NumberBase::Hexadecimal => &mut self.hexadecimal,
            NumberBase::Octal => &mut self.octal,
            NumberBase::Binary => &mut self.binary,
        }
    }

    fn field(&self, base: NumberBase) -> &str {
        match base {
            NumberBase::Decimal => &self.decimal,
            NumberBase::Hexadecimal => &self.hexadecimal,
            NumberBase::Octal => &self.octal,
            NumberBase::Binary => &self.binary,
        }
    }

    fn sync_from(&mut self, from: NumberBase) {
        if self.syncing || self.advanced {
            return;
        }
        let source = self.field(from).to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.clear_others(from);
            return;
        }
        let targets = [
            NumberBase::Decimal,
            NumberBase::Hexadecimal,
            NumberBase::Octal,
            NumberBase::Binary,
        ];
        self.syncing = true;
        let mut failed = false;
        for to in targets {
            if to == from {
                continue;
            }
            match convert_base(&source, from, to) {
                Ok(mut text) => {
                    if self.thousands {
                        text = add_thousands_separators(&text);
                    }
                    *self.field_mut(to) = text;
                }
                Err(err) => {
                    self.error = Some(err.to_string());
                    failed = true;
                    break;
                }
            }
        }
        if !failed {
            self.error = None;
        }
        self.syncing = false;
    }

    fn clear_others(&mut self, from: NumberBase) {
        self.syncing = true;
        for base in [
            NumberBase::Decimal,
            NumberBase::Hexadecimal,
            NumberBase::Octal,
            NumberBase::Binary,
        ] {
            if base == from {
                continue;
            }
            self.field_mut(base).clear();
        }
        self.syncing = false;
    }

    fn reconvert_advanced(&mut self) {
        if !self.advanced {
            return;
        }
        if self.advanced_input.trim().is_empty() {
            self.error = None;
            self.advanced_output.clear();
            return;
        }
        let result = if self.use_custom {
            encode_custom(&self.advanced_input, &self.custom_alphabet)
                .or_else(|_| decode_custom(&self.advanced_input, &self.custom_alphabet))
        } else if self.from_encoding == self.to_encoding {
            encode_rfc4648(&self.advanced_input, self.to_encoding)
        } else {
            convert_rfc4648(&self.advanced_input, self.from_encoding, self.to_encoding)
        };
        match result {
            Ok(text) => {
                self.error = None;
                self.advanced_output = text;
            }
            Err(err) => {
                self.error = Some(err.to_string());
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
                } else {
                    self.to_encoding = encoding;
                }
                self.use_custom = false;
                self.reconvert_advanced();
            }
        }
    }

    fn ui_basic(&mut self, ui: &mut egui::Ui) {
        ui.label("十进制");
        if ui::singleline(ui, "nb-dec", &mut self.decimal, "十进制") {
            self.sync_from(NumberBase::Decimal);
        }
        ui.label("十六进制");
        if ui::singleline(ui, "nb-hex", &mut self.hexadecimal, "十六进制") {
            self.sync_from(NumberBase::Hexadecimal);
        }
        ui.label("八进制");
        if ui::singleline(ui, "nb-oct", &mut self.octal, "八进制") {
            self.sync_from(NumberBase::Octal);
        }
        ui.label("二进制");
        if ui::singleline(ui, "nb-bin", &mut self.binary, "二进制") {
            self.sync_from(NumberBase::Binary);
        }
    }

    fn ui_advanced(&mut self, ui: &mut egui::Ui) {
        ui.label("输入编码");
        ui.horizontal_wrapped(|ui| {
            self.encoding_row(ui, true);
        });
        ui.label("输出编码");
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
                    "输入",
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
                    "输出",
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
                self.error = None;
            }
            if ui::toggle(ui, self.advanced, "高级").clicked() {
                self.advanced = true;
                self.reconvert_advanced();
            }
            if ui.checkbox(&mut self.thousands, "千分位").changed() {
                self.sync_from(NumberBase::Decimal);
            }
            if ui::primary_button(ui, "复制").clicked() && self.error.is_none() {
                let text = if self.advanced {
                    &self.advanced_output
                } else {
                    &self.hexadecimal
                };
                ui::copy_text(ui, text);
            }
        });
        ui::error_label(ui, self.error.as_deref());
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
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        text.get(prefix.len()..)
    } else {
        None
    }
}
