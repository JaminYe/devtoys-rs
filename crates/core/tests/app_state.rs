use devtoys_api::{AppSettings, GroupId, ThemePreference, ToolId, ToolMetadata, SETTINGS_ID};
use devtoys_core::{AppState, CoreError, SettingsStore};

fn meta(id: &'static str, favorable: bool) -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(id),
        display_name: id,
        search_keywords: &[],
        group: GroupId::Text,
        searchable: true,
        favorable,
        accepted_types: &[],
    }
}

fn ids(tools: Vec<&ToolMetadata>) -> Vec<&str> {
    tools.iter().map(|t| t.id.as_str()).collect()
}

fn tools() -> Vec<ToolMetadata> {
    vec![
        meta("a", true),
        meta("b", true),
        meta("c", true),
        meta("d", true),
        meta(SETTINGS_ID, false),
    ]
}

#[test]
fn bootstrap_loads_defaults_when_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));
    assert_eq!(state.settings(), &AppSettings::default());
    assert!(!dir.path().join("settings.json").exists());
}

#[test]
fn bootstrap_loads_existing_settings() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    let mut saved = AppSettings::default();
    saved.theme = ThemePreference::Light;
    saved.favorites = vec!["b".into()];
    store.save(&saved).unwrap();

    let state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));
    assert_eq!(state.settings().theme, ThemePreference::Light);
    assert_eq!(ids(state.favorite_tools()), vec!["b"]);
}

#[test]
fn toggle_favorite_requires_known_favorable_tool() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));

    let err = state.toggle_favorite("missing").unwrap_err();
    assert!(matches!(err, CoreError::UnknownTool(id) if id == "missing"));
    assert!(state.favorite_tools().is_empty());

    let err = state.toggle_favorite(SETTINGS_ID).unwrap_err();
    assert!(matches!(err, CoreError::NotFavorable(id) if id == SETTINGS_ID));
    assert!(!state.is_favorite(SETTINGS_ID));
}

#[test]
fn toggle_favorite_persists_in_list_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));

    state.toggle_favorite("b").unwrap();
    state.toggle_favorite("a").unwrap();
    assert_eq!(ids(state.favorite_tools()), vec!["b", "a"]);
    assert!(state.is_favorite("a"));
    assert!(state.is_favorite("b"));
    assert!(!state.is_favorite("c"));

    state.toggle_favorite("b").unwrap();
    assert_eq!(ids(state.favorite_tools()), vec!["a"]);

    let reloaded = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));
    assert_eq!(ids(reloaded.favorite_tools()), vec!["a"]);
}

#[test]
fn setters_persist_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));

    state.set_theme(ThemePreference::Dark).unwrap();
    state.set_smart_detection_paste(false).unwrap();

    let reloaded = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));
    assert_eq!(reloaded.settings().theme, ThemePreference::Dark);
    assert!(!reloaded.settings().smart_detection_paste);
    assert!(reloaded.settings().smart_detection_enabled);
}

#[test]
fn master_detection_off_is_persisted_and_disables_paste() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));

    state.set_smart_detection_paste(true).unwrap();
    state.set_smart_detection_enabled(false).unwrap();
    assert!(!state.settings().smart_detection_enabled);
    assert!(state.settings().smart_detection_paste);
    assert!(!state.settings().paste_enabled());

    let reloaded = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));
    assert!(!reloaded.settings().smart_detection_enabled);
    assert!(reloaded.settings().smart_detection_paste);
    assert!(!reloaded.settings().paste_enabled());
}

#[test]
fn successful_save_matches_disk_and_memory() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    let mut state = AppState::bootstrap(tools(), store.clone());

    state.set_theme(ThemePreference::Dark).unwrap();
    state.set_smart_detection_enabled(false).unwrap();
    state.set_smart_detection_paste(false).unwrap();
    state.toggle_favorite("b").unwrap();

    let reloaded = AppState::bootstrap(tools(), store);
    assert_eq!(reloaded.settings().theme, ThemePreference::Dark);
    assert!(!reloaded.settings().smart_detection_enabled);
    assert!(!reloaded.settings().smart_detection_paste);
    assert!(reloaded.is_favorite("b"));

    // In-memory state matches what was persisted to disk.
    assert_eq!(state.settings().theme, reloaded.settings().theme);
    assert_eq!(state.settings().favorites, reloaded.settings().favorites);
}

#[test]
fn failed_save_keeps_theme_in_memory_but_not_disk() {
    // A parent that is a regular file makes create_dir_all/write fail.
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    let store = SettingsStore::in_dir(blocker.join("sub"));

    let mut state = AppState::bootstrap(tools(), store.clone());
    let err = state.set_theme(ThemePreference::Dark).unwrap_err();
    assert!(matches!(err, CoreError::Io(_)));

    // Optimistic update: memory keeps the change even though saving failed.
    assert_eq!(state.settings().theme, ThemePreference::Dark);

    // Disk was never written, so a fresh bootstrap reads the default.
    let reloaded = AppState::bootstrap(tools(), store);
    assert_eq!(reloaded.settings().theme, ThemePreference::System);
}

#[test]
fn failed_save_keeps_favorite_in_memory_but_not_disk() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    let store = SettingsStore::in_dir(blocker.join("sub"));

    let mut state = AppState::bootstrap(tools(), store.clone());
    let err = state.toggle_favorite("b").unwrap_err();
    assert!(matches!(err, CoreError::Io(_)));

    // Optimistic update: the favorite is kept in memory even on failure.
    assert!(state.is_favorite("b"));

    // Disk was never written, so a fresh bootstrap does not contain it.
    let reloaded = AppState::bootstrap(tools(), store);
    assert!(!reloaded.is_favorite("b"));
}
