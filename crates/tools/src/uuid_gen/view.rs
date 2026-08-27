use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{generate_uuid, UuidOptions, UuidVersion};

pub struct UuidGenView {
    count_input: Entity<InputState>,
    output: Entity<InputState>,
    version: UuidVersion,
    hyphens: bool,
    uppercase: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl UuidGenView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let count_input = cx.new(|cx| InputState::new(window, cx).placeholder("数量"));
        count_input.update(cx, |input, cx| {
            input.set_value("1".to_string(), window, cx);
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(8)
                .placeholder("生成结果")
        });
        let subscriptions = vec![cx.subscribe_in(
            &count_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.regenerate(window, cx);
                }
            },
        )];
        let mut this = Self {
            count_input,
            output,
            version: UuidVersion::Four,
            hyphens: true,
            uppercase: false,
            error: None,
            _subscriptions: subscriptions,
        };
        this.regenerate(window, cx);
        this
    }

    fn options(&self, cx: &Context<Self>) -> UuidOptions {
        UuidOptions {
            version: self.version,
            hyphens: self.hyphens,
            uppercase: self.uppercase,
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
        match generate_uuid(&self.options(cx)) {
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

    fn version_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: UuidVersion,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.version == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.version = value;
                this.regenerate(window, cx);
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

impl ReceivesData for UuidGenView {
    fn on_data_received(&mut self, _: &str, _: &mut Window, _: &mut Context<Self>) {}
}

impl Render for UuidGenView {
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
                    .child(self.version_button("uuid-v1", "v1", UuidVersion::One, cx))
                    .child(self.version_button("uuid-v4", "v4", UuidVersion::Four, cx))
                    .child(self.version_button("uuid-v7", "v7", UuidVersion::Seven, cx))
                    .child(
                        Switch::new("uuid-hyphens")
                            .label("连字符")
                            .checked(self.hyphens)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.hyphens = *checked;
                                this.regenerate(window, cx);
                            })),
                    )
                    .child(
                        Switch::new("uuid-uppercase")
                            .label("大写")
                            .checked(self.uppercase)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.uppercase = *checked;
                                this.regenerate(window, cx);
                            })),
                    )
                    .child(div().child("数量"))
                    .child(Input::new(&self.count_input).w_20())
                    .child(
                        Button::new("uuid-generate")
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
