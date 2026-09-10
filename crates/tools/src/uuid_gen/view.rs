use crate::slot::ToolView;
use crate::ui;

use super::{generate_uuid, UuidOptions, UuidVersion, ID};

const DEFAULT_VERSION: UuidVersion = UuidVersion::Four;
const DEFAULT_HYPHENS: bool = true;
const DEFAULT_UPPERCASE: bool = false;
const DEFAULT_COUNT: usize = 1;

pub struct UuidGenView {
    count: String,
    output: String,
    version: UuidVersion,
    hyphens: bool,
    uppercase: bool,
    error: Option<String>,
}

impl UuidGenView {
    pub fn new() -> Self {
        let mut this = Self {
            count: DEFAULT_COUNT.to_string(),
            output: String::new(),
            version: DEFAULT_VERSION,
            hyphens: DEFAULT_HYPHENS,
            uppercase: DEFAULT_UPPERCASE,
            error: None,
        };
        this.regenerate();
        this
    }

    fn options(&self) -> UuidOptions {
        UuidOptions {
            version: self.version,
            hyphens: self.hyphens,
            uppercase: self.uppercase,
            count: self.count.parse::<usize>().unwrap_or(DEFAULT_COUNT).max(1),
        }
    }

    fn regenerate(&mut self) {
        match generate_uuid(&self.options()) {
            Ok(text) => {
                self.error = None;
                self.output = text;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for UuidGenView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("v1", UuidVersion::One),
                ("v4", UuidVersion::Four),
                ("v7", UuidVersion::Seven),
            ] {
                if ui::toggle(ui, self.version == value, label).clicked() {
                    self.version = value;
                    dirty = true;
                }
            }
            dirty |= ui
                .checkbox(&mut self.hyphens, ui::t("uuid.hyphens"))
                .changed();
            dirty |= ui
                .checkbox(&mut self.uppercase, ui::t("uuid.uppercase"))
                .changed();
            ui.label(ui::t("uuid.count"));
            ui.allocate_ui(egui::vec2(72.0, ui.spacing().interact_size.y), |ui| {
                dirty |= ui::singleline(ui, "uuid-count", &mut self.count, "数量");
            });
            if ui.button(ui::t("password.generate")).clicked() {
                dirty = true;
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        if dirty {
            self.regenerate();
        }
        ui::error_label(ui, self.error.as_deref());
        ui::labeled_code(
            ui,
            ui::t("common.output"),
            "uuid-out",
            &mut self.output,
            "生成结果",
            false,
        );
    }

    fn on_data_received(&mut self, _: &str) {}

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "version": self.version.as_str(),
                "hyphens": self.hyphens,
                "uppercase": self.uppercase,
                "count": self.options().count,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.version = version_from_settings(value.get("version").and_then(|v| v.as_str()));
        self.hyphens = value
            .get("hyphens")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_HYPHENS);
        self.uppercase = value
            .get("uppercase")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_UPPERCASE);
        self.count = count_from_settings(value.get("count"), DEFAULT_COUNT).to_string();
        self.regenerate();
    }
}

fn version_from_settings(value: Option<&str>) -> UuidVersion {
    value
        .and_then(UuidVersion::parse)
        .unwrap_or(DEFAULT_VERSION)
}

fn count_from_settings(value: Option<&serde_json::Value>, default: usize) -> usize {
    match value
        .and_then(|v| v.as_u64())
        .and_then(|n| usize::try_from(n).ok())
    {
        Some(n) if n >= 1 => n,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    fn assert_no_sensitive(value: &serde_json::Value) {
        assert!(value.get("output").is_none());
        assert!(value.get("input").is_none());
    }

    #[test]
    fn persistable_options_are_version_format_and_count_only() {
        let mut view = UuidGenView::new();
        view.version = UuidVersion::Seven;
        view.hyphens = false;
        view.uppercase = true;
        view.count = "3".into();
        view.output = "should-not-persist-uuid".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["version"], "Seven");
        assert_eq!(value["hyphens"], false);
        assert_eq!(value["uppercase"], true);
        assert_eq!(value["count"], 3);
        assert_no_sensitive(&value);
        let dumped = value.to_string();
        assert!(!dumped.contains("should-not-persist"));
        assert!(!dumped.contains('-'));
    }

    #[test]
    fn restore_v7_compact_uppercase_then_generates_independently() {
        let mut view = UuidGenView::new();
        view.restore_options(&serde_json::json!({
            "version": "Seven",
            "hyphens": false,
            "uppercase": true,
            "count": 3
        }));
        assert_eq!(view.version, UuidVersion::Seven);
        assert!(!view.hyphens);
        assert!(view.uppercase);
        assert_eq!(view.count, "3");
        let lines: Vec<&str> = view.output.lines().collect();
        assert_eq!(lines.len(), 3);
        for line in &lines {
            assert_eq!(line.len(), 32);
            assert!(!line.contains('-'));
            assert_eq!(line.as_bytes()[12], b'7');
            assert!(line
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .all(|c| c.is_ascii_uppercase()));
            assert!(!line.chars().any(|c| c.is_ascii_lowercase()));
        }
    }

    #[test]
    fn restore_ignores_output_from_json() {
        let mut view = UuidGenView::new();
        view.restore_options(&serde_json::json!({
            "version": "Four",
            "hyphens": true,
            "uppercase": false,
            "count": 1,
            "output": "00000000-0000-4000-8000-000000000000"
        }));
        assert_ne!(view.output, "00000000-0000-4000-8000-000000000000");
        assert!(view.output.contains("-4"));
    }

    #[test]
    fn missing_fields_use_documented_defaults() {
        let mut view = UuidGenView::new();
        view.version = UuidVersion::One;
        view.hyphens = false;
        view.uppercase = true;
        view.count = "9".into();
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.version, UuidVersion::Four);
        assert!(view.hyphens);
        assert!(!view.uppercase);
        assert_eq!(view.count, "1");
        assert_eq!(view.output.lines().count(), 1);
        assert_eq!(view.output.as_bytes()[14], b'4');
        assert!(view.output.contains('-'));
        assert!(!view.output.chars().any(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn illegal_version_defaults_but_keeps_valid_count() {
        let mut view = UuidGenView::new();
        view.restore_options(&serde_json::json!({
            "version": "Five",
            "hyphens": false,
            "uppercase": false,
            "count": 2
        }));
        assert_eq!(view.version, UuidVersion::Four);
        assert_eq!(view.count, "2");
        let lines: Vec<&str> = view.output.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            assert_eq!(line.len(), 32);
            assert_eq!(line.as_bytes()[12], b'4');
        }
    }

    #[test]
    fn reconstruct_from_serialized_settings_object() {
        let mut first = UuidGenView::new();
        first.version = UuidVersion::Seven;
        first.hyphens = false;
        first.uppercase = true;
        first.count = "2".into();
        let (id, value) = first.persistable_options().unwrap();
        assert_eq!(id, "UUIDGenerator");

        let stored = serde_json::json!({
            "theme": "dark",
            "tool_options": {
                "JsonFormatter": { "indent": "minified" },
                id: value
            }
        });
        let mut second = UuidGenView::new();
        second.restore_options(&stored["tool_options"]["UUIDGenerator"]);
        assert_eq!(second.version, UuidVersion::Seven);
        let lines: Vec<&str> = second.output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].as_bytes()[12], b'7');
        let persisted = second.persistable_options().unwrap().1;
        assert_no_sensitive(&persisted);
        assert!(!persisted.to_string().contains(&second.output));
    }
}
