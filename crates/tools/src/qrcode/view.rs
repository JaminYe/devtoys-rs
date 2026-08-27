use std::path::Path;

use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;

use super::{decode_image_path, encode_svg, is_existing_image_file};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Conversion {
    #[default]
    Encode,
    Decode,
}

pub struct QrcodeView {
    input: Entity<InputState>,
    output: Entity<InputState>,
    conversion: Conversion,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl QrcodeView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(8)
                .placeholder("文本或图像路径")
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("SVG 或解码文本")
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
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn recompute(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }

        let result = match self.conversion {
            Conversion::Encode => encode_svg(&source),
            Conversion::Decode => decode_image_path(Path::new(source.trim())),
        };

        match result {
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

impl ReceivesData for QrcodeView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.conversion = if is_existing_image_file(payload) {
            Conversion::Decode
        } else {
            Conversion::Encode
        };
        self.input.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.recompute(window, cx);
    }
}

impl Render for QrcodeView {
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
                    .child(self.conversion_button("qr-encode", "编码", Conversion::Encode, cx))
                    .child(self.conversion_button("qr-decode", "解码", Conversion::Decode, cx))
                    .child(
                        Button::new("qr-copy")
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
