use crate::slot::ToolView;
use crate::ui;

use super::{checksum_matches, compute_hash, HashAlgorithm};

pub struct HashChecksumView {
    input: String,
    hmac: String,
    expected: String,
    output: String,
    algorithm: HashAlgorithm,
    uppercase: bool,
    match_state: Option<bool>,
    error: Option<String>,
}

impl HashChecksumView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            hmac: String::new(),
            expected: String::new(),
            output: String::new(),
            algorithm: HashAlgorithm::Md5,
            uppercase: false,
            match_state: None,
            error: None,
        }
    }

    fn recompute(&mut self) {
        let hmac = if self.hmac.is_empty() {
            None
        } else {
            Some(self.hmac.as_str())
        };
        match compute_hash(&self.input, self.algorithm, hmac, self.uppercase, false) {
            Ok(hex) => {
                self.error = None;
                self.match_state = if self.expected.trim().is_empty() {
                    None
                } else {
                    Some(checksum_matches(&hex, &self.expected))
                };
                self.output = hex;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.match_state = None;
                self.output.clear();
            }
        }
    }
}

impl ToolView for HashChecksumView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("MD5", HashAlgorithm::Md5),
                ("SHA1", HashAlgorithm::Sha1),
                ("SHA256", HashAlgorithm::Sha256),
                ("SHA384", HashAlgorithm::Sha384),
                ("SHA512", HashAlgorithm::Sha512),
            ] {
                if ui::toggle(ui, self.algorithm == value, label).clicked() {
                    self.algorithm = value;
                    dirty = true;
                }
            }
            dirty |= ui.checkbox(&mut self.uppercase, "大写").changed();
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui.columns(2, |cols| {
            cols[0].label("HMAC 密钥");
            if ui::singleline(
                &mut cols[0],
                "hash-hmac",
                &mut self.hmac,
                "HMAC 密钥（可选）",
            ) {
                dirty = true;
            }
            cols[1].label("期望校验和");
            if ui::singleline(
                &mut cols[1],
                "hash-expected",
                &mut self.expected,
                "期望校验和",
            ) {
                dirty = true;
            }
        });
        if dirty {
            self.recompute();
        }
        ui::error_label(ui, self.error.as_deref());
        if let Some(matched) = self.match_state {
            if matched {
                ui.colored_label(ui::success(ui), "匹配");
            } else {
                ui.colored_label(ui::danger(ui), "不匹配");
            }
        }
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed =
                    ui::labeled_code(ui, "输入", "hash-in", &mut self.input, "输入文本", true);
            },
            |ui| {
                ui::labeled_code(ui, "输出", "hash-out", &mut self.output, "哈希结果", false);
            },
        );
        if input_changed {
            self.recompute();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = payload.to_string();
        self.recompute();
    }
}
