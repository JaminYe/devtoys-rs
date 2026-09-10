use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use crate::slot::ToolView;
use crate::ui;

use super::helper::{
    checksum_matches_input, compute_hash, hash_path, is_file_input, HashAlgorithm, HashCancel,
    HashError, HashProgress, HashSession, HashTaskResult,
};
use super::ID;

const DEFAULT_ALGORITHM: HashAlgorithm = HashAlgorithm::Md5;
const DEFAULT_UPPERCASE: bool = false;

pub struct HashChecksumView {
    input: String,
    hmac: String,
    expected: String,
    output: String,
    algorithm: HashAlgorithm,
    uppercase: bool,
    match_state: Option<bool>,
    error: Option<String>,
    session: HashSession,
    cancel: HashCancel,
    progress: Arc<Mutex<HashProgress>>,
    rx: Option<Receiver<HashTaskResult>>,
    busy: bool,
}

impl HashChecksumView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            hmac: String::new(),
            expected: String::new(),
            output: String::new(),
            algorithm: DEFAULT_ALGORITHM,
            uppercase: DEFAULT_UPPERCASE,
            match_state: None,
            error: None,
            session: HashSession::new(),
            cancel: HashCancel::new(),
            progress: Arc::new(Mutex::new(HashProgress::default())),
            rx: None,
            busy: false,
        }
    }

    fn hmac_key(&self) -> Option<&str> {
        if self.hmac.is_empty() {
            None
        } else {
            Some(self.hmac.as_str())
        }
    }

    fn apply_hex(&mut self, hex: String) {
        self.error = None;
        self.match_state = if self.expected.trim().is_empty() {
            None
        } else {
            Some(checksum_matches_input(&hex, &self.expected))
        };
        self.output = hex;
    }

    fn fail(&mut self, err: &HashError) {
        self.error = Some(err.to_string());
        self.match_state = None;
        self.output.clear();
    }

    fn refresh_match(&mut self) {
        if self.busy || self.error.is_some() || self.output.is_empty() {
            self.match_state = None;
            return;
        }
        self.match_state = if self.expected.trim().is_empty() {
            None
        } else {
            Some(checksum_matches_input(&self.output, &self.expected))
        };
    }

    fn current_progress(&self) -> HashProgress {
        self.progress.lock().map(|guard| *guard).unwrap_or_default()
    }

    fn pick_file(&mut self) -> bool {
        let Some(path) = rfd::FileDialog::new().set_title("选择文件").pick_file() else {
            return false;
        };
        self.input = path.to_string_lossy().into_owned();
        true
    }

    fn kick(&mut self) {
        self.cancel.cancel();
        let generation = self.session.begin();
        self.cancel = HashCancel::new();
        self.rx = None;

        if !is_file_input(&self.input, false) {
            self.busy = false;
            match compute_hash(
                &self.input,
                self.algorithm,
                self.hmac_key(),
                self.uppercase,
                false,
            ) {
                Ok(hex) => self.apply_hex(hex),
                Err(err) => self.fail(&err),
            }
            return;
        }

        self.busy = true;
        self.output.clear();
        self.match_state = None;
        self.error = None;
        *self.progress.lock().unwrap_or_else(|err| err.into_inner()) = HashProgress::default();

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let path = PathBuf::from(&self.input);
        let algorithm = self.algorithm;
        let hmac = self.hmac_key().map(str::to_owned);
        let uppercase = self.uppercase;
        let cancel = self.cancel.clone();
        let progress = self.progress.clone();
        std::thread::spawn(move || {
            let result = hash_path(
                &path,
                algorithm,
                hmac.as_deref().map(str::as_bytes),
                uppercase,
                generation,
                &cancel,
                |update| {
                    *progress.lock().unwrap_or_else(|err| err.into_inner()) = update;
                },
            );
            let _ = tx.send(result);
        });
    }

    fn poll_task(&mut self, ctx: &egui::Context) {
        if self.rx.is_none() {
            return;
        }
        match self.rx.as_ref().unwrap().try_recv() {
            Ok(result) => {
                self.rx = None;
                self.busy = false;
                match self.session.accept(result) {
                    Some(Ok(hex)) => self.apply_hex(hex),
                    Some(Err(HashError::Cancelled)) => {
                        self.output.clear();
                        self.match_state = None;
                        self.error = None;
                    }
                    Some(Err(err)) => self.fail(&err),
                    None => {}
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.rx = None;
                self.busy = false;
                self.error = Some("任务失败".into());
                self.output.clear();
                self.match_state = None;
            }
        }
    }
}

