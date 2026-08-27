use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{convert, Conversion};

pub struct EscapeUnescapeView {
    input: Entity<InputState>,
    output: Entity<InputState>,
    conversion: Conversion,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl EscapeUnescapeView {
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
                .placeholder("结果")
        });

        let subscriptions = vec![cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reconvert(window, cx);
                }
            },
        )];

        Self {
            input,
            output,
            conversion: Conversion::Encode,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn reconvert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        match convert(&source, self.conversion) {
            Ok(result) => {
                self.error = None;
                self.output.update(cx, |output, cx| {
                    output.set_value(result, window, cx);
                });
            }
            Err(_) => {
                self.error = Some(SharedString::from("非法转义序列"));
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
        self.reconvert(window, cx);
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

impl ReceivesData for EscapeUnescapeView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.conversion = Conversion::Decode;
        self.input.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.reconvert(window, cx);
    }
}

impl Render for EscapeUnescapeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("enc")
                            .label("转义")
                            .compact()
                            .selected(self.conversion == Conversion::Encode)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.set_conversion(Conversion::Encode, window, cx);
                            })),
                    )
                    .child(
                        Button::new("dec")
                            .label("反转义")
                            .compact()
                            .selected(self.conversion == Conversion::Decode)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.set_conversion(Conversion::Decode, window, cx);
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
