use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme};

use crate::slot::ReceivesData;
use super::helper::{test_regex, RegexMatch, RegexOptions, CHEAT_SHEET};

pub struct RegexTesterView {
    pattern: Entity<InputState>,
    sample: Entity<InputState>,
    output: Entity<InputState>,
    options: RegexOptions,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl RegexTesterView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let pattern = cx.new(|cx| InputState::new(window, cx).placeholder("正则表达式"));
        let sample = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("样例文本")
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("匹配与分组")
        });

        let subscriptions = vec![
            cx.subscribe_in(
                &pattern,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.rematch(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &sample,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.rematch(window, cx);
                    }
                },
            ),
        ];

        Self {
            pattern,
            sample,
            output,
            options: RegexOptions::default(),
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn rematch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pattern = self.pattern.read(cx).value().to_string();
        let sample = self.sample.read(cx).value().to_string();
        if pattern.trim().is_empty() {
            self.error = None;
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }

        match test_regex(&pattern, &sample, &self.options) {
            Ok(matches) => {
                self.error = None;
                self.output.update(cx, |output, cx| {
                    output.set_value(format_matches(&matches), window, cx);
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

    fn option_switch(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        set: fn(&mut RegexOptions, bool),
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Switch::new(id)
            .label(label)
            .checked(checked)
            .on_click(cx.listener(move |this, checked, window, cx| {
                set(&mut this.options, *checked);
                this.rematch(window, cx);
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

fn format_matches(matches: &[RegexMatch]) -> String {
    if matches.is_empty() {
        return "无匹配".into();
    }
    let mut lines = Vec::new();
    for m in matches {
        lines.push(format!(
            "匹配 {}  [{}-{}]  {}",
            m.index + 1,
            m.start,
            m.end,
            m.value
        ));
        for g in &m.groups {
            let label = match &g.name {
                Some(name) => format!("分组 {name}"),
                None => format!("分组 {}", g.index),
            };
            lines.push(format!(
                "  {label}  [{}-{}]  {}",
                g.start, g.end, g.value
            ));
        }
    }
    lines.join("\n")
}

impl ReceivesData for RegexTesterView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sample.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.rematch(window, cx);
    }
}

impl Render for RegexTesterView {
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
                    .child(self.option_switch(
                        "all-matches",
                        "全部匹配",
                        self.options.all_matches,
                        |o, v| o.all_matches = v,
                        cx,
                    ))
                    .child(self.option_switch(
                        "ignore-case",
                        "忽略大小写",
                        self.options.ignore_case,
                        |o, v| o.ignore_case = v,
                        cx,
                    ))
                    .child(self.option_switch(
                        "ignore-ws",
                        "忽略空白",
                        self.options.ignore_whitespace,
                        |o, v| o.ignore_whitespace = v,
                        cx,
                    ))
                    .child(self.option_switch(
                        "singleline",
                        "Singleline",
                        self.options.singleline,
                        |o, v| o.singleline = v,
                        cx,
                    ))
                    .child(self.option_switch(
                        "multiline",
                        "Multiline",
                        self.options.multiline,
                        |o, v| o.multiline = v,
                        cx,
                    ))
                    .child(self.option_switch(
                        "ecmascript",
                        "ECMAScript",
                        self.options.ecmascript,
                        |o, v| o.ecmascript = v,
                        cx,
                    ))
                    .child(self.option_switch(
                        "culture",
                        "区域固定",
                        self.options.culture_invariant,
                        |o, v| o.culture_invariant = v,
                        cx,
                    ))
                    .child(self.option_switch(
                        "rtl",
                        "从右向左",
                        self.options.right_to_left,
                        |o, v| o.right_to_left = v,
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
                            .gap_2()
                            .min_h_0()
                            .child(v_flex().gap_1().child("表达式").child(Input::new(&self.pattern)))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .min_h_0()
                                    .child("样例文本")
                                    .child(Input::new(&self.sample).h_full()),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_2()
                            .min_h_0()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .min_h_0()
                                    .child("匹配结果")
                                    .child(Input::new(&self.output).h_full().disabled(true)),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child("速查表")
                                    .child(
                                        h_flex().gap_3().flex_wrap().children(
                                            CHEAT_SHEET.iter().map(|(syntax, desc)| {
                                                div().child(format!("{syntax}  {desc}"))
                                            }),
                                        ),
                                    ),
                            ),
                    ),
            )
    }
}
