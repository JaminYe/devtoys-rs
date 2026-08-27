use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{generate_lorem, Corpus, LoremUnit};

pub struct LoremIpsumView {
    length_input: Entity<InputState>,
    output: Entity<InputState>,
    corpus: Corpus,
    unit: LoremUnit,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl LoremIpsumView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let length_input = cx.new(|cx| InputState::new(window, cx).placeholder("长度"));
        length_input.update(cx, |input, cx| {
            input.set_value("1".to_string(), window, cx);
        });
        let output = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("生成结果")
        });
        let subscriptions = vec![cx.subscribe_in(
            &length_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.regenerate(window, cx);
                }
            },
        )];
        let mut this = Self {
            length_input,
            output,
            corpus: Corpus::LoremIpsum,
            unit: LoremUnit::Paragraphs,
            error: None,
            _subscriptions: subscriptions,
        };
        this.regenerate(window, cx);
        this
    }

    fn length(&self, cx: &Context<Self>) -> usize {
        self.length_input
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(1)
            .max(1)
    }

    fn regenerate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match generate_lorem(self.corpus, self.unit, self.length(cx)) {
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

    fn corpus_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: Corpus,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.corpus == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.corpus = value;
                this.regenerate(window, cx);
            }))
    }

    fn unit_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: LoremUnit,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.unit == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.unit = value;
                this.regenerate(window, cx);
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

impl ReceivesData for LoremIpsumView {
    fn on_data_received(&mut self, _: &str, _: &mut Window, _: &mut Context<Self>) {}
}

impl Render for LoremIpsumView {
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
                    .child(self.corpus_button("c-lorem", "LoremIpsum", Corpus::LoremIpsum, cx))
                    .child(self.corpus_button("c-harold", "ChildHarold", Corpus::ChildHarold, cx))
                    .child(self.corpus_button("c-decameron", "Decameron", Corpus::Decameron, cx))
                    .child(self.corpus_button("c-faust", "Faust", Corpus::Faust, cx))
                    .child(self.corpus_button("c-fremde", "InDerFremde", Corpus::InDerFremde, cx))
                    .child(self.corpus_button("c-bateau", "LeBateauIvre", Corpus::LeBateauIvre, cx))
                    .child(self.corpus_button("c-masque", "LeMasque", Corpus::LeMasque, cx))
                    .child(self.corpus_button("c-nagyon", "NagyonFaj", Corpus::NagyonFaj, cx))
                    .child(self.corpus_button("c-omagyar", "Omagyar", Corpus::Omagyar, cx))
                    .child(self.corpus_button("c-robin", "RobinsonoKruso", Corpus::RobinsonoKruso, cx))
                    .child(self.corpus_button("c-raven", "TheRaven", Corpus::TheRaven, cx))
                    .child(self.corpus_button("c-tierra", "TierrayLuna", Corpus::TierrayLuna, cx)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .flex_wrap()
                    .child(self.unit_button("u-para", "段落", LoremUnit::Paragraphs, cx))
                    .child(self.unit_button("u-sent", "句子", LoremUnit::Sentences, cx))
                    .child(self.unit_button("u-words", "词", LoremUnit::Words, cx))
                    .child(self.unit_button("u-chars", "字符", LoremUnit::Characters, cx))
                    .child(div().child("长度"))
                    .child(Input::new(&self.length_input).w_24())
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
                v_flex()
                    .flex_1()
                    .gap_1()
                    .min_h_0()
                    .child("输出")
                    .child(Input::new(&self.output).h_full().disabled(true)),
            )
    }
}
