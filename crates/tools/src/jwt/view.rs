#[cfg(feature = "gui")]
use crate::slot::ToolView;
#[cfg(feature = "gui")]
use crate::ui;

#[cfg(feature = "gui")]
use super::ID;
use super::{
    claims_table, decode_jwt, encode_jwt, JwtAlgorithm, JwtDecodeOptions, JwtEncodeOptions,
    JwtError,
};

const DEFAULT_MODE: JwtMode = JwtMode::Decode;
const DEFAULT_ALGORITHM: JwtAlgorithm = JwtAlgorithm::Hs256;
const DEFAULT_SECRET_IS_BASE64: bool = false;
const DEFAULT_VALIDATE_TOKEN: bool = false;
const DEFAULT_VALIDATE_SIGNATURE: bool = false;
const DEFAULT_VALIDATE_ISSUER: bool = false;
const DEFAULT_VALIDATE_AUDIENCE: bool = false;
const DEFAULT_VALIDATE_LIFETIME: bool = false;
const DEFAULT_VALIDATE_ACTOR: bool = false;
const DEFAULT_ADD_DEFAULT_TIME_CLAIMS: bool = false;

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
    validate_token: bool,
    validate_signature: bool,
    validate_issuer: bool,
    validate_audience: bool,
    validate_lifetime: bool,
    validate_actor: bool,
    add_default_time_claims: bool,
    claims: Vec<(String, String)>,
    error: Option<String>,
}

impl JwtView {
    pub fn new() -> Self {
        Self {
            mode: DEFAULT_MODE,
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
            algorithm: DEFAULT_ALGORITHM,
            secret_is_base64: DEFAULT_SECRET_IS_BASE64,
            validate_token: DEFAULT_VALIDATE_TOKEN,
            validate_signature: DEFAULT_VALIDATE_SIGNATURE,
            validate_issuer: DEFAULT_VALIDATE_ISSUER,
            validate_audience: DEFAULT_VALIDATE_AUDIENCE,
            validate_lifetime: DEFAULT_VALIDATE_LIFETIME,
            validate_actor: DEFAULT_VALIDATE_ACTOR,
            add_default_time_claims: DEFAULT_ADD_DEFAULT_TIME_CLAIMS,
            claims: Vec::new(),
            error: None,
        }
    }

