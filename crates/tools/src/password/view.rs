use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme};

use crate::slot::ReceivesData;
use super::{generate_password, PasswordOptions};

pub struct PasswordView {
    length_input: Entity<InputState>,
    count_input: Entity<InputState>,
    exclude: Entity<InputState>,
    output: Entity<InputState>,
    uppercase: bool,
    lowercase: bool,
    digits: bool,
    special: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl PasswordView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let length_input = cx.new(|cx| InputState::new(window, cx).placeholder("长度"));
        length_input.update(cx, |input, cx| {
            input.set_value("30".to_string(), window, cx);
        });
        let count_input = cx.new(|cx| InputState::new(window, cx).placeholder("数量"));
        count_input.update(cx, |input, cx| {
            input.set_value("1".to_string(), window, cx);
        });
        let exclude = cx.new(|cx| InputState::new(window, cx).placeholder("排除字符"));
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(8)
                .placeholder("生成结果")
        });
        let mut subscriptions = Vec::new();
        for source in [&length_input, &count_input, &exclude] {
            subscriptions.push(cx.subscribe_in(
                source,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.regenerate(window, cx);
                    }
                },
            ));
        }
        let mut this = Self {
            length_input,
            count_input,
            exclude,
            output,
            uppercase: true,
            lowercase: true,
            digits: true,
            special: true,
            error: None,
            _subscriptions: subscriptions,
        };
        this.regenerate(window, cx);
        this
    }

    fn options(&self, cx: &Context<Self>) -> PasswordOptions {
        PasswordOptions {
            length: self
                .length_input
                .read(cx)
                .value()
                .parse::<usize>()
                .unwrap_or(30),
            uppercase: self.uppercase,
            lowercase: self.lowercase,
            digits: self.digits,
            special: self.special,
            exclude: self.exclude.read(cx).value().to_string(),
            count: self
                .count_input
                .read(cx)
                .value()
                .parse::<usize>()
                .unwrap_or(1)
                .max(1),
        }
    }

    fn regenerate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let options = self.options(cx);
        match generate_password(&options, &mut rand::thread_rng()) {
            Ok(text) => {
                self.error = None;
                self.output.update(cx, |output, cx| {
                    output.set_value(text, window, cx);
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

impl ReceivesData for PasswordView {
    fn on_data_received(&mut self, _: &str, _: &mut Window, _: &mut Context<Self>) {}
}

impl Render for PasswordView {
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
                        Switch::new("pwd-upper")
                            .label("大写")
                            .checked(self.uppercase)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.uppercase = *checked;
                                this.regenerate(window, cx);
                            })),
                    )
                    .child(
                        Switch::new("pwd-lower")
                            .label("小写")
                            .checked(self.lowercase)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.lowercase = *checked;
                                this.regenerate(window, cx);
                            })),
                    )
                    .child(
                        Switch::new("pwd-digits")
                            .label("数字")
                            .checked(self.digits)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.digits = *checked;
                                this.regenerate(window, cx);
                            })),
                    )
                    .child(
                        Switch::new("pwd-special")
                            .label("特殊字符")
                            .checked(self.special)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.special = *checked;
                                this.regenerate(window, cx);
                            })),
                    )
                    .child(div().child("长度"))
                    .child(Input::new(&self.length_input).w_20())
                    .child(div().child("数量"))
                    .child(Input::new(&self.count_input).w_20())
                    .child(
                        Button::new("pwd-generate")
                            .label("生成")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.regenerate(window, cx);
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
            .child(
                v_flex()
                    .gap_1()
                    .child("排除字符")
                    .child(Input::new(&self.exclude)),
            )
            .when_some(self.error.clone(), |this, message| {
                this.child(div().text_color(cx.theme().danger).child(message))
            })
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .min_h_0()
                    .child("输出")
                    .child(Input::new(&self.output).h_full().disabled(true)),
            )
    }
}
