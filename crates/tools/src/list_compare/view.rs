use gpui::{prelude::*, ClipboardItem, Context, Entity, Subscription, Window};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, Selectable};

use crate::slot::ReceivesData;
use super::{compare_lists, ListMode};

pub struct ListCompareView {
    list_a: Entity<InputState>,
    list_b: Entity<InputState>,
    output: Entity<InputState>,
    mode: ListMode,
    case_sensitive: bool,
    _subscriptions: Vec<Subscription>,
}

impl ListCompareView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let list_a = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(10)
                .placeholder("列表 A，一行一项")
        });
        let list_b = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(10)
                .placeholder("列表 B，一行一项")
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(10)
                .placeholder("比对结果")
        });

        let mut subscriptions = Vec::new();
        for source in [&list_a, &list_b] {
            subscriptions.push(cx.subscribe_in(
                source,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.recompute(window, cx);
                    }
                },
            ));
        }

        Self {
            list_a,
            list_b,
            output,
            mode: ListMode::AInterB,
            case_sensitive: false,
            _subscriptions: subscriptions,
        }
    }

    fn recompute(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let a = self.list_a.read(cx).value().to_string();
        let b = self.list_b.read(cx).value().to_string();
        let result = compare_lists(&a, &b, self.mode, self.case_sensitive);
        self.output.update(cx, |output, cx| {
            output.set_value(result, window, cx);
        });
        cx.notify();
    }

    fn set_mode(&mut self, mode: ListMode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        self.recompute(window, cx);
    }

    fn mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: ListMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.mode == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_mode(value, window, cx);
            }))
    }

    fn copy_output(&mut self, cx: &mut Context<Self>) {
        let text = self.output.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

impl ReceivesData for ListCompareView {
    fn on_data_received(
        &mut self,
        _payload: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

impl Render for ListCompareView {
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
                    .child(self.mode_button("inter", "交集", ListMode::AInterB, cx))
                    .child(self.mode_button("union", "并集", ListMode::AUnionB, cx))
                    .child(self.mode_button("a-only", "仅 A", ListMode::AOnly, cx))
                    .child(self.mode_button("b-only", "仅 B", ListMode::BOnly, cx))
                    .child(
                        Switch::new("case-sensitive")
                            .label("大小写敏感")
                            .checked(self.case_sensitive)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.case_sensitive = *checked;
                                this.recompute(window, cx);
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
                            .child("列表 A")
                            .child(Input::new(&self.list_a).h_full()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .min_h_0()
                            .child("列表 B")
                            .child(Input::new(&self.list_b).h_full()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .min_h_0()
                            .child("结果")
                            .child(Input::new(&self.output).h_full().disabled(true)),
                    ),
            )
    }
}
