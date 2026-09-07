use crate::slot::ToolView;
use crate::ui;

use super::{datetime_to_timestamp, timestamp_to_datetime, TimestampFormat};

pub struct DateConverterView {
    timestamp: String,
    datetime: String,
    timezone: String,
    epoch: String,
    format: TimestampFormat,
    custom_epoch: bool,
    syncing: bool,
    error: Option<String>,
}

impl DateConverterView {
    pub fn new() -> Self {
        let initial_timestamp = "0".to_string();
        let initial_datetime =
            timestamp_to_datetime(&initial_timestamp, TimestampFormat::Seconds, None, None)
                .unwrap_or_default();
        Self {
            timestamp: initial_timestamp,
            datetime: initial_datetime,
            timezone: String::new(),
            epoch: String::new(),
            format: TimestampFormat::Seconds,
            custom_epoch: false,
            syncing: false,
            error: None,
        }
    }

    fn zone(&self) -> Option<&str> {
        let trimmed = self.timezone.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    fn epoch_value(&self) -> Option<&str> {
        if !self.custom_epoch {
            return None;
        }
        let trimmed = self.epoch.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    fn sync_from_timestamp(&mut self) {
        if self.syncing {
            return;
        }
        if self.timestamp.trim().is_empty() {
            self.error = None;
            self.set_datetime(String::new());
            return;
        }
        match timestamp_to_datetime(
            &self.timestamp,
            self.format,
            self.zone(),
            self.epoch_value(),
        ) {
            Ok(text) => {
                self.error = None;
                self.set_datetime(text);
            }
            Err(err) => {
                self.error = Some(err.to_string());
            }
        }
    }

    fn sync_from_datetime(&mut self) {
        if self.syncing {
            return;
        }
        if self.datetime.trim().is_empty() {
            self.error = None;
            self.set_timestamp(String::new());
            return;
        }
        match datetime_to_timestamp(&self.datetime, self.format, self.zone(), self.epoch_value()) {
            Ok(text) => {
                self.error = None;
                self.set_timestamp(text);
            }
            Err(err) => {
                self.error = Some(err.to_string());
            }
        }
    }

    fn set_datetime(&mut self, value: String) {
        self.syncing = true;
        self.datetime = value;
        self.syncing = false;
    }

    fn set_timestamp(&mut self, value: String) {
        self.syncing = true;
        self.timestamp = value;
        self.syncing = false;
    }

    fn set_format(&mut self, format: TimestampFormat) {
        self.format = format;
        self.sync_from_datetime();
    }
}

impl ToolView for DateConverterView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("Ticks", TimestampFormat::Ticks),
                ("秒", TimestampFormat::Seconds),
                ("毫秒", TimestampFormat::Milliseconds),
            ] {
                if ui::toggle(ui, self.format == value, label).clicked() {
                    self.set_format(value);
                }
            }
            if ui.checkbox(&mut self.custom_epoch, "自定义纪元").changed() {
                self.sync_from_timestamp();
            }
            if ui::primary_button(ui, "复制").clicked() && self.error.is_none() {
                ui::copy_text(ui, &self.datetime);
            }
        });
        ui.horizontal(|ui| {
            ui.label("时区");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.timezone)
                        .id_salt("date-tz")
                        .hint_text("时区，默认本机")
                        .desired_width(220.0),
                )
                .changed()
            {
                self.sync_from_timestamp();
            }
            if self.custom_epoch {
                ui.label("纪元");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.epoch)
                            .id_salt("date-epoch")
                            .hint_text("自定义纪元")
                            .desired_width(220.0),
                    )
                    .changed()
                {
                    self.sync_from_timestamp();
                }
            }
        });
        ui::error_label(ui, self.error.as_deref());
        ui.label("时间戳");
        if ui::singleline(ui, "date-ts", &mut self.timestamp, "时间戳") {
            self.sync_from_timestamp();
        }
        ui.label("日期");
        if ui::singleline(ui, "date-dt", &mut self.datetime, "日期时间") {
            self.sync_from_datetime();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        let trimmed = payload.trim();
        let digits = trimmed.strip_prefix('-').unwrap_or(trimmed);
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            self.set_timestamp(trimmed.to_string());
            self.sync_from_timestamp();
        } else {
            self.set_datetime(trimmed.to_string());
            self.sync_from_datetime();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_with_stable_initial_values() {
        let view = DateConverterView::new();
        assert_eq!(view.timestamp, "0");
        assert!(!view.datetime.is_empty());
    }
}
