use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;

use super::{
    decode_jwt, encode_jwt, JwtAlgorithm, JwtDecodeOptions, JwtEncodeOptions,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum JwtMode {
    #[default]
    Decode,
    Encode,
}

pub struct JwtView {
    mode: JwtMode,
    token: Entity<InputState>,
    payload: Entity<InputState>,
    secret: Entity<InputState>,
    issuer: Entity<InputState>,
    audience: Entity<InputState>,
    actor: Entity<InputState>,
    expiration: Entity<InputState>,
    header_out: Entity<InputState>,
    payload_out: Entity<InputState>,
    signature_out: Entity<InputState>,
    token_out: Entity<InputState>,
    algorithm: JwtAlgorithm,
    secret_is_base64: bool,
    validate_lifetime: bool,
    add_default_time_claims: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl JwtView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let token = multiline(window, cx, "粘贴 Token", 6);
        let payload = multiline(window, cx, "Payload JSON", 8);
        let secret = single(window, cx, "密钥");
        let issuer = single(window, cx, "签发者 iss");
        let audience = single(window, cx, "受众 aud");
        let actor = single(window, cx, "Actor");
        let expiration = single(window, cx, "过期秒数");
        let header_out = multiline(window, cx, "头部", 6);
        let payload_out = multiline(window, cx, "载荷", 8);
        let signature_out = single(window, cx, "签名");
        let token_out = multiline(window, cx, "Token", 4);

        let mut subscriptions = Vec::new();
        for field in [
            &token, &payload, &secret, &issuer, &audience, &actor, &expiration,
        ] {
            subscriptions.push(cx.subscribe_in(
                field,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.recompute(window, cx);
                    }
                },
            ));
        }

        Self {
            mode: JwtMode::Decode,
            token,
            payload,
            secret,
            issuer,
            audience,
            actor,
            expiration,
            header_out,
            payload_out,
            signature_out,
            token_out,
            algorithm: JwtAlgorithm::Hs256,
            secret_is_base64: false,
            validate_lifetime: false,
            add_default_time_claims: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn recompute(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.mode {
            JwtMode::Decode => self.decode_now(window, cx),
            JwtMode::Encode => self.encode_now(window, cx),
        }
        cx.notify();
    }

    fn decode_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let token = self.token.read(cx).value().to_string();
        if token.trim().is_empty() {
            self.error = None;
            self.set_text(&self.header_out, "", window, cx);
            self.set_text(&self.payload_out, "", window, cx);
            self.set_text(&self.signature_out, "", window, cx);
            return;
        }
        let opts = JwtDecodeOptions {
            secret: optional_text(self.secret.read(cx).value().as_ref()),
            secret_is_base64: self.secret_is_base64,
            issuer: optional_text(self.issuer.read(cx).value().as_ref()),
            audience: optional_text(self.audience.read(cx).value().as_ref()),
            validate_lifetime: self.validate_lifetime,
            actor: optional_text(self.actor.read(cx).value().as_ref()),
        };
        match decode_jwt(&token, &opts) {
            Ok(decoded) => {
                self.error = None;
                self.set_text(&self.header_out, &decoded.header, window, cx);
                self.set_text(&self.payload_out, &decoded.payload, window, cx);
                self.set_text(&self.signature_out, &decoded.signature, window, cx);
            }
            Err(err) => {
                self.error = Some(SharedString::from(err.to_string()));
                self.set_text(&self.header_out, "", window, cx);
                self.set_text(&self.payload_out, "", window, cx);
                self.set_text(&self.signature_out, "", window, cx);
            }
        }
    }

    fn encode_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let payload = self.payload.read(cx).value().to_string();
        if payload.trim().is_empty() {
            self.error = None;
            self.set_text(&self.token_out, "", window, cx);
            return;
        }
        let expiration_secs = parse_expiration(self.expiration.read(cx).value().as_ref());
        let opts = JwtEncodeOptions {
            algorithm: self.algorithm,
            secret: self.secret.read(cx).value().to_string(),
            secret_is_base64: self.secret_is_base64,
            issuer: optional_text(self.issuer.read(cx).value().as_ref()),
            audience: optional_text(self.audience.read(cx).value().as_ref()),
            add_default_time_claims: self.add_default_time_claims,
            expiration_secs,
        };
        match encode_jwt(&payload, &opts) {
            Ok(token) => {
                self.error = None;
                self.set_text(&self.token_out, &token, window, cx);
            }
            Err(err) => {
                self.error = Some(SharedString::from(err.to_string()));
                self.set_text(&self.token_out, "", window, cx);
            }
        }
    }

    fn set_text(
        &self,
        field: &Entity<InputState>,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        field.update(cx, |input, cx| {
            input.set_value(value.to_string(), window, cx);
        });
    }

    fn set_mode(&mut self, mode: JwtMode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        self.recompute(window, cx);
    }

    fn mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: JwtMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.mode == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_mode(value, window, cx);
            }))
    }

    fn algorithm_button(&self, alg: JwtAlgorithm, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new(alg.as_str())
            .label(alg.as_str())
            .compact()
            .selected(self.algorithm == alg)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.algorithm = alg;
                this.recompute(window, cx);
            }))
    }

    fn copy_token(&mut self, cx: &mut Context<Self>) {
        if self.error.is_some() {
            return;
        }
        let text = match self.mode {
            JwtMode::Decode => self.token.read(cx).value().to_string(),
            JwtMode::Encode => self.token_out.read(cx).value().to_string(),
        };
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

fn multiline(
    window: &mut Window,
    cx: &mut Context<JwtView>,
    placeholder: &'static str,
    rows: usize,
) -> Entity<InputState> {
    cx.new(|cx| {
        InputState::new(window, cx)
            .multi_line(true)
            .rows(rows)
            .placeholder(placeholder)
    })
}

fn single(
    window: &mut Window,
    cx: &mut Context<JwtView>,
    placeholder: &'static str,
) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
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

impl ReceivesData for JwtView {
    fn on_data_received(
        &mut self,
        _payload: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

impl Render for JwtView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .flex_wrap()
                    .child(self.mode_button("jwt-decode", "解码", JwtMode::Decode, cx))
                    .child(self.mode_button("jwt-encode", "编码", JwtMode::Encode, cx))
                    .child(
                        Switch::new("jwt-b64")
                            .label("Base64 密钥")
                            .checked(self.secret_is_base64)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.secret_is_base64 = *checked;
                                this.recompute(window, cx);
                            })),
                    )
                    .child(
                        Button::new("jwt-copy")
                            .primary()
                            .label("复制")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.copy_token(cx);
                            })),
                    ),
            )
            .when_some(self.error.clone(), |this, message| {
                this.child(div().text_color(cx.theme().danger).child(message))
            })
            .child(Input::new(&self.secret))
            .child(
                h_flex()
                    .gap_2()
                    .child(Input::new(&self.issuer).flex_1())
                    .child(Input::new(&self.audience).flex_1()),
            )
            .map(|this| match self.mode {
                JwtMode::Decode => this
                    .child(
                        Switch::new("jwt-lifetime")
                            .label("校验有效期")
                            .checked(self.validate_lifetime)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.validate_lifetime = *checked;
                                this.recompute(window, cx);
                            })),
                    )
                    .child(Input::new(&self.actor))
                    .child(
                        v_flex()
                            .gap_1()
                            .child("Token")
                            .child(Input::new(&self.token)),
                    )
                    .child(
                gpui::div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .gap_3()
                    .min_h_0()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .min_h_0()
                                    .child("头部")
                                    .child(Input::new(&self.header_out).h_full().disabled(true)),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .min_h_0()
                                    .child("载荷")
                                    .child(Input::new(&self.payload_out).h_full().disabled(true)),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .min_h_0()
                                    .child("签名")
                                    .child(Input::new(&self.signature_out).h_full().disabled(true)),
                            ),
                    ),
                JwtMode::Encode => this
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .flex_wrap()
                            .children(JwtAlgorithm::ALL.map(|alg| self.algorithm_button(alg, cx))),
                    )
                    .child(
                        Switch::new("jwt-time")
                            .label("默认时间声明")
                            .checked(self.add_default_time_claims)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.add_default_time_claims = *checked;
                                this.recompute(window, cx);
                            })),
                    )
                    .child(Input::new(&self.expiration))
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .min_h_0()
                            .child("载荷")
                            .child(Input::new(&self.payload).h_full()),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child("Token")
                            .child(Input::new(&self.token_out).disabled(true)),
                    ),
            })
    }
}
