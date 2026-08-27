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
    saved.recent = vec!["c".into()];
    store.save(&saved).unwrap();

    let state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));
    assert_eq!(state.settings().theme, ThemePreference::Light);
    assert_eq!(ids(state.favorite_tools()), vec!["b"]);
    assert_eq!(ids(state.recent_tools()), vec!["c"]);
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
fn open_tool_records_recent_max_three_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));

    state.open_tool("a").unwrap();
    state.open_tool("b").unwrap();
    state.open_tool("c").unwrap();
    assert_eq!(ids(state.recent_tools()), vec!["c", "b", "a"]);

    state.open_tool("d").unwrap();
    assert_eq!(ids(state.recent_tools()), vec!["d", "c", "b"]);

    state.open_tool("c").unwrap();
    assert_eq!(ids(state.recent_tools()), vec!["c", "d", "b"]);

    let reloaded = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));
    assert_eq!(ids(reloaded.recent_tools()), vec!["c", "d", "b"]);
}

#[test]
fn open_unknown_id_does_not_record_recent() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState::bootstrap(vec![meta("a", true)], SettingsStore::in_dir(dir.path()));

    state.open_tool(SETTINGS_ID).unwrap();
    state.open_tool("ghost").unwrap();
    assert!(state.recent_tools().is_empty());
    assert!(state.settings().recent.is_empty());
}

#[test]
fn recent_tools_empty_when_hidden_or_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::in_dir(dir.path());
    let mut saved = AppSettings::default();
    saved.recent = vec!["ghost".into(), "a".into(), "b".into()];
    store.save(&saved).unwrap();

    let mut state = AppState::bootstrap(
        vec![meta("a", true), meta("b", true)],
        SettingsStore::in_dir(dir.path()),
    );
    assert_eq!(ids(state.recent_tools()), vec!["a", "b"]);

    state.set_show_recent(false).unwrap();
    assert!(state.recent_tools().is_empty());
    assert_eq!(state.settings().recent, vec!["ghost", "a", "b"]);

    let reloaded = AppState::bootstrap(
        vec![meta("a", true), meta("b", true)],
        SettingsStore::in_dir(dir.path()),
    );
    assert!(!reloaded.settings().show_recent);
    assert!(reloaded.recent_tools().is_empty());
}

#[test]
fn setters_persist_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));

    state.set_theme(ThemePreference::Dark).unwrap();
    state.set_smart_detection_paste(false).unwrap();
    state.set_show_recent(false).unwrap();

    let reloaded = AppState::bootstrap(tools(), SettingsStore::in_dir(dir.path()));
    assert_eq!(reloaded.settings().theme, ThemePreference::Dark);
    assert!(!reloaded.settings().smart_detection_paste);
    assert!(!reloaded.settings().show_recent);
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
