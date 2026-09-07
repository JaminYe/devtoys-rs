use std::fs;

use devtoys_api::{AppSettings, ThemePreference, WindowState};
use devtoys_core::SettingsStore;

#[test]
fn missing_or_corrupt_file_yields_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    assert_eq!(store.load(), AppSettings::default());

    fs::write(store.path(), "not-json{{{").unwrap();
    assert_eq!(store.load(), AppSettings::default());
}

#[test]
fn round_trips_all_settings_fields() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    let settings = AppSettings {
        theme: ThemePreference::Dark,
        smart_detection_enabled: false,
        smart_detection_paste: false,
        favorites: vec!["JsonFormatter".into(), "TextTool".into()],
        window: Some(WindowState {
            x: 12.0,
            y: 34.0,
            width: 800.0,
            height: 600.0,
            maximized: true,
        }),
    };
    store.save(&settings).unwrap();
    assert_eq!(store.load(), settings);
}

#[test]
fn save_creates_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a").join("b");
    let store = SettingsStore::in_dir(&nested);
    store.save(&AppSettings::default()).unwrap();
    assert!(nested.join("settings.json").is_file());
}

#[test]
fn user_store_path_is_under_devtoys_rs() {
    let store = SettingsStore::user();
    let path = store.path();
    assert_eq!(
        path.file_name().and_then(|s| s.to_str()),
        Some("settings.json")
    );
    assert_eq!(
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str()),
        Some("devtoys-rs")
    );
}
