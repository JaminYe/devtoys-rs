use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, Selectable};

use crate::slot::ReceivesData;
use super::{format_sql, Indentation, SqlLanguage};

pub struct SqlFormatterView {
    input: Entity<InputState>,
    output: Entity<InputState>,
    indent: Indentation,
    language: SqlLanguage,
    leading_comma: bool,
    _subscriptions: Vec<Subscription>,
}

impl SqlFormatterView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("粘贴 SQL")
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
            language: SqlLanguage::Sql,
            leading_comma: false,
            _subscriptions: subscriptions,
        }
    }

    fn reformat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }

        let formatted = format_sql(&source, self.indent, self.language, self.leading_comma);
        self.output.update(cx, |output, cx| {
            output.set_value(formatted, window, cx);
        });
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

    fn set_language(
        &mut self,
        language: SqlLanguage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.language = language;
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

    fn language_button(&self, language: SqlLanguage, cx: &mut Context<Self>) -> impl IntoElement {
        let id = language.as_str();
        Button::new(id)
            .label(id)
            .compact()
            .selected(self.language == language)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_language(language, window, cx);
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

impl ReceivesData for SqlFormatterView {
    fn on_data_received(
        &mut self,
        _payload: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

impl Render for SqlFormatterView {
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
                        Switch::new("leading-comma")
                            .label("前导逗号")
                            .checked(self.leading_comma)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.leading_comma = *checked;
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
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .flex_wrap()
                    .child(div().child(SharedString::from("方言")))
                    .children(SqlLanguage::ALL.into_iter().map(|lang| self.language_button(lang, cx))),
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
