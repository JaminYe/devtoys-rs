use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{
    add_thousands_separators, convert_base, convert_rfc4648, decode_custom, encode_custom,
    encode_rfc4648, NumberBase, Rfc4648Encoding,
};

pub struct NumberBaseView {
    decimal: Entity<InputState>,
    hexadecimal: Entity<InputState>,
    octal: Entity<InputState>,
    binary: Entity<InputState>,
    advanced_input: Entity<InputState>,
    advanced_output: Entity<InputState>,
    custom_alphabet: Entity<InputState>,
    advanced: bool,
    thousands: bool,
    from_encoding: Rfc4648Encoding,
    to_encoding: Rfc4648Encoding,
    use_custom: bool,
    syncing: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl NumberBaseView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let decimal = cx.new(|cx| InputState::new(window, cx).placeholder("十进制"));
        let hexadecimal = cx.new(|cx| InputState::new(window, cx).placeholder("十六进制"));
        let octal = cx.new(|cx| InputState::new(window, cx).placeholder("八进制"));
        let binary = cx.new(|cx| InputState::new(window, cx).placeholder("二进制"));
        let advanced_input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(6)
                .placeholder("高级输入")
        });
        let advanced_output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(6)
                .placeholder("高级输出")
        });
        let custom_alphabet = cx.new(|cx| InputState::new(window, cx).placeholder("自定义字符表"));

        let subscriptions = vec![
            cx.subscribe_in(&decimal, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_from(NumberBase::Decimal, window, cx);
                }
            }),
            cx.subscribe_in(&hexadecimal, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_from(NumberBase::Hexadecimal, window, cx);
                }
            }),
            cx.subscribe_in(&octal, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_from(NumberBase::Octal, window, cx);
                }
            }),
            cx.subscribe_in(&binary, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_from(NumberBase::Binary, window, cx);
                }
            }),
            cx.subscribe_in(&advanced_input, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reconvert_advanced(window, cx);
                }
            }),
            cx.subscribe_in(&custom_alphabet, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reconvert_advanced(window, cx);
                }
            }),
        ];

        Self {
            decimal,
            hexadecimal,
            octal,
            binary,
            advanced_input,
            advanced_output,
            custom_alphabet,
            advanced: false,
            thousands: false,
            from_encoding: Rfc4648Encoding::Base64,
            to_encoding: Rfc4648Encoding::Base16,
            use_custom: false,
            syncing: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn field(&self, base: NumberBase) -> &Entity<InputState> {
        match base {
            NumberBase::Decimal => &self.decimal,
            NumberBase::Hexadecimal => &self.hexadecimal,
            NumberBase::Octal => &self.octal,
            NumberBase::Binary => &self.binary,
        }
    }

    fn sync_from(&mut self, from: NumberBase, window: &mut Window, cx: &mut Context<Self>) {
        if self.syncing || self.advanced {
            return;
        }
        let source = self.field(from).read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.clear_others(from, window, cx);
            cx.notify();
            return;
        }
        let targets = [
            NumberBase::Decimal,
            NumberBase::Hexadecimal,
            NumberBase::Octal,
            NumberBase::Binary,
        ];
        self.syncing = true;
        let mut failed = false;
        for to in targets {
            if to == from {
                continue;
            }
            match convert_base(&source, from, to) {
                Ok(mut text) => {
                    if self.thousands {
                        text = add_thousands_separators(&text);
                    }
                    self.field(to).update(cx, |input, cx| {
                        input.set_value(text, window, cx);
                    });
                }
                Err(err) => {
                    self.error = Some(SharedString::from(err.to_string()));
                    failed = true;
                    break;
                }
            }
        }
        if !failed {
            self.error = None;
        }
        self.syncing = false;
        cx.notify();
    }

    fn clear_others(&mut self, from: NumberBase, window: &mut Window, cx: &mut Context<Self>) {
        self.syncing = true;
        for base in [
            NumberBase::Decimal,
            NumberBase::Hexadecimal,
            NumberBase::Octal,
            NumberBase::Binary,
        ] {
            if base == from {
                continue;
            }
            self.field(base).update(cx, |input, cx| {
                input.set_value(String::new(), window, cx);
            });
        }
        self.syncing = false;
    }

    fn reconvert_advanced(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.advanced {
            return;
        }
        let source = self.advanced_input.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.advanced_output.update(cx, |output, cx| {
                output.set_value(String::new(), window, cx);
            });
            cx.notify();
            return;
        }
        let result = if self.use_custom {
            let alphabet = self.custom_alphabet.read(cx).value().to_string();
            encode_custom(&source, &alphabet).or_else(|_| decode_custom(&source, &alphabet))
        } else if self.from_encoding == self.to_encoding {
            encode_rfc4648(&source, self.to_encoding)
        } else {
            convert_rfc4648(&source, self.from_encoding, self.to_encoding)
        };
        match result {
            Ok(text) => {
                self.error = None;
                self.advanced_output.update(cx, |output, cx| {
                    output.set_value(text, window, cx);
                });
            }
            Err(err) => {
                self.error = Some(SharedString::from(err.to_string()));
                self.advanced_output.update(cx, |output, cx| {
                    output.set_value(String::new(), window, cx);
                });
            }
        }
        cx.notify();
    }

    fn encoding_button(
        &self,
        id: &'static str,
        label: &'static str,
        encoding: Rfc4648Encoding,
        is_from: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = if is_from {
            self.from_encoding == encoding
        } else {
            self.to_encoding == encoding
        };
        Button::new(id)
            .label(label)
            .compact()
            .selected(selected)
            .on_click(cx.listener(move |this, _, window, cx| {
                if is_from {
                    this.from_encoding = encoding;
                } else {
                    this.to_encoding = encoding;
                }
                this.use_custom = false;
                this.reconvert_advanced(window, cx);
            }))
    }

    fn copy_output(&mut self, cx: &mut Context<Self>) {
        if self.error.is_some() {
            return;
        }
        let text = if self.advanced {
            self.advanced_output.read(cx).value().to_string()
        } else {
            self.hexadecimal.read(cx).value().to_string()
        };
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

impl ReceivesData for NumberBaseView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.advanced = false;
        let trimmed = payload.trim();
        let (base, value) = if let Some(rest) = strip_prefix_ci(trimmed, "0x") {
            (NumberBase::Hexadecimal, rest)
        } else if let Some(rest) = strip_prefix_ci(trimmed, "0b") {
            (NumberBase::Binary, rest)
        } else if let Some(rest) = strip_prefix_ci(trimmed, "0o") {
            (NumberBase::Octal, rest)
        } else {
            (NumberBase::Decimal, trimmed)
        };
        self.syncing = true;
        self.field(base).update(cx, |input, cx| {
            input.set_value(value.to_string(), window, cx);
        });
        self.syncing = false;
        self.sync_from(base, window, cx);
    }
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        text.get(prefix.len()..)
    } else {
        None
    }
}

