use gpui::{div, prelude::*, Context, Entity, Subscription, Window};
use gpui_component::button::Button;
use gpui_component::highlighter::HighlightTheme;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::text::{TextView, TextViewStyle};
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::markdown_to_html;

pub struct MarkdownPreviewView {
    editor: Entity<InputState>,
    html: Entity<InputState>,
    preview_dark: bool,
    _subscriptions: Vec<Subscription>,
}

impl MarkdownPreviewView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("Markdown")
        });
        let html = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(4)
                .placeholder("HTML")
        });

        let subscriptions = vec![cx.subscribe_in(
            &editor,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.refresh_html(window, cx);
                }
            },
        )];

        Self {
            editor,
            html,
            preview_dark: cx.theme().is_dark(),
            _subscriptions: subscriptions,
        }
    }

    fn refresh_html(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.editor.read(cx).value().to_string();
        let rendered = markdown_to_html(&source);
        self.html.update(cx, |html, cx| {
            html.set_value(rendered, window, cx);
        });
        cx.notify();
    }
}

impl ReceivesData for MarkdownPreviewView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editor.update(cx, |input, cx| {
            input.set_value(payload.to_string(), window, cx);
        });
        self.refresh_html(window, cx);
    }
}

impl Render for MarkdownPreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.editor.read(cx).value().to_string();
        let highlight = if self.preview_dark {
            HighlightTheme::default_dark()
        } else {
            HighlightTheme::default_light()
        };
        let style = TextViewStyle {
            is_dark: self.preview_dark,
            highlight_theme: highlight,
            ..Default::default()
        };

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("preview-dark")
                            .label("深色")
                            .compact()
                            .selected(self.preview_dark)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.preview_dark = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("preview-light")
                            .label("浅色")
                            .compact()
                            .selected(!self.preview_dark)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.preview_dark = false;
                                cx.notify();
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
                            .child("编辑")
                            .child(Input::new(&self.editor).h_full()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .min_h_0()
                            .child("预览")
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .p_3()
                                    .bg(if self.preview_dark {
                                        cx.theme().background
                                    } else {
                                        cx.theme().background
                                    })
                                    .child(
                                        TextView::markdown("md-preview", source, window, cx)
                                            .style(style)
                                            .scrollable(true)
                                            .selectable(true),
                                    ),
                            )
                            .child("HTML")
                            .child(Input::new(&self.html).disabled(true)),
                    ),
            )
    }
}