impl Drop for HashChecksumView {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl ToolView for HashChecksumView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.poll_task(ui.ctx());

        let mut hash_dirty = false;
        let mut compare_dirty = false;
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
                    hash_dirty = true;
                }
            }
            hash_dirty |= ui
                .checkbox(&mut self.uppercase, ui::t("hash.uppercase"))
                .changed();
            if ui.button("选择文件").clicked() {
                hash_dirty |= self.pick_file();
            }
            ui::copy_button(
                ui,
                (!self.busy && self.error.is_none()).then_some(self.output.as_str()),
            );
        });
        ui.columns(2, |cols| {
            cols[0].label(ui::t("hash.secret_key"));
            if ui::singleline(
                &mut cols[0],
                "hash-hmac",
                &mut self.hmac,
                "HMAC 密钥（可选）",
            ) {
                hash_dirty = true;
            }
            cols[1].label("期望校验和");
            if ui::singleline(
                &mut cols[1],
                "hash-expected",
                &mut self.expected,
                "期望校验和或文件路径",
            ) {
                compare_dirty = true;
            }
        });

        if self.busy {
            ui.horizontal(|ui| {
                let progress = self.current_progress();
                match (progress.fraction(), progress.total_bytes) {
                    (Some(fraction), Some(total)) => {
                        ui.add(
                            egui::ProgressBar::new(fraction)
                                .desired_width(240.0)
                                .text(format!("{} / {total} 字节", progress.bytes_read)),
                        );
                    }
                    _ => {
                        ui.add(
                            egui::ProgressBar::new(0.0)
                                .desired_width(240.0)
                                .animate(true)
                                .text(format!("计算中… {} 字节", progress.bytes_read)),
                        );
                    }
                }
                if ui.button("取消").clicked() {
                    self.cancel.cancel();
                }
            });
        }

        if hash_dirty {
            self.kick();
        } else if compare_dirty {
            self.refresh_match();
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
                input_changed = ui::labeled_code(
                    ui,
                    ui::t("common.input"),
                    "hash-in",
                    &mut self.input,
                    "输入文本或文件路径",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(
                    ui,
                    ui::t("common.output"),
                    "hash-out",
                    &mut self.output,
                    "哈希结果",
                    false,
                );
            },
        );
        if input_changed {
            self.kick();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = payload.to_string();
        self.kick();
    }

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "algorithm": self.algorithm.as_str(),
                "uppercase": self.uppercase,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.algorithm = algorithm_from_settings(value.get("algorithm").and_then(|v| v.as_str()));
        self.uppercase = value
            .get("uppercase")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_UPPERCASE);
    }
}

