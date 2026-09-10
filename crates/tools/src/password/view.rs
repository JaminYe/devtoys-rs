use crate::slot::ToolView;
use crate::ui;

use super::{generate_password, PasswordOptions, ID};

const DEFAULT_LENGTH: usize = 16;
const DEFAULT_COUNT: usize = 1;
const DEFAULT_UPPERCASE: bool = true;
const DEFAULT_LOWERCASE: bool = true;
const DEFAULT_DIGITS: bool = true;
const DEFAULT_SPECIAL: bool = true;

pub struct PasswordView {
    length: String,
    count: String,
    exclude: String,
    output: String,
    uppercase: bool,
    lowercase: bool,
    digits: bool,
    special: bool,
    error: Option<String>,
}

impl PasswordView {
    pub fn new() -> Self {
        let mut this = Self {
            length: DEFAULT_LENGTH.to_string(),
            count: DEFAULT_COUNT.to_string(),
            exclude: String::new(),
            output: String::new(),
            uppercase: DEFAULT_UPPERCASE,
            lowercase: DEFAULT_LOWERCASE,
            digits: DEFAULT_DIGITS,
            special: DEFAULT_SPECIAL,
            error: None,
        };
        this.regenerate();
        this
    }

    fn options(&self) -> PasswordOptions {
        PasswordOptions {
            length: self.length.parse::<usize>().unwrap_or(DEFAULT_LENGTH),
            uppercase: self.uppercase,
            lowercase: self.lowercase,
            digits: self.digits,
            special: self.special,
            exclude: self.exclude.clone(),
            count: self.count.parse::<usize>().unwrap_or(DEFAULT_COUNT).max(1),
        }
    }

    fn regenerate(&mut self) {
        match generate_password(&self.options(), &mut rand::thread_rng()) {
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

impl ToolView for PasswordView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            dirty |= ui
                .checkbox(&mut self.uppercase, ui::t("password.uppercase"))
                .changed();
            dirty |= ui
                .checkbox(&mut self.lowercase, ui::t("password.lowercase"))
                .changed();
            dirty |= ui
                .checkbox(&mut self.digits, ui::t("password.digits"))
                .changed();
            dirty |= ui
                .checkbox(&mut self.special, ui::t("password.special"))
                .changed();
            ui.label(ui::t("password.length"));
            ui.allocate_ui(egui::vec2(72.0, ui.spacing().interact_size.y), |ui| {
                dirty |= ui::singleline(ui, "pwd-len", &mut self.length, "长度");
            });
            ui.label("数量");
            ui.allocate_ui(egui::vec2(72.0, ui.spacing().interact_size.y), |ui| {
                dirty |= ui::singleline(ui, "pwd-count", &mut self.count, "数量");
            });
            if ui.button(ui::t("password.generate")).clicked() {
                dirty = true;
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui.label("排除字符");
        dirty |= ui::singleline(ui, "pwd-exclude", &mut self.exclude, "排除字符");
        if dirty {
            self.regenerate();
        }
        ui::error_label(ui, self.error.as_deref());
        ui::labeled_code(
            ui,
            ui::t("common.output"),
            "pwd-out",
            &mut self.output,
            "生成结果",
            false,
        );
    }

    fn on_data_received(&mut self, _: &str) {}

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        let opts = self.options();
        Some((
            ID.to_string(),
            serde_json::json!({
                "length": opts.length,
                "count": opts.count,
                "uppercase": self.uppercase,
                "lowercase": self.lowercase,
                "digits": self.digits,
                "special": self.special,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.length = usize_from_settings(value.get("length"), DEFAULT_LENGTH).to_string();
        self.count = count_from_settings(value.get("count"), DEFAULT_COUNT).to_string();
        self.uppercase = value
            .get("uppercase")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_UPPERCASE);
        self.lowercase = value
            .get("lowercase")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_LOWERCASE);
        self.digits = value
            .get("digits")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_DIGITS);
        self.special = value
            .get("special")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_SPECIAL);
        self.regenerate();
    }
}

fn usize_from_settings(value: Option<&serde_json::Value>, default: usize) -> usize {
    value
        .and_then(|v| v.as_u64())
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(default)
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
        assert!(value.get("exclude").is_none());
        assert!(value.get("input").is_none());
    }

