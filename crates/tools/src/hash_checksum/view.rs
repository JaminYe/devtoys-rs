use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{checksum_matches, compute_hash, HashAlgorithm};

pub struct HashChecksumView {
    input: Entity<InputState>,
    hmac: Entity<InputState>,
    expected: Entity<InputState>,
    output: Entity<InputState>,
    algorithm: HashAlgorithm,
    uppercase: bool,
    match_state: Option<bool>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl HashChecksumView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(10)
                .placeholder("输入文本")
        });
        let hmac = cx.new(|cx| InputState::new(window, cx).placeholder("HMAC 密钥（可选）"));
        let expected = cx.new(|cx| InputState::new(window, cx).placeholder("期望校验和"));
        let output = cx.new(|cx| InputState::new(window, cx).placeholder("哈希结果"));

        let mut subscriptions = Vec::new();
        for source in [&input, &hmac, &expected] {
            subscriptions.push(cx.subscribe_in(
                source,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.recompute(window, cx);
                    }
                },
            ));
        }

        Self {
            input,
            hmac,
            expected,
            output,
            algorithm: HashAlgorithm::Md5,
            uppercase: false,
            match_state: None,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn recompute(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        let hmac_raw = self.hmac.read(cx).value().to_string();
        let hmac = if hmac_raw.is_empty() {
            None
        } else {
            Some(hmac_raw.as_str())
        };
        match compute_hash(&source, self.algorithm, hmac, self.uppercase, false) {
            Ok(hex) => {
                self.error = None;
                let expected = self.expected.read(cx).value().to_string();
                self.match_state = if expected.trim().is_empty() {
                    None
                } else {
                    Some(checksum_matches(&hex, &expected))
                };
                self.output.update(cx, |output, cx| {
                    output.set_value(hex, window, cx);
                });
            }
            Err(err) => {
                self.error = Some(SharedString::from(err.to_string()));
                self.match_state = None;
                self.output.update(cx, |output, cx| {
                    output.set_value(String::new(), window, cx);
                });
            }
        }
        cx.notify();
    }

    fn algo_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: HashAlgorithm,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.algorithm == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.algorithm = value;
                this.recompute(window, cx);
            }))
    }

    fn copy_output(&mut self, cx: &mut Context<Self>) {
        if self.error.is_some() {
            return;
        }
        let text = self.output.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

impl ReceivesData for HashChecksumView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.recompute(window, cx);
    }
}

impl Render for HashChecksumView {
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
                    .child(self.algo_button("algo-md5", "MD5", HashAlgorithm::Md5, cx))
                    .child(self.algo_button("algo-sha1", "SHA1", HashAlgorithm::Sha1, cx))
                    .child(self.algo_button("algo-sha256", "SHA256", HashAlgorithm::Sha256, cx))
                    .child(self.algo_button("algo-sha384", "SHA384", HashAlgorithm::Sha384, cx))
                    .child(self.algo_button("algo-sha512", "SHA512", HashAlgorithm::Sha512, cx))
                    .child(
                        Switch::new("hash-uppercase")
                            .label("大写")
                            .checked(self.uppercase)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.uppercase = *checked;
                                this.recompute(window, cx);
                            })),
                    )
                    .child(
                        Button::new("copy-output")
                            .primary()
                            .label("复制")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.copy_output(cx);
                            })),
                    ),
            )
            .when_some(self.error.clone(), |this, message| {
                this.child(div().text_color(cx.theme().danger).child(message))
            })
            .when_some(self.match_state, |this, matched| {
                this.child(
                    div()
                        .text_color(if matched {
                            cx.theme().success
                        } else {
                            cx.theme().danger
                        })
                        .child(if matched { "匹配" } else { "不匹配" }),
                )
            })
            .child(
                h_flex()
                    .gap_3()
                    .child(v_flex().flex_1().gap_1().child("HMAC 密钥").child(Input::new(&self.hmac)))
                    .child(v_flex().flex_1().gap_1().child("期望校验和").child(Input::new(&self.expected))),
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
                            .child("输入")
                            .child(Input::new(&self.input).h_full()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .min_h_0()
                            .child("输出")
                            .child(Input::new(&self.output).h_full().disabled(true)),
                    ),
            )
    }
}
