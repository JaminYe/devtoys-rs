use std::fs;
use std::path::{Path, PathBuf};

use clap::Command;
use devtoys_api::GroupId;
use devtoys_tools::{build_cli_from, default_catalog, load_extensions, run_cli_from, ToolCatalog};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/extensions")
}

fn write_toml(root: &Path, folder: &str, body: &str) {
    let dir = root.join(folder);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("devtoys-extension.toml"), body).unwrap();
}

fn searchable(id: &str, display: &str, keywords: &[&str], query: &str) -> bool {
    let needle = query.to_lowercase();
    display.to_lowercase().contains(&needle)
        || id.to_lowercase().contains(&needle)
        || keywords.iter().any(|k| k.to_lowercase().contains(&needle))
}

#[test]
fn fixture_extension_is_in_catalog_searchable_and_opens() {
    let loaded = load_extensions(fixture_dir());
    assert!(
        loaded.errors.is_empty(),
        "fixture must be valid: {:?}",
        loaded.errors
    );
    let catalog = ToolCatalog::with_extensions(loaded.tools);
    assert_eq!(default_catalog().len(), 23);
    assert_eq!(catalog.len(), 24);

    let meta = catalog
        .all_metadata()
        .into_iter()
        .find(|tool| tool.id.as_str() == "EchoUpper")
        .expect("fixture tool");
    assert_eq!(meta.display_name, "Echo Upper");
    assert_eq!(meta.group, GroupId::Text);
    assert!(meta.searchable);
    assert!(meta.favorable);
    assert!(searchable(
        meta.id.as_str(),
        meta.display_name,
        meta.search_keywords,
        "echo"
    ));
    assert!(searchable(
        meta.id.as_str(),
        meta.display_name,
        meta.search_keywords,
        "uppercase"
    ));
    assert!(catalog
        .all_cli()
        .iter()
        .any(|cli| cli.name == "EchoUpper" && cli.tool_id == "EchoUpper"));

    #[cfg(feature = "gui")]
    {
        let mut view = catalog.open_view("EchoUpper").expect("open extension");
        view.on_data_received("hello");
    }
}

#[test]
fn remove_files_then_reload_drops_extension() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        dir.path(),
        "echo_upper",
        r#"
id = "TempEcho"
display_name = "Temp Echo"
group = "Text"
search_keywords = ["temp-echo"]
"#,
    );
    let first = load_extensions(dir.path());
    assert_eq!(first.tools.len(), 1);
    fs::remove_dir_all(dir.path().join("echo_upper")).unwrap();
    let second = load_extensions(dir.path());
    assert!(second.tools.is_empty());
    assert!(second.errors.is_empty());
    assert_eq!(default_catalog().len(), 23);
}

#[test]
fn corrupt_manifest_is_isolated_builtins_stay_23() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(dir.path(), "broken", "not = [ toml");
    write_toml(
        dir.path(),
        "ok",
        r#"
id = "OkExt"
display_name = "OK"
group = "Generators"
"#,
    );
    let loaded = load_extensions(dir.path());
    assert_eq!(loaded.tools.len(), 1);
    assert_eq!(loaded.errors.len(), 1);
    let catalog = ToolCatalog::with_extensions(loaded.tools);
    assert_eq!(catalog.len(), 24);
    assert_eq!(default_catalog().len(), 23);
}

#[test]
fn duplicate_builtin_id_skipped() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        dir.path(),
        "dup",
        r#"
id = "JsonFormatter"
display_name = "Impostor"
group = "Formatters"
"#,
    );
    let loaded = load_extensions(dir.path());
    assert!(loaded.tools.is_empty());
    assert_eq!(loaded.errors.len(), 1);
    assert!(loaded.errors[0].message.contains("JsonFormatter"));
    assert_eq!(default_catalog().len(), 23);
    assert_eq!(ToolCatalog::with_extensions(loaded.tools).len(), 23);
}

#[test]
fn default_catalog_len_is_still_23() {
    assert_eq!(default_catalog().len(), 23);
    assert_eq!(ToolCatalog::default_catalog().len(), 23);
}

#[test]
fn extension_cli_uppercase_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        dir.path(),
        "echo_upper",
        r#"
id = "CliEcho"
display_name = "CLI Echo"
group = "Text"
cli = "CliEcho"
"#,
    );
    let loaded = load_extensions(dir.path());
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    let catalog = ToolCatalog::with_extensions(loaded.tools);
    let out = dir.path().join("out.txt");
    let cmd: Command = build_cli_from(&catalog);
    let matches = cmd
        .try_get_matches_from([
            "devtoys-cli",
            "CliEcho",
            "-i",
            "hello",
            "-m",
            "uppercase",
            "-o",
            out.to_str().unwrap(),
        ])
        .expect("extension CLI should parse");
    run_cli_from(&catalog, &matches).unwrap();
    assert_eq!(fs::read_to_string(out).unwrap(), "HELLO");
}

#[test]
fn builtin_cli_untouched_when_extensions_present() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        dir.path(),
        "echo_upper",
        r#"
id = "CliEcho2"
display_name = "CLI Echo 2"
group = "Text"
cli = "CliEcho2"
"#,
    );
    let catalog = ToolCatalog::with_extensions(load_extensions(dir.path()).tools);
    let json_cli = catalog
        .all_cli()
        .into_iter()
        .find(|cli| cli.tool_id == "JsonFormatter")
        .expect("JsonFormatter CLI stays");
    assert_eq!(json_cli.name, "JsonFormatter");
    assert!(catalog.all_cli().iter().any(|cli| cli.name == "CliEcho2"));
    assert_eq!(default_catalog().all_cli().len(), 19);
}