    /// GUI 粘贴入口：原样写入密钥，不把 CR/LF 折叠成空格。
    fn apply_pasted_key(&mut self, key: &str) {
        self.secret = key.to_string();
        self.recompute();
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
            self.claims.clear();
            return;
        }
        let opts = JwtDecodeOptions {
            secret: optional_text(&self.secret),
            secret_is_base64: self.secret_is_base64,
            issuer: optional_text(&self.issuer),
            audience: optional_text(&self.audience),
            actor: optional_text(&self.actor),
            validate_token: self.validate_token,
            validate_signature: self.validate_signature,
            validate_issuer: self.validate_issuer,
            validate_audience: self.validate_audience,
            validate_lifetime: self.validate_lifetime,
            validate_actor: self.validate_actor,
        };
        match decode_jwt(&self.token, &opts) {
            Ok(decoded) => {
                self.error = None;
                self.apply_decoded(decoded);
            }
            Err(err) => {
                self.error = Some(err.to_string());
                if matches!(err, JwtError::VerificationFailed) {
                    let mut peek = opts.clone();
                    peek.validate_token = false;
                    if let Ok(decoded) = decode_jwt(&self.token, &peek) {
                        self.apply_decoded(decoded);
                        return;
                    }
                }
                self.header_out.clear();
                self.payload_out.clear();
                self.signature_out.clear();
                self.claims.clear();
            }
        }
    }

    fn apply_decoded(&mut self, decoded: super::JwtDecoded) {
        self.claims = claims_table(&decoded.payload);
        self.header_out = decoded.header;
        self.payload_out = decoded.payload;
        self.signature_out = decoded.signature;
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

#[cfg(feature = "gui")]
fn key_edit(ui: &mut egui::Ui, text: &mut String) -> bool {
    // PEM 含换行；TextEdit::singleline 会把 CR/LF 换成空格，严格 PEM 解码失败。
    ui.add(
        egui::TextEdit::multiline(text)
            .id_salt("jwt-secret")
            .hint_text("密钥")
            .desired_width(f32::INFINITY)
            .desired_rows(6)
            .font(egui::TextStyle::Monospace),
    )
    .changed()
}

#[cfg(feature = "gui")]
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

#[cfg(feature = "gui")]
impl ToolView for JwtView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            if ui::toggle(
                ui,
                self.mode == JwtMode::Decode,
                ui::t("base64_text.decode"),
            )
            .clicked()
            {
                self.mode = JwtMode::Decode;
                dirty = true;
            }
            if ui::toggle(
                ui,
                self.mode == JwtMode::Encode,
                ui::t("base64_text.encode"),
            )
            .clicked()
            {
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
        if let Some(err) = self.error.as_deref() {
            let msg = if err == "校验失败" {
                ui::t("jwt.invalid")
            } else {
                err
            };
            ui::error_label(ui, Some(msg));
        } else if self.validate_token && !self.token.trim().is_empty() {
            ui.colored_label(ui::success(ui), ui::t("jwt.valid"));
        }
        dirty |= key_edit(ui, &mut self.secret);
        ui.columns(2, |cols| {
            dirty |= ui::singleline(&mut cols[0], "jwt-iss", &mut self.issuer, "签发者 iss");
            dirty |= ui::singleline(&mut cols[1], "jwt-aud", &mut self.audience, "受众 aud");
        });
        match self.mode {
            JwtMode::Decode => {
                dirty |= ui
                    .checkbox(&mut self.validate_token, ui::t("jwt.validate"))
                    .changed();
                ui.add_enabled_ui(self.validate_token, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        dirty |= ui
                            .checkbox(&mut self.validate_signature, "签名密钥")
                            .changed();
                        dirty |= ui.checkbox(&mut self.validate_issuer, "签发者").changed();
                        dirty |= ui.checkbox(&mut self.validate_audience, "受众").changed();
                        dirty |= ui.checkbox(&mut self.validate_lifetime, "有效期").changed();
                        dirty |= ui.checkbox(&mut self.validate_actor, "actor").changed();
                    });
                });
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
                            ui::t("jwt.header"),
                            "jwt-header",
                            &mut self.header_out,
                            "头部",
                            false,
                        );
                    },
                    |ui| {
                        let avail = ui.available_size();
                        let json_h = (avail.y * 0.55).max(80.0);
                        ui.allocate_ui(egui::vec2(avail.x, json_h), |ui| {
                            ui::labeled_code(
                                ui,
                                ui::t("jwt.payload"),
                                "jwt-payload-out",
                                &mut self.payload_out,
                                "载荷",
                                false,
                            );
                        });
                        ui.label("声明");
                        egui::ScrollArea::vertical()
                            .id_salt("jwt-claims-scroll")
                            .show(ui, |ui| {
                                egui::Grid::new("jwt-claims")
                                    .num_columns(2)
                                    .striped(true)
                                    .min_col_width(48.0)
                                    .show(ui, |ui| {
                                        ui.strong("名称");
                                        ui.strong("值");
                                        ui.end_row();
                                        for (name, value) in &self.claims {
                                            ui.label(name);
                                            ui.monospace(value);
                                            ui.end_row();
                                        }
                                    });
                            });
                    },
                    |ui| {
                        ui::labeled_code(
                            ui,
                            ui::t("jwt.signature"),
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
                        ui::t("jwt.payload"),
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

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "mode": mode_to_settings(self.mode),
                "algorithm": self.algorithm.as_str(),
                "secret_is_base64": self.secret_is_base64,
                "validate_token": self.validate_token,
                "validate_signature": self.validate_signature,
                "validate_issuer": self.validate_issuer,
                "validate_audience": self.validate_audience,
                "validate_lifetime": self.validate_lifetime,
                "validate_actor": self.validate_actor,
                "add_default_time_claims": self.add_default_time_claims,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.mode = mode_from_settings(value.get("mode").and_then(|v| v.as_str()));
        self.algorithm = algorithm_from_settings(value.get("algorithm").and_then(|v| v.as_str()));
        self.secret_is_base64 = value
            .get("secret_is_base64")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_SECRET_IS_BASE64);
        self.validate_token = value
            .get("validate_token")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_VALIDATE_TOKEN);
        self.validate_signature = value
            .get("validate_signature")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_VALIDATE_SIGNATURE);
        self.validate_issuer = value
            .get("validate_issuer")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_VALIDATE_ISSUER);
        self.validate_audience = value
            .get("validate_audience")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_VALIDATE_AUDIENCE);
        self.validate_lifetime = value
            .get("validate_lifetime")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_VALIDATE_LIFETIME);
        self.validate_actor = value
            .get("validate_actor")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_VALIDATE_ACTOR);
        self.add_default_time_claims = value
            .get("add_default_time_claims")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_ADD_DEFAULT_TIME_CLAIMS);
        self.recompute();
    }
}

