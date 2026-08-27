use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;

use super::{convert, Conversion};

pub struct UrlView {
    input: Entity<InputState>,
    output: Entity<InputState>,
    conversion: Conversion,
    multiline: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl UrlView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("粘贴文本")
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("编解码结果")
        });

        let subscriptions = vec![cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.recompute(window, cx);
                }
            },
        )];

        Self {
            input,
            output,
            conversion: Conversion::Encode,
            multiline: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn recompute(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        if source.is_empty() {
            self.error = None;
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }

        match convert(&source, self.conversion, self.multiline) {
            Ok(formatted) => {
                self.error = None;
                self.output.update(cx, |output, cx| {
                    output.set_value(formatted, window, cx);
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

    fn set_conversion(
        &mut self,
        conversion: Conversion,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.conversion = conversion;
        self.recompute(window, cx);
    }

    fn conversion_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: Conversion,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.conversion == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_conversion(value, window, cx);
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

impl ReceivesData for UrlView {
    fn on_data_received(
        &mut self,
        _payload: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

impl Render for UrlView {
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
                    .child(self.conversion_button("url-encode", "编码", Conversion::Encode, cx))
                    .child(self.conversion_button("url-decode", "解码", Conversion::Decode, cx))
                    .child(
                        Switch::new("url-multiline")
                            .label("多行")
                            .checked(self.multiline)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.multiline = *checked;
                                this.recompute(window, cx);
                            })),
                    )
                    .child(
                        Button::new("url-copy")
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
