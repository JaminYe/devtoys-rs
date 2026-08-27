use gpui::{
    div, prelude::*, ClipboardItem, Context, Entity, SharedString, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex, ActiveTheme, Selectable};

use crate::slot::ReceivesData;
use super::{datetime_to_timestamp, timestamp_to_datetime, TimestampFormat};

pub struct DateConverterView {
    timestamp: Entity<InputState>,
    datetime: Entity<InputState>,
    timezone: Entity<InputState>,
    epoch: Entity<InputState>,
    format: TimestampFormat,
    custom_epoch: bool,
    syncing: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl DateConverterView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial_timestamp = "0";
        let initial_datetime = timestamp_to_datetime(
            initial_timestamp,
            TimestampFormat::Seconds,
            None,
            None,
        )
        .unwrap_or_default();
        let timestamp = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("时间戳")
                .default_value(initial_timestamp)
        });
        let datetime = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("日期时间")
                .default_value(initial_datetime)
        });
        let timezone = cx.new(|cx| InputState::new(window, cx).placeholder("时区，默认本机"));
        let epoch = cx.new(|cx| InputState::new(window, cx).placeholder("自定义纪元"));

        let subscriptions = vec![
            cx.subscribe_in(&timestamp, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_from_timestamp(window, cx);
                }
            }),
            cx.subscribe_in(&datetime, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_from_datetime(window, cx);
                }
            }),
            cx.subscribe_in(&timezone, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_from_timestamp(window, cx);
                }
            }),
            cx.subscribe_in(&epoch, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_from_timestamp(window, cx);
                }
            }),
        ];

        Self {
            timestamp,
            datetime,
            timezone,
            epoch,
            format: TimestampFormat::Seconds,
            custom_epoch: false,
            syncing: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn zone(&self, cx: &Context<Self>) -> Option<String> {
        let value = self.timezone.read(cx).value().to_string();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn epoch_value(&self, cx: &Context<Self>) -> Option<String> {
        if !self.custom_epoch {
            return None;
        }
        let value = self.epoch.read(cx).value().to_string();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn sync_from_timestamp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.syncing {
            return;
        }
        let source = self.timestamp.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.set_datetime(String::new(), window, cx);
            cx.notify();
            return;
        }
        let zone = self.zone(cx);
        let epoch = self.epoch_value(cx);
        match timestamp_to_datetime(&source, self.format, zone.as_deref(), epoch.as_deref()) {
            Ok(text) => {
                self.error = None;
                self.set_datetime(text, window, cx);
            }
            Err(err) => {
                self.error = Some(SharedString::from(err.to_string()));
            }
        }
        cx.notify();
    }

    fn sync_from_datetime(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.syncing {
            return;
        }
        let source = self.datetime.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.set_timestamp(String::new(), window, cx);
            cx.notify();
            return;
        }
        let zone = self.zone(cx);
        let epoch = self.epoch_value(cx);
        match datetime_to_timestamp(&source, self.format, zone.as_deref(), epoch.as_deref()) {
            Ok(text) => {
                self.error = None;
                self.set_timestamp(text, window, cx);
            }
            Err(err) => {
                self.error = Some(SharedString::from(err.to_string()));
            }
        }
        cx.notify();
    }

    fn set_datetime(&mut self, value: String, window: &mut Window, cx: &mut Context<Self>) {
        self.syncing = true;
        self.datetime.update(cx, |input, cx| {
            input.set_value(value, window, cx);
        });
        self.syncing = false;
    }

    fn set_timestamp(&mut self, value: String, window: &mut Window, cx: &mut Context<Self>) {
        self.syncing = true;
        self.timestamp.update(cx, |input, cx| {
            input.set_value(value, window, cx);
        });
        self.syncing = false;
    }

    fn set_format(
        &mut self,
        format: TimestampFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.format = format;
        self.sync_from_datetime(window, cx);
    }

    fn format_button(
        &self,
        id: &'static str,
        label: &'static str,
        value: TimestampFormat,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .label(label)
            .compact()
            .selected(self.format == value)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.set_format(value, window, cx);
            }))
    }

    fn copy_datetime(&mut self, cx: &mut Context<Self>) {
        if self.error.is_some() {
            return;
        }
        let text = self.datetime.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

impl ReceivesData for DateConverterView {
    fn on_data_received(
        &mut self,
        payload: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let trimmed = payload.trim();
        let digits = trimmed.strip_prefix('-').unwrap_or(trimmed);
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            self.set_timestamp(trimmed.to_string(), window, cx);
            self.sync_from_timestamp(window, cx);
        } else {
            self.set_datetime(trimmed.to_string(), window, cx);
            self.sync_from_datetime(window, cx);
        }
    }
}

impl Render for DateConverterView {
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
                    .child(self.format_button("fmt-ticks", "Ticks", TimestampFormat::Ticks, cx))
                    .child(self.format_button("fmt-secs", "秒", TimestampFormat::Seconds, cx))
                    .child(self.format_button(
                        "fmt-ms",
                        "毫秒",
                        TimestampFormat::Milliseconds,
                        cx,
                    ))
                    .child(
                        Switch::new("custom-epoch")
                            .label("自定义纪元")
                            .checked(self.custom_epoch)
                            .on_click(cx.listener(|this, checked, window, cx| {
                                this.custom_epoch = *checked;
                                this.sync_from_timestamp(window, cx);
                            })),
                    )
                    .child(
                        Button::new("copy-output")
                            .primary()
                            .label("复制")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.copy_datetime(cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child("时区")
                    .child(Input::new(&self.timezone).w_64())
                    .when(self.custom_epoch, |this| {
                        this.child("纪元").child(Input::new(&self.epoch).w_64())
                    }),
            )
            .when_some(self.error.clone(), |this, message| {
                this.child(div().text_color(cx.theme().danger).child(message))
            })
            .child(
                v_flex()
                    .gap_1()
                    .child("时间戳")
                    .child(Input::new(&self.timestamp)),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child("日期")
                    .child(Input::new(&self.datetime)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn opens_with_stable_initial_values(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(DateConverterView::new);
        let (timestamp, datetime) = cx.update(|_, cx| {
            let view = view.read(cx);
            (
                view.timestamp.read(cx).value().to_string(),
                view.datetime.read(cx).value().to_string(),
            )
        });

        assert_eq!(timestamp, "0");
        assert!(!datetime.is_empty());
    }
}
