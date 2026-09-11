use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
}

/// User preferences persisted as `devtoys-rs/settings.json`.
///
/// `smart_detection_paste` is ignored at runtime when `smart_detection_enabled`
/// is false. Favorites tool IDs are stored here so they survive restart.
///
/// `tool_options` is keyed by tool id; each value is that tool's nested object.
/// JSON formatter currently stores `{ "indent": "two_spaces", "sort_properties": false }`
/// (`indent` is `two_spaces` | `four_spaces` | `one_tab` | `minified`). Input/output
/// content is never stored. Missing `tool_options` (old files) loads as empty.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default = "default_true")]
    pub smart_detection_enabled: bool,
    #[serde(default = "default_true")]
    pub smart_detection_paste: bool,
    #[serde(default = "default_true")]
    pub auto_check_updates: bool,
    #[serde(default)]
    pub favorites: Vec<String>,
    #[serde(default)]
    pub window: Option<WindowState>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tool_options: BTreeMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            smart_detection_enabled: true,
            smart_detection_paste: true,
            auto_check_updates: true,
            favorites: Vec::new(),
            window: None,
            tool_options: BTreeMap::new(),
        }
    }
}

impl AppSettings {
    /// Auto-paste is only effective while the master switch is on.
    pub fn paste_enabled(&self) -> bool {
        self.smart_detection_enabled && self.smart_detection_paste
    }

    pub fn tool_options(&self, tool_id: &str) -> Option<&serde_json::Value> {
        self.tool_options.get(tool_id)
    }

    pub fn set_tool_options(&mut self, tool_id: impl Into<String>, value: serde_json::Value) {
        self.tool_options.insert(tool_id.into(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_file_without_tool_options_keeps_globals() {
        let json = r#"{
            "theme": "dark",
            "smart_detection_enabled": false,
            "smart_detection_paste": true,
            "favorites": ["JsonFormatter"],
            "window": {"x": 1.0, "y": 2.0, "width": 800.0, "height": 600.0, "maximized": true}
        }"#;
        let settings: AppSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.theme, ThemePreference::Dark);
        assert!(!settings.smart_detection_enabled);
        assert!(settings.smart_detection_paste);
        assert!(settings.auto_check_updates);
        assert_eq!(settings.favorites, ["JsonFormatter"]);
        assert_eq!(
            settings.window,
            Some(WindowState {
                x: 1.0,
                y: 2.0,
                width: 800.0,
                height: 600.0,
                maximized: true,
            })
        );
        assert!(settings.tool_options.is_empty());
    }

    #[test]
    fn missing_tool_option_fields_stay_absent_for_view_defaults() {
        let json = r#"{"tool_options":{"JsonFormatter":{}}}"#;
        let settings: AppSettings = serde_json::from_str(json).unwrap();
        let opts = settings.tool_options("JsonFormatter").unwrap();
        assert!(opts.get("indent").is_none());
        assert!(opts.get("sort_properties").is_none());
    }

    #[test]
    fn illegal_indent_is_preserved_for_the_view_to_default() {
        let json = r#"{"tool_options":{"JsonFormatter":{"indent":"eight_spaces","sort_properties":true}}}"#;
        let settings: AppSettings = serde_json::from_str(json).unwrap();
        let opts = settings.tool_options("JsonFormatter").unwrap();
        assert_eq!(opts["indent"], "eight_spaces");
        assert_eq!(opts["sort_properties"], true);
    }

    #[test]
    fn auto_check_updates_can_be_disabled_and_persisted() {
        let json = r#"{"auto_check_updates": false}"#;
        let settings: AppSettings = serde_json::from_str(json).unwrap();
        assert!(!settings.auto_check_updates);

        let serialized = serde_json::to_string(&settings).unwrap();
        assert!(serialized.contains(r#""auto_check_updates":false"#));
    }
}