#[cfg(feature = "gui")]
fn mode_to_settings(mode: JwtMode) -> &'static str {
    match mode {
        JwtMode::Decode => "decode",
        JwtMode::Encode => "encode",
    }
}

#[cfg(feature = "gui")]
fn mode_from_settings(value: Option<&str>) -> JwtMode {
    match value {
        Some("encode") => JwtMode::Encode,
        _ => DEFAULT_MODE,
    }
}

#[cfg(feature = "gui")]
fn algorithm_from_settings(value: Option<&str>) -> JwtAlgorithm {
    value
        .and_then(JwtAlgorithm::parse)
        .unwrap_or(DEFAULT_ALGORITHM)
}

#[cfg(test)]
mod tests {
    use super::super::helper::{
        ES512_INDEPENDENT_TOKEN, ES512_PRIVATE_PKCS8, ES512_PRIVATE_SEC1, ES512_PUBLIC,
        HS256_SECRET, HS256_TOKEN,
    };
    use super::*;
    #[cfg(feature = "gui")]
    use crate::slot::ToolView;

    fn pem_crlf(pem: &str) -> String {
        pem.replace('\n', "\r\n")
    }

    fn enable_signature_checks(view: &mut JwtView) {
        view.validate_token = true;
        view.validate_signature = true;
    }

    #[test]
    fn apply_pasted_key_keeps_lf_and_crlf() {
        let mut view = JwtView::new();
        view.apply_pasted_key(ES512_PRIVATE_SEC1);
        assert!(view.secret.contains('\n'), "pasted PEM must keep LF");
        assert!(
            !view.secret.contains("-----BEGIN EC PRIVATE KEY----- MIHc"),
            "must not collapse PEM newlines to spaces"
        );

        let crlf = pem_crlf(ES512_PUBLIC);
        view.apply_pasted_key(&crlf);
        assert!(view.secret.contains("\r\n"));
        assert!(view.secret.contains("-----BEGIN PUBLIC KEY-----"));
    }

