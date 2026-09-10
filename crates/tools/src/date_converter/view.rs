use chrono::{DateTime, Utc};

use crate::slot::ToolView;
use crate::ui;

use super::helper::{
    datetime_to_parts, dst_hint_at_datetime, now_values, parts_to_values, DateParts,
    COMMON_TIMEZONES,
};
use super::{datetime_to_timestamp, timestamp_to_datetime, TimestampFormat, ID};

/// Settings JSON `format` values: `ticks` | `seconds` | `milliseconds`.
/// Unknown / missing → [`TimestampFormat::Seconds`]. `timezone` missing → `""` (本机).
/// Epoch text is user content and is not persisted, so the custom-epoch
/// checkbox is also not persisted (it would restore checked with an empty epoch).
const DEFAULT_FORMAT: TimestampFormat = TimestampFormat::Seconds;

pub struct DateConverterView {
    timestamp: String,
    datetime: String,
    timezone: String,
    epoch: String,
    parts: DateParts,
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
        let parts = datetime_to_parts(&initial_datetime, None).unwrap_or_default();
        Self {
            timestamp: initial_timestamp,
            datetime: initial_datetime,
            timezone: String::new(),
            epoch: String::new(),
            parts,
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

    fn timezone_display(&self) -> &str {
        let trimmed = self.timezone.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("local") || trimmed == "本机" {
            "本机"
        } else {
            trimmed
        }
    }

    fn timezone_list_selected(&self, name: &str) -> bool {
        if name == "本机" {
            let trimmed = self.timezone.trim();
            trimmed.is_empty() || trimmed.eq_ignore_ascii_case("local") || trimmed == "本机"
        } else {
            self.timezone.trim() == name
        }
    }

    fn refresh_parts(&mut self) {
        if let Ok(parts) = datetime_to_parts(&self.datetime, self.zone()) {
            self.parts = parts;
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
                self.refresh_parts();
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
                self.refresh_parts();
            }
            Err(err) => {
                self.error = Some(err.to_string());
            }
        }
    }

    fn sync_from_parts(&mut self) {
        if self.syncing {
            return;
        }
        match parts_to_values(self.parts, self.format, self.zone(), self.epoch_value()) {
            Ok((timestamp, datetime)) => {
                self.error = None;
                self.set_timestamp(timestamp);
                self.set_datetime(datetime);
            }
            Err(err) => {
                self.error = Some(err.to_string());
            }
        }
    }

