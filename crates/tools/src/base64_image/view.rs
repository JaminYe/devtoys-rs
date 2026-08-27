use std::fs;
use std::path::Path;

use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme};

use crate::slot::ReceivesData;
use super::{decode_base64, encode_bytes, inspect_image};

pub struct Base64ImageView {
    input: Entity<InputState>,
    preview: Option<SharedString>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl Base64ImageView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(12)
                .placeholder("粘贴 Base64 或 data URI")
        });

        let subscriptions = vec![cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.refresh(window, cx);
                }
            },
        )];

        Self {
            input,
            preview: None,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.preview = None;
            cx.notify();
            return;
        }
        match decode_base64(&source) {
            Ok(bytes) => {
                self.error = None;
                self.preview = Some(SharedString::from(inspect_image(&bytes).summary()));
            }
            Err(err) => {
                self.preview = None;
                self.error = Some(SharedString::from(err.to_string()));
            }
        }
        cx.notify();
    }

    fn copy_input(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

impl ReceivesData for Base64ImageView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = if Path::new(payload).is_file() {
            match fs::read(payload) {
                Ok(bytes) => encode_bytes(&bytes),
                Err(_) => payload.to_string(),
            }
        } else {
            payload.to_string()
        };
        self.input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
        });
        self.refresh(window, cx);
    }
}

impl Render for Base64ImageView {
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
                        Button::new("copy-output")
                            .primary()
                            .label("复制")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.copy_input(cx);
                            })),
                    ),
            )
            .when_some(self.error.clone(), |this, message| {
                this.child(
                    div()
                        .text_color(cx.theme().danger)
                        .child(message),
                )
            })
            .when_some(self.preview.clone(), |this, summary| {
                this.child(div().child(format!("预览：{summary}")))
            })
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .min_h_0()
                    .child("Base64")
                    .child(Input::new(&self.input).h_full()),
            )
    }
}