    #[test]
    fn pasted_multiline_pem_signs_and_verifies() {
        let mut view = JwtView::new();
        view.mode = JwtMode::Encode;
        view.algorithm = JwtAlgorithm::Es512;
        view.payload = r#"{"sub":"1"}"#.into();
        view.apply_pasted_key(ES512_PRIVATE_SEC1);
        assert_eq!(view.error, None);
        assert!(!view.token_out.is_empty());
        let lf_token = view.token_out.clone();

        view.apply_pasted_key(&pem_crlf(ES512_PRIVATE_SEC1));
        assert_eq!(view.error, None);
        assert!(!view.token_out.is_empty());

        view.apply_pasted_key(&pem_crlf(ES512_PRIVATE_PKCS8));
        assert_eq!(view.error, None);
        let pkcs8_token = view.token_out.clone();

        view.mode = JwtMode::Decode;
        view.token = lf_token;
        enable_signature_checks(&mut view);
        view.apply_pasted_key(&pem_crlf(ES512_PUBLIC));
        assert_eq!(view.error, None);
        assert!(view.header_out.contains("ES512"));
        assert!(view.payload_out.contains("\"sub\""));
        assert!(!view.error.as_deref().unwrap_or("").contains("BEGIN"));

        view.token = pkcs8_token;
        view.apply_pasted_key(ES512_PUBLIC);
        assert_eq!(view.error, None);
        assert!(view.header_out.contains("ES512"));

        view.token = ES512_INDEPENDENT_TOKEN.to_string();
        view.apply_pasted_key(&pem_crlf(ES512_PUBLIC));
        assert_eq!(view.error, None);
        assert!(view.payload_out.contains("es512-vector"));
    }

    #[test]
    fn hmac_text_and_base64_keys_still_work() {
        let mut view = JwtView::new();
        view.mode = JwtMode::Encode;
        view.algorithm = JwtAlgorithm::Hs256;
        view.payload = r#"{"sub":"1"}"#.into();
        view.apply_pasted_key("secret");
        assert_eq!(view.error, None);
        let token = view.token_out.clone();
        assert!(!token.is_empty());

        view.mode = JwtMode::Decode;
        view.token = token.clone();
        enable_signature_checks(&mut view);
        view.apply_pasted_key("secret");
        assert_eq!(view.error, None);
        assert!(view.header_out.contains("HS256"));
        assert!(view.payload_out.contains("\"sub\""));

        view.mode = JwtMode::Encode;
        view.secret_is_base64 = true;
        view.apply_pasted_key("c2VjcmV0");
        assert_eq!(view.error, None);
        assert_eq!(
            view.token_out, token,
            "Base64 of 'secret' must match the text HMAC key"
        );

        view.mode = JwtMode::Decode;
        view.token = token;
        enable_signature_checks(&mut view);
        view.apply_pasted_key("c2VjcmV0");
        assert_eq!(view.error, None);
        assert!(view.header_out.contains("HS256"));
    }

    #[test]
    fn decode_without_key_differs_from_verified_with_key() {
        let mut view = JwtView::new();
        view.mode = JwtMode::Encode;
        view.algorithm = JwtAlgorithm::Es512;
        view.payload = r#"{"sub":"1"}"#.into();
        view.apply_pasted_key(ES512_PRIVATE_SEC1);
        let token = view.token_out.clone();

        view.mode = JwtMode::Decode;
        view.token = token;
        view.apply_pasted_key("");
        assert_eq!(view.error, None);
        assert!(view.header_out.contains("ES512"));
        let unverified = view.payload_out.clone();
        assert!(unverified.contains("\"sub\""));
        assert!(!view.claims.is_empty());

        enable_signature_checks(&mut view);
        view.apply_pasted_key(ES512_PUBLIC);
        assert_eq!(view.error, None);
        assert_eq!(view.payload_out, unverified);

        let bad = "-----BEGIN PUBLIC KEY-----\nnot-a-real-key\n-----END PUBLIC KEY-----";
        view.apply_pasted_key(bad);
        assert_eq!(view.error.as_deref(), Some("非法密钥"));
        let message = view.error.clone().unwrap();
        assert!(!message.contains(bad));
        assert!(!message.contains("not-a-real-key"));
        assert!(!message.contains("BEGIN"));
        assert!(view.payload_out.is_empty());
        assert!(view.claims.is_empty());
    }

