use std::sync::Arc;

use gpui::{
    div, img, prelude::*, px, Context, Image, ImageFormat, SharedString, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{v_flex, ActiveTheme};

use crate::slot::ReceivesData;

use super::simulate_color_blindness;

pub struct ColorBlindnessView {
    path: gpui::Entity<InputState>,
    error: Option<SharedString>,
    original: Option<Vec<u8>>,
    protanopia: Option<Vec<u8>>,
    deuteranopia: Option<Vec<u8>>,
    tritanopia: Option<Vec<u8>>,
    _subscriptions: Vec<gpui::Subscription>,
}

impl ColorBlindnessView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let path = cx.new(|cx| InputState::new(window, cx).placeholder("图像文件路径"));
        let subscriptions = vec![cx.subscribe_in(
            &path,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.resimulate(window, cx);
                }
            },
        )];
        Self {
            path,
            error: None,
            original: None,
            protanopia: None,
            deuteranopia: None,
            tritanopia: None,
            _subscriptions: subscriptions,
        }
    }

    fn load_path(&mut self, payload: &str, window: &mut Window, cx: &mut Context<Self>) {
        let path = payload.trim();
        self.path.update(cx, |input, cx| {
            input.set_value(path.to_string(), window, cx);
        });
        self.resimulate(window, cx);
    }

    fn resimulate(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let source = self.path.read(cx).value().to_string();
        let path = source.trim();
        if path.is_empty() {
            self.error = None;
            self.clear_images();
            cx.notify();
            return;
        }

        match std::fs::read(path) {
            Ok(bytes) => match simulate_color_blindness(&bytes) {
                Ok(images) => {
                    self.error = None;
                    self.original = Some(images.original);
                    self.protanopia = Some(images.protanopia);
                    self.deuteranopia = Some(images.deuteranopia);
                    self.tritanopia = Some(images.tritanopia);
                }
                Err(_) => {
                    self.error = Some(SharedString::from("无法解码图像"));
                    self.clear_images();
                }
            },
            Err(_) => {
                self.error = Some(SharedString::from("无法读取图像"));
                self.clear_images();
            }
        }
        cx.notify();
    }

    fn clear_images(&mut self) {
        self.original = None;
        self.protanopia = None;
        self.deuteranopia = None;
        self.tritanopia = None;
    }
}

impl ReceivesData for ColorBlindnessView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = payload
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("");
        if path.is_empty() {
            return;
        }
        self.load_path(path, window, cx);
    }
}

impl Render for ColorBlindnessView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child("图像路径")
            .child(Input::new(&self.path))
            .when_some(self.error.clone(), |this, message| {
                this.child(div().text_color(cx.theme().danger).child(message))
            })
            .child(
                v_flex()
                    .flex_1()
                    .gap_3()
                    .min_h_0()
                    .child(
                gpui::div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .gap_3()
                    .min_h_0()
                            .child(pane("原图", self.original.as_deref()))
                            .child(pane("红色盲（Protanopia）", self.protanopia.as_deref())),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .flex_row()
                            .flex_1()
                            .gap_3()
                            .min_h_0()
                            .child(pane("绿色盲（Deuteranopia）", self.deuteranopia.as_deref()))
                            .child(pane("黄蓝色盲（Tritanopia）", self.tritanopia.as_deref())),
                    ),
            )
    }
}

fn pane(title: &'static str, png: Option<&[u8]>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .gap_1()
        .min_h_0()
        .child(title)
        .child(
            div()
                .flex_1()
                .min_h(px(120.))
                .border_1()
                .child(match png {
                    Some(bytes) => img(Arc::new(Image::from_bytes(
                        ImageFormat::Png,
                        bytes.to_vec(),
                    )))
                    .w_full()
                    .h_full()
                    .into_any_element(),
                    None => div().into_any_element(),
                }),
        )
}
