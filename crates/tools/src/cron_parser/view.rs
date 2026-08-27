use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{
    parse_cron, DEFAULT_DATE_FORMAT, DEFAULT_EXPR_WITHOUT_SECONDS, DEFAULT_EXPR_WITH_SECONDS,
};

pub struct CronParserView {
    expression: Entity<InputState>,
    date_format: Entity<InputState>,
    output: Entity<InputState>,
    include_seconds: bool,
    count: usize,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl CronParserView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let expression = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(DEFAULT_EXPR_WITH_SECONDS)
        });
        expression.update(cx, |input, cx| {
            input.set_value(DEFAULT_EXPR_WITH_SECONDS.to_string(), window, cx);
        });
        let date_format = cx.new(|cx| {
            InputState::new(window, cx).placeholder(DEFAULT_DATE_FORMAT)
        });
        date_format.update(cx, |input, cx| {
            input.set_value(DEFAULT_DATE_FORMAT.to_string(), window, cx);
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("解析结果")
        });

        let subscriptions = vec![
            cx.subscribe_in(&expression, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reparse(window, cx);
                }
            }),
            cx.subscribe_in(&date_format, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reparse(window, cx);
                }
            }),
        ];

        let mut view = Self {
            expression,
            date_format,
            output,
            include_seconds: true,
            count: 5,
            error: None,
            _subscriptions: subscriptions,
        };
        view.reparse(window, cx);
        view
    }

    fn reparse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.expression.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }
        let format = self.date_format.read(cx).value().to_string();
        match parse_cron(&source, self.include_seconds, self.count, &format) {
            Ok(result) => {
                self.error = None;
                self.output.update(cx, |output, cx| {
                    output.set_value(result.display_text(), window, cx);
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

    fn set_count(&mut self, count: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.count = count;
        self.reparse(window, cx);
    }

    fn count_button(
        &self,
        id: &'static str,
        count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(count.to_string())
            .compact()
            .selected(self.count == count)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_count(count, window, cx);
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

impl ReceivesData for CronParserView {
    fn on_data_received(
        &mut self,
        _payload: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

impl Render for CronParserView {
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
                        Switch::new("include-seconds")
                            .label("包含秒字段")
                            .checked(self.include_seconds)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.include_seconds = *checked;
                                let current = this.expression.read(cx).value().to_string();
                                let next = if *checked {
                                    if current.trim() == DEFAULT_EXPR_WITHOUT_SECONDS {
                                        DEFAULT_EXPR_WITH_SECONDS
                                    } else {
                                        current.as_str()
                                    }
                                } else if current.trim() == DEFAULT_EXPR_WITH_SECONDS {
                                    DEFAULT_EXPR_WITHOUT_SECONDS
                                } else {
                                    current.as_str()
                                };
                                if next != current {
                                    this.expression.update(cx, |input, cx| {
                                        input.set_value(next.to_string(), window, cx);
                                    });
                                }
                                this.reparse(window, cx);
                            })),
                    )
                    .child("预览条数")
                    .child(self.count_button("count-5", 5, cx))
                    .child(self.count_button("count-10", 10, cx))
                    .child(self.count_button("count-25", 25, cx))
                    .child(self.count_button("count-50", 50, cx))
                    .child(self.count_button("count-100", 100, cx))
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
                h_flex()
                    .gap_2()
                    .items_center()
                    .child("日期格式")
                    .child(Input::new(&self.date_format).w_80()),
            )
            .when_some(self.error.clone(), |this, message| {
                this.child(div().text_color(cx.theme().danger).child(message))
            })
            .child(
                v_flex()
                    .gap_1()
                    .child("表达式")
                    .child(Input::new(&self.expression)),
            )
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