    #[test]
    fn persistable_options_are_length_count_and_classes_only() {
        let mut view = PasswordView::new();
        view.length = "32".into();
        view.count = "3".into();
        view.uppercase = false;
        view.lowercase = false;
        view.digits = true;
        view.special = false;
        view.exclude = "abc".into();
        view.output = "hunter2-should-not-persist".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["length"], 32);
        assert_eq!(value["count"], 3);
        assert_eq!(value["uppercase"], false);
        assert_eq!(value["lowercase"], false);
        assert_eq!(value["digits"], true);
        assert_eq!(value["special"], false);
        assert_no_sensitive(&value);
        let dumped = value.to_string();
        assert!(!dumped.contains("hunter2"));
        assert!(!dumped.contains("abc"));
    }

    #[test]
    fn restore_length_and_digits_then_generates_independently() {
        let mut view = PasswordView::new();
        view.restore_options(&serde_json::json!({
            "length": 8,
            "count": 2,
            "uppercase": false,
            "lowercase": false,
            "digits": true,
            "special": false
        }));
        assert_eq!(view.length, "8");
        assert_eq!(view.count, "2");
        assert!(!view.uppercase);
        assert!(!view.lowercase);
        assert!(view.digits);
        assert!(!view.special);
        let lines: Vec<&str> = view.output.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            assert_eq!(line.chars().count(), 8);
            assert!(line.chars().all(|c| c.is_ascii_digit()), "{line}");
        }
        assert_ne!(view.output, "hunter2");
        assert!(view.exclude.is_empty());
    }

    #[test]
    fn restore_ignores_output_and_exclude_from_json() {
        let mut view = PasswordView::new();
        view.exclude = "keep-x".into();
        view.restore_options(&serde_json::json!({
            "length": 10,
            "count": 1,
            "uppercase": false,
            "lowercase": false,
            "digits": true,
            "special": false,
            "exclude": "injected",
            "output": "hunter2"
        }));
        assert_eq!(view.exclude, "keep-x");
        assert_ne!(view.output, "hunter2");
        assert_eq!(view.length, "10");
    }

    #[test]
    fn missing_fields_use_documented_defaults() {
        let mut view = PasswordView::new();
        view.length = "32".into();
        view.count = "5".into();
        view.uppercase = false;
        view.lowercase = false;
        view.digits = true;
        view.special = false;
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.length, "16");
        assert_eq!(view.count, "1");
        assert!(view.uppercase);
        assert!(view.lowercase);
        assert!(view.digits);
        assert!(view.special);
        assert_eq!(view.output.lines().count(), 1);
        assert_eq!(view.output.chars().count(), 16);
    }

    #[test]
    fn illegal_count_defaults_but_keeps_valid_length() {
        let mut view = PasswordView::new();
        view.restore_options(&serde_json::json!({
            "length": 12,
            "count": 0,
            "uppercase": false,
            "lowercase": true,
            "digits": false,
            "special": false
        }));
        assert_eq!(view.length, "12");
        assert_eq!(view.count, "1");
        assert_eq!(view.output.chars().count(), 12);
        assert!(view.output.chars().all(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn reconstruct_from_serialized_settings_object() {
        let mut first = PasswordView::new();
        first.length = "24".into();
        first.digits = true;
        first.uppercase = false;
        first.lowercase = false;
        first.special = false;
        let (id, value) = first.persistable_options().unwrap();
        assert_eq!(id, "PasswordGenerator");

        let stored = serde_json::json!({
            "theme": "dark",
            "tool_options": {
                "JsonFormatter": { "indent": "minified" },
                id: value
            }
        });
        let mut second = PasswordView::new();
        second.restore_options(&stored["tool_options"]["PasswordGenerator"]);
        assert_eq!(second.length, "24");
        assert_eq!(second.output.chars().count(), 24);
        assert!(second.output.chars().all(|c| c.is_ascii_digit()));
        let persisted = second.persistable_options().unwrap().1;
        assert_no_sensitive(&persisted);
        assert!(!persisted.to_string().contains(&second.output));
    }
}
