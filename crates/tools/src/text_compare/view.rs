use gpui::{div, prelude::*, Context, Entity, Subscription, Window};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{diff_lines, DiffMode, DiffTag};

pub struct TextCompareView {
    left: Entity<InputState>,
    right: Entity<InputState>,
    mode: DiffMode,
    _subscriptions: Vec<Subscription>,
}

impl TextCompareView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let left = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(10)
                .placeholder("左侧文本")
        });
        let right = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(10)
                .placeholder("右侧文本")
        });

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe_in(&left, window, |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        }));
        subscriptions.push(cx.subscribe_in(&right, window, |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        }));

        Self {
            left,
            right,
            mode: DiffMode::SideBySide,
            _subscriptions: subscriptions,
        }
    }

    fn mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: DiffMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.mode == value)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.mode = value;
                cx.notify();
            }))
    }
}

impl ReceivesData for TextCompareView {
    fn on_data_received(
        &mut self,
        _payload: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

impl Render for TextCompareView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let left = self.left.read(cx).value().to_string();
        let right = self.right.read(cx).value().to_string();
        let hunks = diff_lines(&left, &right);
        let inline = self.mode == DiffMode::Inline;

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(self.mode_button("mode-side", "并排", DiffMode::SideBySide, cx))
                    .child(self.mode_button("mode-inline", "行内", DiffMode::Inline, cx)),
            )
            .child(
                h_flex()
                    .gap_3()
                    .h(gpui::px(200.))
                    .min_h_0()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .min_h_0()
                            .child("左侧")
                            .child(Input::new(&self.left).h_full()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .min_h_0()
                            .child("右侧")
                            .child(Input::new(&self.right).h_full()),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .min_h_0()
                    .child("差异")
                    .child(if inline {
                        v_flex()
                            .gap_1()
                            .children(hunks.iter().map(|hunk| {
                                let color = match hunk.tag {
                                    DiffTag::Delete => cx.theme().danger,
                                    DiffTag::Insert => cx.theme().success,
                                    DiffTag::Equal => cx.theme().foreground,
                                };
                                let prefix = match hunk.tag {
                                    DiffTag::Delete => "- ",
                                    DiffTag::Insert => "+ ",
                                    DiffTag::Equal => "  ",
                                };
                                div()
                                    .text_color(color)
                                    .child(format!("{prefix}{}", hunk.text.trim_end()))
                            }))
                            .into_any_element()
                    } else {
                        h_flex()
                            .gap_3()
                            .flex_1()
                            .child(
                                v_flex().flex_1().gap_1().children(hunks.iter().filter_map(|hunk| {
                                    match hunk.tag {
                                        DiffTag::Insert => None,
                                        DiffTag::Delete => Some(
                                            div()
                                                .text_color(cx.theme().danger)
                                                .child(hunk.text.trim_end().to_string()),
                                        ),
                                        DiffTag::Equal => Some(
                                            div().child(hunk.text.trim_end().to_string()),
                                        ),
                                    }
                                })),
                            )
                            .child(
                                v_flex().flex_1().gap_1().children(hunks.iter().filter_map(|hunk| {
                                    match hunk.tag {
                                        DiffTag::Delete => None,
                                        DiffTag::Insert => Some(
                                            div()
                                                .text_color(cx.theme().success)
                                                .child(hunk.text.trim_end().to_string()),
                                        ),
                                        DiffTag::Equal => Some(
                                            div().child(hunk.text.trim_end().to_string()),
                                        ),
                                    }
                                })),
                            )
                            .into_any_element()
                    }),
            )
    }
}