    #[test]
    fn decode_fills_claims_table_from_fixed_token() {
        let mut view = JwtView::new();
        view.token = HS256_TOKEN.into();
        view.apply_pasted_key("wrong");
        assert_eq!(view.error, None);
        assert_eq!(
            view.claims,
            vec![
                ("sub".into(), "1234567890".into()),
                ("name".into(), "John Doe".into()),
                ("iat".into(), "1516239022".into()),
            ]
        );

        enable_signature_checks(&mut view);
        view.apply_pasted_key("wrong");
        assert_eq!(view.error.as_deref(), Some("校验失败"));
        assert_eq!(
            view.claims,
            vec![
                ("sub".into(), "1234567890".into()),
                ("name".into(), "John Doe".into()),
                ("iat".into(), "1516239022".into()),
            ]
        );
        assert!(view.payload_out.contains("\"sub\""));

        view.apply_pasted_key(HS256_SECRET);
        assert_eq!(view.error, None);
        assert_eq!(view.claims[0], ("sub".into(), "1234567890".into()));
    }

    #[cfg(feature = "gui")]
    const SECRET: &str = "unit-test-secret";
    #[cfg(feature = "gui")]
    const PAYLOAD: &str = r#"{"sub":"persist-1"}"#;

    #[cfg(feature = "gui")]
    fn assert_no_sensitive(value: &serde_json::Value) {
        for key in [
            "secret",
            "token",
            "payload",
            "issuer",
            "audience",
            "actor",
            "expiration",
            "token_out",
            "header_out",
            "payload_out",
            "signature_out",
            "claims",
            "input",
            "output",
        ] {
            assert!(value.get(key).is_none(), "unexpected key {key}");
        }
    }