    fn apply_now(&mut self, now: DateTime<Utc>) {
        match now_values(now, self.format, self.zone(), self.epoch_value()) {
            Ok((timestamp, datetime)) => {
                self.error = None;
                self.set_timestamp(timestamp);
                self.set_datetime(datetime);
                self.refresh_parts();
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
            ui.label(ui::t(ui, "date.format"));
            for (label, value) in [
                ("Ticks", TimestampFormat::Ticks),
                (ui::t(ui, "date.second"), TimestampFormat::Seconds),
                ("毫秒", TimestampFormat::Milliseconds),
            ] {
                if ui::toggle(ui, self.format == value, label).clicked() {
                    self.set_format(value);
                }
            }
            if ui
                .checkbox(&mut self.custom_epoch, ui::t(ui, "date.custom_epoch"))
                .changed()
            {
                self.sync_from_timestamp();
            }
            if ui.button(ui::t(ui, "date.now")).clicked() {
                self.apply_now(Utc::now());
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.datetime.as_str()));
        });
        ui.horizontal(|ui| {
            ui.label(ui::t(ui, "date.timezone"));
            let mut tz_changed = false;
            let tz_display = self.timezone_display().to_string();
            egui::ComboBox::from_id_salt("date-tz-combo")
                .selected_text(tz_display)
                .width(180.0)
                .show_ui(ui, |ui| {
                    for &name in COMMON_TIMEZONES {
                        let selected = self.timezone_list_selected(name);
                        if ui.selectable_label(selected, name).clicked() {
                            self.timezone = if name == "本机" {
                                String::new()
                            } else {
                                name.to_string()
                            };
                            tz_changed = true;
                        }
                    }
                });
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.timezone)
                        .id_salt("date-tz")
                        .hint_text("时区，默认本机")
                        .desired_width(180.0),
                )
                .changed()
            {
                tz_changed = true;
            }
            if tz_changed {
                self.sync_from_timestamp();
            }
            if self.custom_epoch {
                ui.label(ui::t(ui, "date.custom_epoch"));
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
        if let Some(hint) = dst_hint_at_datetime(&self.datetime, self.zone()) {
            ui.label(hint);
        }
        let err_text = self.error.as_deref().map(|e| {
            if e == "非法日期或时间戳" {
                ui::t(ui, "date.invalid")
            } else {
                e
            }
        });
        ui::error_label(ui, err_text);
        ui.label("时间戳");
        if ui::singleline(ui, "date-ts", &mut self.timestamp, "时间戳") {
            self.sync_from_timestamp();
        }
        ui.label("日期");
        if ui::singleline(ui, "date-dt", &mut self.datetime, "日期时间") {
            self.sync_from_datetime();
        }
        ui.horizontal(|ui| {
            if labeled_part(ui, ui::t(ui, "date.year"), "date-year", &mut self.parts.year, 1..=9999) {
                self.sync_from_parts();
            }
            if labeled_part(ui, ui::t(ui, "date.month"), "date-month", &mut self.parts.month, 1..=12) {
                self.sync_from_parts();
            }
            if labeled_part(ui, ui::t(ui, "date.day"), "date-day", &mut self.parts.day, 1..=31) {
                self.sync_from_parts();
            }
            if labeled_part(ui, ui::t(ui, "date.hour"), "date-hour", &mut self.parts.hour, 0..=23) {
                self.sync_from_parts();
            }
            if labeled_part(ui, ui::t(ui, "date.minute"), "date-minute", &mut self.parts.minute, 0..=59) {
                self.sync_from_parts();
            }
            if labeled_part(ui, ui::t(ui, "date.second"), "date-second", &mut self.parts.second, 0..=59) {
                self.sync_from_parts();
            }
        });
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

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "format": format_to_settings(self.format),
                "timezone": self.timezone,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.format = format_from_settings(value.get("format").and_then(|v| v.as_str()));
        self.timezone = value
            .get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        self.sync_from_timestamp();
    }
}

fn labeled_part(
    ui: &mut egui::Ui,
    label: &str,
    id: &str,
    value: &mut i32,
    range: std::ops::RangeInclusive<i32>,
) -> bool {
    ui.push_id(id, |ui| {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(value).range(range).speed(1.0))
                .changed()
        })
        .inner
    })
    .inner
}

fn format_to_settings(format: TimestampFormat) -> &'static str {
    match format {
        TimestampFormat::Ticks => "ticks",
        TimestampFormat::Seconds => "seconds",
        TimestampFormat::Milliseconds => "milliseconds",
    }
}

