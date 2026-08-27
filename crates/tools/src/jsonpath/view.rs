use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme};

use crate::slot::ReceivesData;
use super::helper::CHEAT_SHEET;
use super::eval_jsonpath;

pub struct JsonPathView {
    json: Entity<InputState>,
    path: Entity<InputState>,
    output: Entity<InputState>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl JsonPathView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let json = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("粘贴 JSON")
        });
        let path = cx.new(|cx| InputState::new(window, cx).placeholder("$.path"));
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("匹配结果")
        });

        let subscriptions = vec![
            cx.subscribe_in(&json, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reeval(window, cx);
                }
            }),
            cx.subscribe_in(&path, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reeval(window, cx);
                }
            }),
        ];

        Self {
            json,
            path,
            output,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn reeval(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let json = self.json.read(cx).value().to_string();
        let path = self.path.read(cx).value().to_string();
        if json.trim().is_empty() || path.trim().is_empty() {
            self.error = None;
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }

        match eval_jsonpath(&json, &path) {
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

impl ReceivesData for JsonPathView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.json.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.reeval(window, cx);
    }
}

impl Render for JsonPathView {
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
                            .child("JSON")
                            .child(Input::new(&self.json).h_full()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_2()
                            .min_h_0()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child("JSONPath")
                                    .child(Input::new(&self.path)),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .min_h_0()
                                    .child("匹配结果")
                                    .child(Input::new(&self.output).h_full().disabled(true)),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child("速查表")
                    .child(
                        h_flex()
                            .gap_3()
                            .flex_wrap()
                            .children(CHEAT_SHEET.iter().map(|(syntax, desc)| {
                                div().child(format!("{syntax}  {desc}"))
                            })),
                    ),
            )
    }
}