fn algorithm_from_settings(value: Option<&str>) -> HashAlgorithm {
    value
        .and_then(HashAlgorithm::parse)
        .unwrap_or(DEFAULT_ALGORITHM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    /// Independent NIST/RFC SHA-256("abc").
    const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const MD5_ABC: &str = "900150983cd24fb0d6963f7d28e17f72";

    fn assert_no_sensitive(value: &serde_json::Value) {
        assert!(value.get("input").is_none());
        assert!(value.get("output").is_none());
        assert!(value.get("hmac").is_none());
        assert!(value.get("expected").is_none());
        assert!(value.get("path").is_none());
    }

    #[test]
    fn persistable_options_are_algorithm_and_uppercase_only() {
        let mut view = HashChecksumView::new();
        view.algorithm = HashAlgorithm::Sha256;
        view.uppercase = true;
        view.input = r"C:\Users\secret\file.bin".into();
        view.hmac = "super-secret-hmac-key".into();
        view.expected = "deadbeef-checksum".into();
        view.output = "should-not-persist-hash".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["algorithm"], "Sha256");
        assert_eq!(value["uppercase"], true);
        assert_no_sensitive(&value);
        let dumped = value.to_string();
        assert!(!dumped.contains("secret"));
        assert!(!dumped.contains("deadbeef"));
        assert!(!dumped.contains("should-not-persist"));
        assert!(!dumped.contains(r"C:\\Users"));
    }

    #[test]
    fn restore_sha256_uppercase_then_hashes_independently() {
        let mut view = HashChecksumView::new();
        view.restore_options(&serde_json::json!({
            "algorithm": "Sha256",
            "uppercase": true
        }));
        view.on_data_received("abc");
        assert_eq!(view.algorithm, HashAlgorithm::Sha256);
        assert!(view.uppercase);
        assert_eq!(view.output, SHA256_ABC.to_ascii_uppercase());
        assert!(view.hmac.is_empty());
    }

    #[test]
    fn restore_ignores_hmac_and_input_from_json() {
        let mut view = HashChecksumView::new();
        view.hmac = "keep-hmac".into();
        view.input = "keep-input".into();
        view.restore_options(&serde_json::json!({
            "algorithm": "Sha256",
            "uppercase": false,
            "hmac": "injected-secret",
            "input": "abc",
            "output": "00"
        }));
        assert_eq!(view.hmac, "keep-hmac");
        assert_eq!(view.input, "keep-input");
        assert_ne!(view.output, "00");
        let mut fresh = HashChecksumView::new();
        fresh.restore_options(&serde_json::json!({
            "algorithm": "Sha256",
            "hmac": "key"
        }));
        fresh.on_data_received("abc");
        assert!(fresh.hmac.is_empty());
        assert_eq!(fresh.output, SHA256_ABC);
    }

    #[test]
    fn missing_fields_use_documented_defaults() {
        let mut view = HashChecksumView::new();
        view.algorithm = HashAlgorithm::Sha512;
        view.uppercase = true;
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.algorithm, HashAlgorithm::Md5);
        assert!(!view.uppercase);
        view.on_data_received("abc");
        assert_eq!(view.output, MD5_ABC);
    }

    #[test]
    fn illegal_algorithm_defaults_but_keeps_valid_uppercase() {
        let mut view = HashChecksumView::new();
        view.restore_options(&serde_json::json!({
            "algorithm": "Blake3",
            "uppercase": true
        }));
        assert_eq!(view.algorithm, HashAlgorithm::Md5);
        assert!(view.uppercase);
        view.on_data_received("abc");
        assert_eq!(view.output, MD5_ABC.to_ascii_uppercase());
    }

    #[test]
    fn reconstruct_from_serialized_settings_object() {
        let mut first = HashChecksumView::new();
        first.algorithm = HashAlgorithm::Sha256;
        first.uppercase = true;
        let (id, value) = first.persistable_options().unwrap();
        assert_eq!(id, "HashAndChecksumGenerator");

        let stored = serde_json::json!({
            "theme": "dark",
            "tool_options": {
                "JsonFormatter": { "indent": "minified" },
                id: value
            }
        });
        let mut second = HashChecksumView::new();
        second.restore_options(&stored["tool_options"]["HashAndChecksumGenerator"]);
        second.on_data_received("abc");
        assert_eq!(second.output, SHA256_ABC.to_ascii_uppercase());
        let persisted = second.persistable_options().unwrap().1;
        assert_no_sensitive(&persisted);
        assert!(!persisted.to_string().contains("abc"));
    }
}