    #[cfg(feature = "gui")]
    #[test]
    fn persistable_options_are_mode_algorithm_and_switches_only() {
        let mut view = JwtView::new();
        view.mode = JwtMode::Encode;
        view.algorithm = JwtAlgorithm::Hs384;
        view.secret_is_base64 = true;
        view.validate_token = true;
        view.validate_signature = true;
        view.validate_issuer = true;
        view.validate_audience = true;
        view.validate_lifetime = true;
        view.validate_actor = true;
        view.add_default_time_claims = true;
        view.secret = SECRET.into();
        view.token = "header.payload.sig".into();
        view.payload = PAYLOAD.into();
        view.issuer = "iss-claim".into();
        view.audience = "aud-claim".into();
        view.actor = "act-claim".into();
        view.expiration = "3600".into();
        view.token_out = "should-not-persist-token".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["mode"], "encode");
        assert_eq!(value["algorithm"], "HS384");
        assert_eq!(value["secret_is_base64"], true);
        assert_eq!(value["validate_token"], true);
        assert_eq!(value["validate_signature"], true);
        assert_eq!(value["validate_issuer"], true);
        assert_eq!(value["validate_audience"], true);
        assert_eq!(value["validate_lifetime"], true);
        assert_eq!(value["validate_actor"], true);
        assert_eq!(value["add_default_time_claims"], true);
        assert_no_sensitive(&value);
        let dumped = value.to_string();
        assert!(!dumped.contains(SECRET));
        assert!(!dumped.contains("persist-1"));
        assert!(!dumped.contains("iss-claim"));
        assert!(!dumped.contains("should-not-persist"));
    }

    #[cfg(feature = "gui")]
    #[test]
    fn restore_encode_hs384_then_signs_independently() {
        let mut view = JwtView::new();
        view.restore_options(&serde_json::json!({
            "mode": "encode",
            "algorithm": "HS384",
            "secret_is_base64": false,
            "validate_lifetime": false,
            "add_default_time_claims": false
        }));
        assert_eq!(view.mode, JwtMode::Encode);
        assert_eq!(view.algorithm, JwtAlgorithm::Hs384);
        view.payload = PAYLOAD.into();
        view.apply_pasted_key(SECRET);
        assert_eq!(view.error, None);
        let expected = encode_jwt(
            PAYLOAD,
            &JwtEncodeOptions {
                algorithm: JwtAlgorithm::Hs384,
                secret: SECRET.into(),
                secret_is_base64: false,
                issuer: None,
                audience: None,
                add_default_time_claims: false,
                expiration_secs: None,
            },
        )
        .unwrap();
        assert_eq!(view.token_out, expected);
    }

    #[cfg(feature = "gui")]
    #[test]
    fn restore_decode_validate_lifetime_then_verifies_independently() {
        let token = encode_jwt(
            PAYLOAD,
            &JwtEncodeOptions {
                algorithm: JwtAlgorithm::Hs256,
                secret: SECRET.into(),
                ..JwtEncodeOptions::default()
            },
        )
        .unwrap();
        let mut view = JwtView::new();
        view.token = token.clone();
        view.secret = SECRET.into();
        view.restore_options(&serde_json::json!({
            "mode": "decode",
            "algorithm": "HS256",
            "validate_token": true,
            "validate_lifetime": true
        }));
        assert_eq!(view.mode, JwtMode::Decode);
        assert!(view.validate_token);
        assert!(view.validate_lifetime);
        assert_eq!(view.error.as_deref(), Some("校验失败"));
        assert!(view.payload_out.contains("persist-1"));

        view.restore_options(&serde_json::json!({
            "mode": "decode",
            "validate_token": true,
            "validate_signature": true,
            "validate_lifetime": false
        }));
        assert!(view.validate_token);
        assert!(view.validate_signature);
        assert!(!view.validate_lifetime);
        assert_eq!(view.error, None);
        assert!(view.payload_out.contains("persist-1"));
        let expected = decode_jwt(
            &token,
            &JwtDecodeOptions {
                secret: Some(SECRET.into()),
                validate_token: true,
                validate_signature: true,
                validate_lifetime: false,
                ..JwtDecodeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(view.payload_out, expected.payload);
    }

    #[cfg(feature = "gui")]
    #[test]
    fn restore_add_default_time_claims_then_encodes_exp() {
        let mut view = JwtView::new();
        view.restore_options(&serde_json::json!({
            "mode": "encode",
            "add_default_time_claims": true
        }));
        assert!(view.add_default_time_claims);
        view.payload = PAYLOAD.into();
        view.apply_pasted_key(SECRET);
        assert_eq!(view.error, None);
        let decoded = decode_jwt(&view.token_out, &JwtDecodeOptions::default()).unwrap();
        assert!(decoded.payload.contains("\"iat\""), "{}", decoded.payload);
        assert!(decoded.payload.contains("\"exp\""), "{}", decoded.payload);
    }

    #[cfg(feature = "gui")]
    #[test]
    fn restore_ignores_secret_token_and_claims_from_json() {
        let mut view = JwtView::new();
        view.secret = "keep-secret".into();
        view.payload = "keep-payload".into();
        view.token = "keep.token.sig".into();
        view.issuer = "keep-iss".into();
        view.restore_options(&serde_json::json!({
            "mode": "encode",
            "algorithm": "HS256",
            "secret": "injected-secret",
            "token": "a.b.c",
            "payload": PAYLOAD,
            "issuer": "injected-iss",
            "audience": "injected-aud"
        }));
        assert_eq!(view.secret, "keep-secret");
        assert_eq!(view.payload, "keep-payload");
        assert_eq!(view.token, "keep.token.sig");
        assert_eq!(view.issuer, "keep-iss");
        assert!(view.audience.is_empty());
    }

    #[cfg(feature = "gui")]
    #[test]
    fn missing_fields_use_documented_defaults() {
        let mut view = JwtView::new();
        view.mode = JwtMode::Encode;
        view.algorithm = JwtAlgorithm::Rs256;
        view.secret_is_base64 = true;
        view.validate_token = true;
        view.validate_signature = true;
        view.validate_issuer = true;
        view.validate_audience = true;
        view.validate_lifetime = true;
        view.validate_actor = true;
        view.add_default_time_claims = true;
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.mode, JwtMode::Decode);
        assert_eq!(view.algorithm, JwtAlgorithm::Hs256);
        assert!(!view.secret_is_base64);
        assert!(!view.validate_token);
        assert!(!view.validate_signature);
        assert!(!view.validate_issuer);
        assert!(!view.validate_audience);
        assert!(!view.validate_lifetime);
        assert!(!view.validate_actor);
        assert!(!view.add_default_time_claims);
    }

    #[cfg(feature = "gui")]
    #[test]
    fn illegal_algorithm_defaults_but_keeps_valid_mode() {
        let mut view = JwtView::new();
        view.restore_options(&serde_json::json!({
            "mode": "encode",
            "algorithm": "HS1024",
            "secret_is_base64": true
        }));
        assert_eq!(view.mode, JwtMode::Encode);
        assert_eq!(view.algorithm, JwtAlgorithm::Hs256);
        assert!(view.secret_is_base64);
    }

    #[cfg(feature = "gui")]
    #[test]
    fn reconstruct_from_serialized_settings_object() {
        let mut first = JwtView::new();
        first.mode = JwtMode::Encode;
        first.algorithm = JwtAlgorithm::Hs512;
        first.secret_is_base64 = true;
        let (id, value) = first.persistable_options().unwrap();
        assert_eq!(id, "JsonWebTokenEncoderDecoder");

        let stored = serde_json::json!({
            "theme": "dark",
            "tool_options": {
                "JsonFormatter": { "indent": "minified" },
                id: value
            }
        });
        let mut second = JwtView::new();
        second.restore_options(&stored["tool_options"]["JsonWebTokenEncoderDecoder"]);
        assert_eq!(second.mode, JwtMode::Encode);
        assert_eq!(second.algorithm, JwtAlgorithm::Hs512);
        assert!(second.secret_is_base64);
        second.payload = PAYLOAD.into();
        second.apply_pasted_key("c2VjcmV0");
        let expected = encode_jwt(
            PAYLOAD,
            &JwtEncodeOptions {
                algorithm: JwtAlgorithm::Hs512,
                secret: "c2VjcmV0".into(),
                secret_is_base64: true,
                ..JwtEncodeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(second.token_out, expected);
        let persisted = second.persistable_options().unwrap().1;
        assert_no_sensitive(&persisted);
        assert!(!persisted.to_string().contains("persist-1"));
        assert!(!persisted.to_string().contains("c2VjcmV0"));
    }

    #[test]
    fn encoders_generators_keys_have_both_translations_and_error_path_translates() {
        let keys = [
            "base64_text.title",
            "base64_text.encode",
            "base64_text.decode",
            "base64_image.title",
            "jwt.title",
            "jwt.header",
            "jwt.payload",
            "jwt.signature",
            "jwt.validate",
            "jwt.valid",
            "jwt.invalid",
            "url.title",
            "url.encode",
            "url.decode",
            "hash.title",
            "hash.algorithm",
            "hash.uppercase",
            "hash.hmac",
            "hash.secret_key",
            "password.title",
            "password.length",
            "password.digits",
            "password.uppercase",
            "password.lowercase",
            "password.special",
            "password.generate",
            "uuid.title",
            "uuid.count",
            "uuid.hyphens",
            "uuid.uppercase",
            "uuid.version",
        ];
        for key in keys {
            let zh = devtoys_api::t(key);
            assert_ne!(zh, key, "Key {key} should have Chinese translation");
            assert!(!zh.is_empty(), "Translation for {key} should not be empty");
        }

        // Test error path translation: JWT invalid signature
        let err_zh = devtoys_api::t("jwt.invalid");
        assert_eq!(err_zh, "签名无效");
    }
}
