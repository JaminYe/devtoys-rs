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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default = "default_true")]
    pub smart_detection_enabled: bool,
    #[serde(default = "default_true")]
    pub smart_detection_paste: bool,
    #[serde(default)]
    pub favorites: Vec<String>,
    #[serde(default)]
    pub window: Option<WindowState>,
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
            favorites: Vec::new(),
            window: None,
        }
    }
}

impl AppSettings {
    /// Auto-paste is only effective while the master switch is on.
    pub fn paste_enabled(&self) -> bool {
        self.smart_detection_enabled && self.smart_detection_paste
    }
}