fn format_from_settings(value: Option<&str>) -> TimestampFormat {
    match value {
        Some("ticks") => TimestampFormat::Ticks,
        Some("seconds") => TimestampFormat::Seconds,
        Some("milliseconds") => TimestampFormat::Milliseconds,
        _ => DEFAULT_FORMAT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn opens_with_stable_initial_values() {
        let view = DateConverterView::new();
        assert_eq!(view.timestamp, "0");
        assert!(!view.datetime.is_empty());
    }

    #[test]
    fn display_and_copy_use_helper_fractional_seconds() {
        let mut view = DateConverterView::new();
        view.timezone = "UTC".into();
        view.format = TimestampFormat::Milliseconds;
        view.on_data_received("1");
        assert_eq!(view.datetime, "1970-01-01T00:00:00.0010000+00:00");
        assert!(view.error.is_none());

        view.format = TimestampFormat::Ticks;
        view.on_data_received("0");
        assert_eq!(view.datetime, "0001-01-01T00:00:00.0000000+00:00");
        view.on_data_received("1");
        assert_eq!(view.datetime, "0001-01-01T00:00:00.0000001+00:00");
        assert!(view.error.is_none());
    }

    #[test]
    fn persistable_options_are_format_and_timezone_only() {
        let mut view = DateConverterView::new();
        view.format = TimestampFormat::Milliseconds;
        view.timezone = "UTC".into();
        view.custom_epoch = true;
        view.epoch = "2000-01-01T00:00:00Z".into();
        view.timestamp = "123456".into();
        view.datetime = "secret-date".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["format"], "milliseconds");
        assert_eq!(value["timezone"], "UTC");
        assert!(value.get("custom_epoch").is_none());
        assert!(value.get("timestamp").is_none());
        assert!(value.get("datetime").is_none());
        assert!(value.get("epoch").is_none());
        let dumped = value.to_string();
        assert!(!dumped.contains("123456"));
        assert!(!dumped.contains("secret-date"));
        assert!(!dumped.contains("2000-01-01"));
    }

    #[test]
    fn restore_ticks_utc_then_converts_initial_timestamp() {
        let mut view = DateConverterView::new();
        view.restore_options(&serde_json::json!({
            "format": "ticks",
            "timezone": "UTC",
            "custom_epoch": true
        }));
        assert_eq!(view.format, TimestampFormat::Ticks);
        assert_eq!(view.timezone, "UTC");
        assert!(
            !view.custom_epoch,
            "stale custom_epoch in old settings must not check the box without epoch text"
        );
        assert_eq!(view.datetime, "0001-01-01T00:00:00.0000000+00:00");
        assert!(view.error.is_none());
        view.on_data_received("1");
        assert_eq!(view.datetime, "0001-01-01T00:00:00.0000001+00:00");
    }

    #[test]
    fn missing_and_illegal_format_use_defaults() {
        let mut view = DateConverterView::new();
        view.format = TimestampFormat::Ticks;
        view.timezone = "UTC".into();
        view.restore_options(&serde_json::json!({ "format": "nanos" }));
        assert_eq!(view.format, TimestampFormat::Seconds);
        assert_eq!(view.timezone, "");
        assert!(!view.custom_epoch);
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.format, TimestampFormat::Seconds);
        assert_eq!(view.timezone, "");
        assert!(!view.custom_epoch);
    }

    #[test]
    fn now_uses_injected_clock_and_updates_parts() {
        let mut view = DateConverterView::new();
        view.timezone = "UTC".into();
        let now = DateTime::from_timestamp(1_721_061_045, 123_000_000).unwrap();
        view.apply_now(now);
        assert_eq!(view.timestamp, "1721061045");
        assert_eq!(view.datetime, "2024-07-15T16:30:45.1230000+00:00");
        assert_eq!(view.parts.year, 2024);
        assert_eq!(view.parts.month, 7);
        assert_eq!(view.parts.day, 15);
        assert_eq!(view.parts.hour, 16);
        assert_eq!(view.parts.minute, 30);
        assert_eq!(view.parts.second, 45);
        assert!(view.error.is_none());
    }

    #[test]
    fn i18n_keys_have_both_translations_and_error_switches() {
        let keys = [
            "cron.expression",
            "cron.include_seconds",
            "cron.next_occurrences",
            "cron.description",
            "cron.invalid_expression",
            "date.format",
            "date.timezone",
            "date.now",
            "date.custom_epoch",
            "date.year",
            "date.month",
            "date.day",
            "date.hour",
            "date.minute",
            "date.second",
            "date.invalid",
            "json_table.flatten",
            "json_yaml.json_to_yaml",
            "json_yaml.yaml_to_json",
            "number_base.decimal",
            "number_base.hexadecimal",
            "number_base.octal",
            "number_base.binary",
            "number_base.signed",
            "number_base.unsigned",
            "number_base.format_thousands",
            "json.sort_properties",
            "sql.language",
            "sql.leading_comma",
            "xml.attributes_on_new_lines",
            "common.input",
            "common.output",
            "common.two_spaces",
            "common.four_spaces",
            "common.one_tab",
            "common.minified",
        ];
        for k in keys {
            let zh = devtoys_api::t(k, devtoys_api::Language::ZhCn);
            let en = devtoys_api::t(k, devtoys_api::Language::EnUs);
            assert!(!zh.is_empty(), "key {k} has empty zh");
            assert!(!en.is_empty(), "key {k} has empty en");
            assert_ne!(zh, k, "key {k} missing zh translation");
            assert_ne!(en, k, "key {k} missing en translation");
            assert_ne!(zh, en, "key {k} zh and en should differ");
        }

        let zh_err = devtoys_api::t("date.invalid", devtoys_api::Language::ZhCn);
        let en_err = devtoys_api::t("date.invalid", devtoys_api::Language::EnUs);
        assert_eq!(zh_err, "无效的日期或时间戳");
        assert_eq!(en_err, "Invalid date or timestamp");
        assert_ne!(zh_err, en_err);
    }
}
