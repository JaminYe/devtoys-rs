use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex};

use crate::slot::ReceivesData;
use super::{apply, stats, Operation, TextStats};

pub struct TextAnalyzerView {
    input: Entity<InputState>,
    stats: TextStats,
    _subscriptions: Vec<Subscription>,
}

impl TextAnalyzerView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("粘贴或输入文本")
        });

        let subscriptions = vec![cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.refresh_stats(cx);
                }
            },
        )];

        Self {
            input,
            stats: stats(""),
            _subscriptions: subscriptions,
        }
    }

    fn refresh_stats(&mut self, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        self.stats = stats(&source);
        cx.notify();
    }

    fn apply_op(&mut self, op: Operation, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        let mut rng = rand::thread_rng();
        let result = apply(&source, &[op], &mut rng);
        self.input.update(cx, |input, cx| {
            input.set_value(result, window, cx);
        });
        self.refresh_stats(cx);
    }

    fn op_button(
        &self,
        id: &'static str,
        label: &'static str,
        op: Operation,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.apply_op(op, window, cx);
            }))
    }

    fn copy_text(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    fn stat_row(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(div().min_w(px_label()).child(label))
            .child(value.into())
    }
}

fn px_label() -> gpui::Pixels {
    gpui::px(72.)
}

impl ReceivesData for TextAnalyzerView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.refresh_stats(cx);
    }
}

impl Render for TextAnalyzerView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cursor = self.input.read(cx).cursor_position();
        let s = &self.stats;

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child("换行")
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .child(self.op_button("eol-lf", "LF", Operation::LineEndingsLf, cx))
                            .child(self.op_button("eol-crlf", "CRLF", Operation::LineEndingsCrlf, cx)),
                    )
                    .child("大小写")
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .child(self.op_button("case-lower", "小写", Operation::Lower, cx))
                            .child(self.op_button("case-upper", "大写", Operation::Upper, cx))
                            .child(self.op_button("case-sentence", "句首", Operation::Sentence, cx))
                            .child(self.op_button("case-title", "标题", Operation::Title, cx))
                            .child(self.op_button("case-camel", "camel", Operation::Camel, cx))
                            .child(self.op_button("case-pascal", "Pascal", Operation::Pascal, cx))
                            .child(self.op_button("case-snake", "snake", Operation::Snake, cx))
                            .child(self.op_button("case-const", "CONSTANT", Operation::Constant, cx))
                            .child(self.op_button("case-kebab", "kebab", Operation::Kebab, cx))
                            .child(self.op_button("case-cobol", "COBOL", Operation::Cobol, cx))
                            .child(self.op_button("case-train", "Train", Operation::Train, cx))
                            .child(self.op_button("case-alt", "交替", Operation::Alternating, cx))
                            .child(self.op_button("case-inv", "反转", Operation::Inverse, cx))
                            .child(self.op_button("case-rand", "随机", Operation::RandomCase, cx)),
                    )
                    .child("行")
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .child(self.op_button("sort-asc", "字母序", Operation::SortLines, cx))
                            .child(self.op_button("sort-desc", "倒序", Operation::SortLinesDesc, cx))
                            .child(self.op_button("sort-last", "按末词", Operation::SortByLastWord, cx))
                            .child(self.op_button(
                                "sort-last-desc",
                                "按末词倒序",
                                Operation::SortByLastWordDesc,
                                cx,
                            ))
                            .child(self.op_button("rev-lines", "反转行", Operation::ReverseLines, cx))
                            .child(self.op_button("shuffle", "打乱行", Operation::ShuffleLines, cx))
                            .child(
                                Button::new("copy-text")
                                    .primary()
                                    .label("复制")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.copy_text(cx);
                                    })),
                            ),
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
                            .child("输入")
                            .child(Input::new(&self.input).h_full()),
                    )
                    .child(
                        v_flex()
                            .w(gpui::px(260.))
                            .gap_1()
                            .min_h_0()
                            .child("统计")
                            .child(Self::stat_row("选区行", format!("{}", cursor.line + 1)))
                            .child(Self::stat_row("选区列", format!("{}", cursor.character + 1)))
                            .child(Self::stat_row("字节", format!("{}", s.bytes)))
                            .child(Self::stat_row("字符", format!("{}", s.chars)))
                            .child(Self::stat_row("词", format!("{}", s.words)))
                            .child(Self::stat_row("句", format!("{}", s.sentences)))
                            .child(Self::stat_row("段", format!("{}", s.paragraphs)))
                            .child(Self::stat_row("行", format!("{}", s.lines)))
                            .child(Self::stat_row("换行", s.eol.as_str()))
                            .child(div().mt_2().child("词频"))
                            .children(top_word_freq(&s.word_freq).into_iter().map(|(k, v)| {
                                h_flex()
                                    .gap_2()
                                    .child(div().min_w(px_label()).child(k))
                                    .child(v)
                            }))
                            .child(div().mt_2().child("字符频"))
                            .children(top_char_freq(&s.char_freq).into_iter().map(|(k, v)| {
                                h_flex()
                                    .gap_2()
                                    .child(div().min_w(px_label()).child(k))
                                    .child(v)
                            })),
                    ),
            )
    }
}

fn top_word_freq(map: &std::collections::HashMap<String, usize>) -> Vec<(String, String)> {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    items
        .into_iter()
        .take(12)
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect()
}

fn top_char_freq(map: &std::collections::HashMap<char, usize>) -> Vec<(String, String)> {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    items
        .into_iter()
        .take(12)
        .map(|(k, v)| {
            let label = if *k == ' ' { "⎵".to_string() } else { k.to_string() };
            (label, v.to_string())
        })
        .collect()
}
