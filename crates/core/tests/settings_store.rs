use std::fs;

use devtoys_api::{AppSettings, ThemePreference, WindowState};
use devtoys_core::{AppState, SettingsStore};

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
    let mut tool_options = std::collections::BTreeMap::new();
    tool_options.insert(
        "JsonFormatter".into(),
        serde_json::json!({
            "indent": "four_spaces",
            "sort_properties": true
        }),
    );
    let settings = AppSettings {
        theme: ThemePreference::Dark,
        smart_detection_enabled: false,
        smart_detection_paste: false,
        auto_check_updates: true,
        include_prerelease: false,
        favorites: vec!["JsonFormatter".into(), "TextTool".into()],
        window: Some(WindowState {
            x: 12.0,
            y: 34.0,
            width: 800.0,
            height: 600.0,
            maximized: true,
        }),
        tool_options,
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

#[test]
fn old_file_without_tool_options_keeps_globals() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    fs::write(
        store.path(),
        r#"{
            "theme": "light",
            "smart_detection_enabled": false,
            "smart_detection_paste": true,
            "favorites": ["JsonFormatter"],
            "window": {"x": 8.0, "y": 9.0, "width": 400.0, "height": 300.0, "maximized": false}
        }"#,
    )
    .unwrap();

    let loaded = store.load();
    assert_eq!(loaded.theme, ThemePreference::Light);
    assert!(!loaded.smart_detection_enabled);
    assert!(loaded.smart_detection_paste);
    assert_eq!(loaded.favorites, ["JsonFormatter"]);
    assert_eq!(
        loaded.window,
        Some(WindowState {
            x: 8.0,
            y: 9.0,
            width: 400.0,
            height: 300.0,
            maximized: false,
        })
    );
    assert!(loaded.tool_options.is_empty());
}

#[test]
fn tool_options_are_nested_objects_keyed_by_tool_id() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    let mut settings = AppSettings::default();
    settings.set_tool_options(
        "JsonFormatter",
        serde_json::json!({"indent": "one_tab", "sort_properties": true}),
    );
    settings.set_tool_options("XmlFormatter", serde_json::json!({"indent": "four_spaces"}));
    store.save(&settings).unwrap();

    let raw = fs::read_to_string(store.path()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(
        parsed.get("JsonFormatter_indent").is_none(),
        "must not flatten tools into undifferentiated keys"
    );
    assert_eq!(parsed["tool_options"]["JsonFormatter"]["indent"], "one_tab");
    assert_eq!(
        parsed["tool_options"]["JsonFormatter"]["sort_properties"],
        true
    );
    assert_eq!(
        parsed["tool_options"]["XmlFormatter"]["indent"],
        "four_spaces"
    );

    let loaded = store.load();
    assert_eq!(
        loaded.tool_options("JsonFormatter").unwrap()["indent"],
        "one_tab"
    );
    assert_eq!(
        loaded.tool_options("XmlFormatter").unwrap()["indent"],
        "four_spaces"
    );
}

#[test]
fn updating_one_tool_does_not_drop_another_or_globals() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    let mut state = AppState::bootstrap(Vec::new(), store.clone());
    state.set_theme(ThemePreference::Dark).unwrap();
    state
        .set_tool_options("XmlFormatter", serde_json::json!({"indent": "minified"}))
        .unwrap();
    state
        .set_tool_options(
            "JsonFormatter",
            serde_json::json!({"indent": "four_spaces", "sort_properties": true}),
        )
        .unwrap();

    let reloaded = AppState::bootstrap(Vec::new(), store);
    assert_eq!(reloaded.settings().theme, ThemePreference::Dark);
    assert_eq!(
        reloaded.settings().tool_options("XmlFormatter").unwrap()["indent"],
        "minified"
    );
    assert_eq!(
        reloaded.settings().tool_options("JsonFormatter").unwrap()["sort_properties"],
        true
    );
}

#[test]
fn missing_and_illegal_option_values_remain_loadable() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    fs::write(
        store.path(),
        r#"{
            "theme": "dark",
            "favorites": ["JsonFormatter"],
            "tool_options": {
                "JsonFormatter": {"indent": "not_an_indent", "extra": 1}
            }
        }"#,
    )
    .unwrap();

    let loaded = store.load();
    assert_eq!(loaded.theme, ThemePreference::Dark);
    assert_eq!(loaded.favorites, ["JsonFormatter"]);
    let opts = loaded.tool_options("JsonFormatter").unwrap();
    assert_eq!(opts["indent"], "not_an_indent");
    assert!(opts.get("sort_properties").is_none());
    assert_eq!(opts["extra"], 1);
}

#[test]
fn failed_write_does_not_corrupt_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    let mut original = AppSettings::default();
    original.theme = ThemePreference::Light;
    original.set_tool_options(
        "JsonFormatter",
        serde_json::json!({"indent": "two_spaces", "sort_properties": false}),
    );
    store.save(&original).unwrap();
    let before = fs::read(store.path()).unwrap();

    let tmp = {
        let mut raw = store.path().as_os_str().to_os_string();
        raw.push(".tmp");
        std::path::PathBuf::from(raw)
    };
    fs::create_dir(&tmp).unwrap();

    let mut next = original.clone();
    next.theme = ThemePreference::Dark;
    next.set_tool_options(
        "JsonFormatter",
        serde_json::json!({"indent": "minified", "sort_properties": true}),
    );
    assert!(store.save(&next).is_err());
    assert_eq!(fs::read(store.path()).unwrap(), before);
    assert_eq!(store.load(), original);
}

#[test]
fn legacy_language_setting_loads_and_saves_without_language_key() {
    for lang_val in [
        "\"en_us\"",
        "\"en-US\"",
        "\"system\"",
        "\"zh_cn\"",
        "\"zh-CN\"",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::in_dir(dir.path());
        let legacy_json = format!(
            r#"{{
            "theme": "dark",
            "language": {lang_val},
            "smart_detection_enabled": false,
            "smart_detection_paste": false,
            "favorites": ["JsonFormatter"]
        }}"#
        );
        fs::write(store.path(), legacy_json).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.theme, ThemePreference::Dark);
        assert_eq!(loaded.favorites, ["JsonFormatter"]);

        // Saving the loaded settings writes back without the language key
        store.save(&loaded).unwrap();
        let re_read = fs::read_to_string(store.path()).unwrap();
        assert!(
            !re_read.contains("language"),
            "Saved settings should not contain language key: {re_read}"
        );
    }
}
