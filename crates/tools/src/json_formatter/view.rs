use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{format_json, Indentation};

pub struct JsonFormatterView {
    input: Entity<InputState>,
    output: Entity<InputState>,
    indent: Indentation,
    sort_properties: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl JsonFormatterView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("粘贴 JSON")
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("格式化结果")
        });

        let subscriptions = vec![cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reformat(window, cx);
                }
            },
        )];

        Self {
            input,
            output,
            indent: Indentation::TwoSpaces,
            sort_properties: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn reformat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }

        match format_json(&source, self.indent, self.sort_properties) {
            Ok(formatted) => {
                self.error = None;
                self.output.update(cx, |output, cx| {
                    output.set_value(formatted, window, cx);
                });
            }
            Err(_) => {
                self.error = Some(SharedString::from("非法 JSON"));
                self.output.update(cx, |output, cx| {
                    output.set_value(String::new(), window, cx);
                });
            }
        }
        cx.notify();
    }

    fn set_indent(
        &mut self,
        indent: Indentation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.indent = indent;
        self.reformat(window, cx);
    }

    fn indent_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: Indentation,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.indent == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_indent(value, window, cx);
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

impl ReceivesData for JsonFormatterView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.reformat(window, cx);
    }
}

impl Render for JsonFormatterView {
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
                    .child(self.indent_button("indent-two", "两空格", Indentation::TwoSpaces, cx))
                    .child(self.indent_button("indent-four", "四空格", Indentation::FourSpaces, cx))
                    .child(self.indent_button("indent-tab", "Tab", Indentation::OneTab, cx))
                    .child(self.indent_button("indent-min", "压缩", Indentation::Minified, cx))
                    .child(
                        Switch::new("sort-properties")
                            .label("按属性名排序")
                            .checked(self.sort_properties)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.sort_properties = *checked;
                                this.reformat(window, cx);
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
