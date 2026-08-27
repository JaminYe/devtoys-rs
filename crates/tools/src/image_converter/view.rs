use gpui::{
    div, prelude::*, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;

use super::{convert_image, ImageTargetFormat};

pub struct ImageConverterView {
    path: Entity<InputState>,
    /// Last chosen target; in-memory only (no global settings).
    target: ImageTargetFormat,
    status: Option<SharedString>,
    error: Option<SharedString>,
    preview: Option<Vec<u8>>,
    _subscriptions: Vec<Subscription>,
}

impl ImageConverterView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let path = cx.new(|cx| {
            InputState::new(window, cx).placeholder("图像文件或目录（多行路径）")
        });
        let subscriptions = vec![cx.subscribe_in(
            &path,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.reconvert(window, cx);
                }
            },
        )];
        Self {
            path,
            target: ImageTargetFormat::Png,
            status: None,
            error: None,
            preview: None,
            _subscriptions: subscriptions,
        }
    }

    fn reconvert(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.path.read(cx).value().to_string();
        let paths: Vec<&str> = raw
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        if paths.is_empty() {
            self.error = None;
            self.status = None;
            self.preview = None;
            cx.notify();
            return;
        }

        let first = paths[0];
        match std::fs::read(first) {
            Ok(bytes) => match convert_image(&bytes, self.target) {
                Ok(converted) => {
                    self.error = None;
                    self.preview = Some(converted);
                    self.status = Some(SharedString::from(format!(
                        "已转换 {} 个路径 → {}",
                        paths.len(),
                        self.target.as_str()
                    )));
                }
                Err(_) => {
                    self.preview = None;
                    self.status = None;
                    self.error = Some(SharedString::from("无法转换图像"));
                }
            },
            Err(_) => {
                self.preview = None;
                self.status = None;
                self.error = Some(SharedString::from("无法读取图像"));
            }
        }
        cx.notify();
    }

    fn set_target(
        &mut self,
        target: ImageTargetFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.target = target;
        self.reconvert(window, cx);
    }

    fn format_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: ImageTargetFormat,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.target == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_target(value, window, cx);
            }))
    }
}

impl ReceivesData for ImageConverterView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let filled = payload
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if filled.is_empty() {
            return;
        }
        self.path.update(cx, |input, cx| {
            input.set_value(filled, window, cx);
        });
        self.reconvert(window, cx);
    }
}

impl Render for ImageConverterView {
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
                    .child(self.format_button("fmt-bmp", "BMP", ImageTargetFormat::Bmp, cx))
                    .child(self.format_button("fmt-jpeg", "JPEG", ImageTargetFormat::Jpeg, cx))
                    .child(self.format_button("fmt-pbm", "PBM", ImageTargetFormat::Pbm, cx))
                    .child(self.format_button("fmt-png", "PNG", ImageTargetFormat::Png, cx))
                    .child(self.format_button("fmt-tga", "TGA", ImageTargetFormat::Tga, cx))
                    .child(self.format_button("fmt-tiff", "TIFF", ImageTargetFormat::Tiff, cx))
                    .child(self.format_button("fmt-webp", "WEBP", ImageTargetFormat::Webp, cx)),
            )
            .when_some(self.error.clone(), |this, message| {
                this.child(div().text_color(cx.theme().danger).child(message))
            })
            .when_some(self.status.clone(), |this, message| this.child(message))
            .child("选中路径")
            .child(Input::new(&self.path).h_full())
            .when(self.preview.is_some(), |this| {
                this.child(format!(
                    "预览已生成（{} 字节）",
                    self.preview.as_ref().map(Vec::len).unwrap_or(0)
                ))
            })
    }
}
