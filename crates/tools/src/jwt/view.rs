use crate::slot::ToolView;
use crate::ui;

use super::{decode_jwt, encode_jwt, JwtAlgorithm, JwtDecodeOptions, JwtEncodeOptions};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum JwtMode {
    #[default]
    Decode,
    Encode,
}

pub struct JwtView {
    mode: JwtMode,
    token: String,
    payload: String,
    secret: String,
    issuer: String,
    audience: String,
    actor: String,
    expiration: String,
    header_out: String,
    payload_out: String,
    signature_out: String,
    token_out: String,
    algorithm: JwtAlgorithm,
    secret_is_base64: bool,
    validate_lifetime: bool,
    add_default_time_claims: bool,
    error: Option<String>,
}

impl JwtView {
    pub fn new() -> Self {
        Self {
            mode: JwtMode::Decode,
            token: String::new(),
            payload: String::new(),
            secret: String::new(),
            issuer: String::new(),
            audience: String::new(),
            actor: String::new(),
            expiration: String::new(),
            header_out: String::new(),
            payload_out: String::new(),
            signature_out: String::new(),
            token_out: String::new(),
            algorithm: JwtAlgorithm::Hs256,
            secret_is_base64: false,
            validate_lifetime: false,
            add_default_time_claims: false,
            error: None,
        }
    }

    fn recompute(&mut self) {
        match self.mode {
            JwtMode::Decode => self.decode_now(),
            JwtMode::Encode => self.encode_now(),
        }
    }

    fn decode_now(&mut self) {
        if self.token.trim().is_empty() {
            self.error = None;
            self.header_out.clear();
            self.payload_out.clear();
            self.signature_out.clear();
            return;
        }
        let opts = JwtDecodeOptions {
            secret: optional_text(&self.secret),
            secret_is_base64: self.secret_is_base64,
            issuer: optional_text(&self.issuer),
            audience: optional_text(&self.audience),
            validate_lifetime: self.validate_lifetime,
            actor: optional_text(&self.actor),
        };
        match decode_jwt(&self.token, &opts) {
            Ok(decoded) => {
                self.error = None;
                self.header_out = decoded.header;
                self.payload_out = decoded.payload;
                self.signature_out = decoded.signature;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.header_out.clear();
                self.payload_out.clear();
                self.signature_out.clear();
            }
        }
    }

    fn encode_now(&mut self) {
        if self.payload.trim().is_empty() {
            self.error = None;
            self.token_out.clear();
            return;
        }
        let opts = JwtEncodeOptions {
            algorithm: self.algorithm,
            secret: self.secret.clone(),
            secret_is_base64: self.secret_is_base64,
            issuer: optional_text(&self.issuer),
            audience: optional_text(&self.audience),
            add_default_time_claims: self.add_default_time_claims,
            expiration_secs: parse_expiration(&self.expiration),
        };
        match encode_jwt(&self.payload, &opts) {
            Ok(token) => {
                self.error = None;
                self.token_out = token;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.token_out.clear();
            }
        }
    }
}

fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_expiration(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse().ok()
    }
}

fn split_3(
    ui: &mut egui::Ui,
    a: impl FnOnce(&mut egui::Ui),
    b: impl FnOnce(&mut egui::Ui),
    c: impl FnOnce(&mut egui::Ui),
) {
    let spacing = 12.0;
    let total = ui.available_size();
    let w = ((total.x - spacing * 2.0) / 3.0).max(80.0);
    ui.horizontal(|ui| {
        ui.set_min_height(total.y);
        ui.allocate_ui(egui::vec2(w, total.y), a);
        ui.add_space(spacing);
        ui.allocate_ui(egui::vec2(w, total.y), b);
        ui.add_space(spacing);
        ui.allocate_ui(egui::vec2(w, total.y), c);
    });
}

impl ToolView for JwtView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            if ui::toggle(ui, self.mode == JwtMode::Decode, "解码").clicked() {
                self.mode = JwtMode::Decode;
                dirty = true;
            }
            if ui::toggle(ui, self.mode == JwtMode::Encode, "编码").clicked() {
                self.mode = JwtMode::Encode;
                dirty = true;
            }
            dirty |= ui
                .checkbox(&mut self.secret_is_base64, "Base64 密钥")
                .changed();
            ui::copy_button(
                ui,
                self.error.is_none().then_some(match self.mode {
                    JwtMode::Decode => self.token.as_str(),
                    JwtMode::Encode => self.token_out.as_str(),
                }),
            );
        });
        ui::error_label(ui, self.error.as_deref());
        dirty |= ui::singleline(ui, "jwt-secret", &mut self.secret, "密钥");
        ui.columns(2, |cols| {
            dirty |= ui::singleline(&mut cols[0], "jwt-iss", &mut self.issuer, "签发者 iss");
            dirty |= ui::singleline(&mut cols[1], "jwt-aud", &mut self.audience, "受众 aud");
        });
        match self.mode {
            JwtMode::Decode => {
                dirty |= ui
                    .checkbox(&mut self.validate_lifetime, "校验有效期")
                    .changed();
                dirty |= ui::singleline(ui, "jwt-actor", &mut self.actor, "Actor");
                let avail = ui.available_size();
                let token_h = 120.0;
                ui.allocate_ui(egui::vec2(avail.x, token_h), |ui| {
                    dirty |= ui::labeled_code(
                        ui,
                        "Token",
                        "jwt-token",
                        &mut self.token,
                        "粘贴 Token",
                        true,
                    );
                });
                if dirty {
                    self.recompute();
                }
                split_3(
                    ui,
                    |ui| {
                        ui::labeled_code(
                            ui,
                            "头部",
                            "jwt-header",
                            &mut self.header_out,
                            "头部",
                            false,
                        );
                    },
                    |ui| {
                        ui::labeled_code(
                            ui,
                            "载荷",
                            "jwt-payload-out",
                            &mut self.payload_out,
                            "载荷",
                            false,
                        );
                    },
                    |ui| {
                        ui::labeled_code(
                            ui,
                            "签名",
                            "jwt-sig",
                            &mut self.signature_out,
                            "签名",
                            false,
                        );
                    },
                );
            }
            JwtMode::Encode => {
                ui.horizontal_wrapped(|ui| {
                    for alg in JwtAlgorithm::ALL {
                        if ui::toggle(ui, self.algorithm == alg, alg.as_str()).clicked() {
                            self.algorithm = alg;
                            dirty = true;
                        }
                    }
                });
                dirty |= ui
                    .checkbox(&mut self.add_default_time_claims, "默认时间声明")
                    .changed();
                dirty |= ui::singleline(ui, "jwt-exp", &mut self.expiration, "过期秒数");
                let avail = ui.available_size();
                let token_h = 100.0;
                ui.allocate_ui(egui::vec2(avail.x, (avail.y - token_h).max(80.0)), |ui| {
                    dirty |= ui::labeled_code(
                        ui,
                        "载荷",
                        "jwt-payload",
                        &mut self.payload,
                        "Payload JSON",
                        true,
                    );
                });
                if dirty {
                    self.recompute();
                }
                ui.allocate_ui(egui::vec2(avail.x, token_h), |ui| {
                    ui::labeled_code(
                        ui,
                        "Token",
                        "jwt-token-out",
                        &mut self.token_out,
                        "Token",
                        false,
                    );
                });
            }
        }
    }

    fn on_data_received(&mut self, _: &str) {}
}