impl Render for NumberBaseView {
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
                        Button::new("mode-basic")
                            .label("基础")
                            .compact()
                            .selected(!self.advanced)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.advanced = false;
                                this.error = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("mode-advanced")
                            .label("高级")
                            .compact()
                            .selected(self.advanced)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.advanced = true;
                                this.reconvert_advanced(window, cx);
                            })),
                    )
                    .child(
                        Switch::new("thousands")
                            .label("千分位")
                            .checked(self.thousands)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.thousands = *checked;
                                this.sync_from(NumberBase::Decimal, window, cx);
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
            .child(if self.advanced {
                self.render_advanced(cx)
            } else {
                self.render_basic()
            })
    }
}

impl NumberBaseView {
    fn render_basic(&self) -> gpui::AnyElement {
        v_flex()
            .gap_2()
            .child(labeled_input("十进制", &self.decimal))
            .child(labeled_input("十六进制", &self.hexadecimal))
            .child(labeled_input("八进制", &self.octal))
            .child(labeled_input("二进制", &self.binary))
            .into_any_element()
    }

    fn render_advanced(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        v_flex()
            .flex_1()
            .gap_2()
            .min_h_0()
            .child("输入编码")
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(self.encoding_button("from-16", "Base16", Rfc4648Encoding::Base16, true, cx))
                    .child(self.encoding_button("from-32", "Base32", Rfc4648Encoding::Base32, true, cx))
                    .child(self.encoding_button(
                        "from-32h",
                        "Base32Hex",
                        Rfc4648Encoding::Base32Hex,
                        true,
                        cx,
                    ))
                    .child(self.encoding_button("from-64", "Base64", Rfc4648Encoding::Base64, true, cx))
                    .child(self.encoding_button(
                        "from-64u",
                        "Base64URL",
                        Rfc4648Encoding::Base64Url,
                        true,
                        cx,
                    )),
            )
            .child("输出编码")
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(self.encoding_button("to-16", "Base16", Rfc4648Encoding::Base16, false, cx))
                    .child(self.encoding_button("to-32", "Base32", Rfc4648Encoding::Base32, false, cx))
                    .child(self.encoding_button(
                        "to-32h",
                        "Base32Hex",
                        Rfc4648Encoding::Base32Hex,
                        false,
                        cx,
                    ))
                    .child(self.encoding_button("to-64", "Base64", Rfc4648Encoding::Base64, false, cx))
                    .child(self.encoding_button(
                        "to-64u",
                        "Base64URL",
                        Rfc4648Encoding::Base64Url,
                        false,
                        cx,
                    ))
                    .child(
                        Switch::new("use-custom")
                            .label("自定义字符表")
                            .checked(self.use_custom)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.use_custom = *checked;
                                this.reconvert_advanced(window, cx);
                            })),
                    ),
            )
            .when(self.use_custom, |this| {
                this.child(Input::new(&self.custom_alphabet))
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
                            .child(Input::new(&self.advanced_input).h_full()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .min_h_0()
                            .child("输出")
                            .child(Input::new(&self.advanced_output).h_full().disabled(true)),
                    ),
            )
            .into_any_element()
    }
}

fn labeled_input(label: &'static str, input: &Entity<InputState>) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(label)
        .child(Input::new(input))
}
