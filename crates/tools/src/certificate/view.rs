use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme};

use crate::slot::ReceivesData;
use super::decode_certificate;

pub struct CertificateView {
    input: Entity<InputState>,
    password: Entity<InputState>,
    output: Entity<InputState>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl CertificateView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("粘贴 PEM / CER / CRT")
        });
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("PFX 密码（可选）")
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("证书信息")
        });

        let mut subscriptions = vec![cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.redecode(window, cx);
                }
            },
        )];
        subscriptions.push(cx.subscribe_in(
            &password,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.redecode(window, cx);
                }
            },
        ));

        Self {
            input,
            password,
            output,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn redecode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }
        let password = self.password.read(cx).value().to_string();
        let password = if password.is_empty() {
            None
        } else {
            Some(password)
        };
        match decode_certificate(source.as_bytes(), password.as_deref()) {
            Ok(result) => {
                self.error = None;
                self.output.update(cx, |output, cx| {
                    output.set_value(result, window, cx);
                });
            }
            Err(err) => {
                self.error = Some(SharedString::from(err.to_string()));
                self.output.update(cx, |output, cx| {
                    output.set_value(String::new(), window, cx);
                });
            }
        }
        cx.notify();
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

impl ReceivesData for CertificateView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.redecode(window, cx);
    }
}

impl Render for CertificateView {
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
                    .child(
                        v_flex()
                            .w_64()
                            .child(Input::new(&self.password)),
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
                this.child(
                    div()
                        .text_color(cx.theme().danger)
                        .child(message),
                )
            })
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
