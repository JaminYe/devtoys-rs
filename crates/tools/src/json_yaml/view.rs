use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::indent::Indentation;
use crate::slot::ReceivesData;
use super::{convert_json_yaml, looks_like_json, looks_like_yaml, Conversion};

pub struct JsonYamlView {
    input: Entity<InputState>,
    output: Entity<InputState>,
    direction: Conversion,
    indent: Indentation,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl JsonYamlView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("粘贴 JSON 或 YAML")
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("转换结果")
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
            direction: Conversion::JsonToYaml,
            indent: Indentation::TwoSpaces,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn reconvert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }
        match convert_json_yaml(&source, self.direction, self.indent) {
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

    fn set_direction(
        &mut self,
        direction: Conversion,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.direction = direction;
        self.reconvert(window, cx);
    }

    fn set_indent(&mut self, indent: Indentation, window: &mut Window, cx: &mut Context<Self>) {
        self.indent = indent;
        self.reconvert(window, cx);
    }

    fn option_button(
        &self,
        id: &'static str,
        label: &'static str,
        selected: bool,
        on_pick: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(selected)
            .on_click(cx.listener(move |this, _, window, cx| {
                on_pick(this, window, cx);
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

impl ReceivesData for JsonYamlView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if looks_like_json(payload) {
            self.direction = Conversion::JsonToYaml;
        } else if looks_like_yaml(payload) {
            self.direction = Conversion::YamlToJson;
        }
        self.input.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.reconvert(window, cx);
    }
}

impl Render for JsonYamlView {
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
                    .child(self.option_button(
                        "dir-json-yaml",
                        "JSON → YAML",
                        self.direction == Conversion::JsonToYaml,
                        |this, window, cx| this.set_direction(Conversion::JsonToYaml, window, cx),
                        cx,
                    ))
                    .child(self.option_button(
                        "dir-yaml-json",
                        "YAML → JSON",
                        self.direction == Conversion::YamlToJson,
                        |this, window, cx| this.set_direction(Conversion::YamlToJson, window, cx),
                        cx,
                    ))
                    .child(self.option_button(
                        "indent-two",
                        "两空格",
                        self.indent == Indentation::TwoSpaces,
                        |this, window, cx| this.set_indent(Indentation::TwoSpaces, window, cx),
                        cx,
                    ))
                    .child(self.option_button(
                        "indent-four",
                        "四空格",
                        self.indent == Indentation::FourSpaces,
                        |this, window, cx| this.set_indent(Indentation::FourSpaces, window, cx),
                        cx,
                    ))
                    .child(self.option_button(
                        "indent-tab",
                        "Tab",
                        self.indent == Indentation::OneTab,
                        |this, window, cx| this.set_indent(Indentation::OneTab, window, cx),
                        cx,
                    ))
                    .child(self.option_button(
                        "indent-min",
                        "压缩",
                        self.indent == Indentation::Minified,
                        |this, window, cx| this.set_indent(Indentation::Minified, window, cx),
                        cx,
                    ))
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
