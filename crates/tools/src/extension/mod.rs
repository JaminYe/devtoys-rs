//! Local directory extensions loaded at process start (no store, no hot-reload).
//!
//! Layout: `{data_dir}/devtoys-rs/extensions/<name>/devtoys-extension.toml`
//! (Windows: `%APPDATA%/devtoys-rs/extensions`). JSON manifests are also accepted.
//!
//! ```toml
//! id = "EchoUpper"
//! display_name = "Echo Upper"
//! group = "Text" # Converters | EncodersDecoders | Formatters | Generators | Graphic | Testers | Text
//! search_keywords = ["echo", "uppercase"]
//! cli = "EchoUpper" # optional; generic echo/uppercase dispatcher
//! ```
//!
//! Implementation is an in-process [`ManifestTool`] (text → echo / uppercase),
//! not a dylib/WASM ABI. Dynamic strings are leaked so [`ToolMetadata`] stays `'static`.

mod cli;
mod helper;

#[cfg(feature = "gui")]
mod view;

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use crate::ToolCatalog;
use devtoys_api::{GroupId, ToolId, ToolMetadata};

const MANIFEST_TOML: &str = "devtoys-extension.toml";
const MANIFEST_JSON: &str = "devtoys-extension.json";

/// Default local extension directory next to `settings.json`.
///
/// Windows: `%APPDATA%/devtoys-rs/extensions`.
pub fn default_extensions_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("devtoys-rs")
        .join("extensions")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionLoadError {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for ExtensionLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

pub struct ExtensionLoadResult {
    pub tools: Vec<Box<dyn Tool>>,
    pub errors: Vec<ExtensionLoadError>,
}

/// Loads manifest tools from `dir`. Missing directories yield an empty result.
/// Broken manifests, unknown groups, and ids that collide with builtins or
/// earlier extensions are skipped and recorded; the caller still starts.
pub fn load_extensions(dir: impl AsRef<Path>) -> ExtensionLoadResult {
    let dir = dir.as_ref();
    let mut result = ExtensionLoadResult {
        tools: Vec::new(),
        errors: Vec::new(),
    };
    if !dir.exists() {
        return result;
    }
    if !dir.is_dir() {
        result.errors.push(load_error(dir, "扩展路径不是目录"));
        return result;
    }

    let mut reserved_ids: HashSet<String> = crate::default_catalog()
        .all_metadata()
        .into_iter()
        .map(|meta| meta.id.as_str().to_string())
        .collect();
    let mut reserved_cli: HashSet<String> = crate::default_catalog()
        .all_cli()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();

    for candidate in extension_dirs(dir) {
        match load_one(&candidate, &reserved_ids, &reserved_cli) {
            Ok(LoadedExtension {
                tool,
                cli_name,
                warning,
            }) => {
                reserved_ids.insert(tool.metadata().id.as_str().to_string());
                if let Some(name) = cli_name {
                    reserved_cli.insert(name.to_string());
                }
                if let Some(warning) = warning {
                    result.errors.push(warning);
                }
                result.tools.push(Box::new(tool));
            }
            Err(err) => result.errors.push(err),
        }
    }
    result
}

struct LoadedExtension {
    tool: ManifestTool,
    cli_name: Option<&'static str>,
    warning: Option<ExtensionLoadError>,
}

#[derive(Debug, Deserialize)]
struct ExtensionManifest {
    id: String,
    display_name: String,
    group: String,
    #[serde(default)]
    search_keywords: Vec<String>,
    #[serde(default)]
    cli: Option<String>,
}

fn extension_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && manifest_file(&path).is_some() {
                dirs.push(path);
            }
        }
    }
    dirs.sort();
    if dirs.is_empty() && manifest_file(root).is_some() {
        dirs.push(root.to_path_buf());
    }
    dirs
}

fn manifest_file(dir: &Path) -> Option<PathBuf> {
    let toml = dir.join(MANIFEST_TOML);
    if toml.is_file() {
        return Some(toml);
    }
    let json = dir.join(MANIFEST_JSON);
    if json.is_file() {
        return Some(json);
    }
    None
}

fn load_one(
    dir: &Path,
    reserved_ids: &HashSet<String>,
    reserved_cli: &HashSet<String>,
) -> Result<LoadedExtension, ExtensionLoadError> {
    let path = manifest_file(dir).ok_or_else(|| load_error(dir, "缺少扩展清单"))?;
    let text = fs::read_to_string(&path)
        .map_err(|err| load_error(&path, format!("无法读取清单: {err}")))?;
    let manifest = parse_manifest(&path, &text)?;
    let id = manifest.id.trim();
    if id.is_empty() {
        return Err(load_error(&path, "id 不能为空"));
    }
    if reserved_ids.contains(id) {
        return Err(load_error(
            &path,
            format!("id `{id}` 与已注册工具冲突，已跳过"),
        ));
    }
    let display_name = manifest.display_name.trim();
    if display_name.is_empty() {
        return Err(load_error(&path, "display_name 不能为空"));
    }
    let group = parse_group(&manifest.group)
        .ok_or_else(|| load_error(&path, format!("未知分组 `{}`", manifest.group.trim())))?;

    let mut warning = None;
    let cli_name = match manifest
        .cli
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(name) if name.chars().any(char::is_whitespace) => {
            warning = Some(load_error(
                &path,
                format!("cli 名称 `{name}` 含空白，已忽略 CLI"),
            ));
            None
        }
        Some(name) if reserved_cli.contains(name) => {
            warning = Some(load_error(
                &path,
                format!("cli 名称 `{name}` 冲突，已忽略 CLI"),
            ));
            None
        }
        Some(name) => Some(leak_str(name)),
        None => None,
    };

    let keywords: Vec<String> = manifest
        .search_keywords
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();

    let tool = ManifestTool {
        id: leak_str(id),
        display_name: leak_str(display_name),
        search_keywords: leak_keywords(keywords),
        group,
        cli_name,
    };
    Ok(LoadedExtension {
        tool,
        cli_name,
        warning,
    })
}

fn parse_manifest(path: &Path, text: &str) -> Result<ExtensionManifest, ExtensionLoadError> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if ext.eq_ignore_ascii_case("json") {
        serde_json::from_str(text).map_err(|err| load_error(path, format!("清单 JSON 无效: {err}")))
    } else {
        toml::from_str(text).map_err(|err| load_error(path, format!("清单 TOML 无效: {err}")))
    }
}

fn parse_group(raw: &str) -> Option<GroupId> {
    let key = raw.trim();
    GroupId::ALL
        .into_iter()
        .find(|group| group.key().eq_ignore_ascii_case(key) || group.display_name() == key)
}

fn load_error(path: impl AsRef<Path>, message: impl Into<String>) -> ExtensionLoadError {
    ExtensionLoadError {
        path: path.as_ref().to_path_buf(),
        message: message.into(),
    }
}

fn leak_str(value: impl Into<String>) -> &'static str {
    Box::leak(value.into().into_boxed_str())
}

fn leak_keywords(items: Vec<String>) -> &'static [&'static str] {
    if items.is_empty() {
        return &[];
    }
    let leaked: Vec<&'static str> = items.into_iter().map(leak_str).collect();
    Box::leak(leaked.into_boxed_slice())
}

/// In-process adapter for a loaded manifest. One visible op: echo / uppercase.
pub struct ManifestTool {
    id: &'static str,
    display_name: &'static str,
    search_keywords: &'static [&'static str],
    group: GroupId,
    cli_name: Option<&'static str>,
}

impl Tool for ManifestTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: ToolId::new(self.id),
            display_name: self.display_name,
            search_keywords: self.search_keywords,
            group: self.group,
            searchable: true,
            favorable: true,
            accepted_types: &[],
        }
    }

    fn cli(&self) -> Option<CliTool> {
        self.cli_name
            .map(|name| cli::cli_tool(self.id, name, self.display_name))
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(Box::new(view::ManifestToolView::new()))
    }
}

/// Merge builtins with extensions loaded from `dir`.
pub fn catalog_with_extensions_dir(
    dir: impl AsRef<Path>,
) -> (ToolCatalog, Vec<ExtensionLoadError>) {
    let loaded = load_extensions(dir);
    (ToolCatalog::with_extensions(loaded.tools), loaded.errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_manifest(dir: &Path, name: &str, body: &str) -> PathBuf {
        let ext_dir = dir.join(name);
        fs::create_dir_all(&ext_dir).unwrap();
        let path = ext_dir.join(MANIFEST_TOML);
        fs::write(&path, body).unwrap();
        ext_dir
    }

    fn sample_toml(id: &str) -> String {
        format!(
            r#"
id = "{id}"
display_name = "Echo Upper"
group = "Text"
search_keywords = ["echo", "uppercase"]
cli = "{id}"
"#
        )
    }

    fn searchable(meta: &ToolMetadata, query: &str) -> bool {
        let needle = query.to_lowercase();
        meta.searchable
            && (meta.display_name.to_lowercase().contains(&needle)
                || meta.id.as_str().to_lowercase().contains(&needle)
                || meta
                    .search_keywords
                    .iter()
                    .any(|keyword| keyword.to_lowercase().contains(&needle)))
    }

    #[test]
    fn default_dir_ends_with_extensions() {
        let dir = default_extensions_dir();
        assert_eq!(dir.file_name().unwrap(), "extensions");
        assert_eq!(dir.parent().unwrap().file_name().unwrap(), "devtoys-rs");
    }

    #[test]
    fn missing_dir_is_empty_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        let loaded = load_extensions(&missing);
        assert!(loaded.tools.is_empty());
        assert!(loaded.errors.is_empty());
        assert_eq!(crate::default_catalog().len(), 23);
    }

    #[test]
    fn valid_extension_registers_and_is_searchable() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "echo_upper", &sample_toml("EchoUpper"));
        let loaded = load_extensions(dir.path());
        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(loaded.tools.len(), 1);
        let meta = loaded.tools[0].metadata();
        assert_eq!(meta.id.as_str(), "EchoUpper");
        assert_eq!(meta.display_name, "Echo Upper");
        assert_eq!(meta.group, GroupId::Text);
        assert!(meta.favorable);
        assert!(searchable(&meta, "echo"));
        assert!(searchable(&meta, "EchoUpper"));
        assert!(loaded.tools[0].cli().is_some());
        #[cfg(feature = "gui")]
        assert!(loaded.tools[0].create_view().is_some());

        let catalog = ToolCatalog::with_extensions(loaded.tools);
        assert_eq!(catalog.len(), 24);
        assert_eq!(crate::default_catalog().len(), 23);
    }

    #[test]
    fn removed_files_are_gone_on_next_load() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = write_manifest(dir.path(), "echo_upper", &sample_toml("EchoUpper"));
        assert_eq!(load_extensions(dir.path()).tools.len(), 1);
        fs::remove_dir_all(&ext_dir).unwrap();
        let loaded = load_extensions(dir.path());
        assert!(loaded.tools.is_empty());
        assert!(loaded.errors.is_empty());
        assert_eq!(crate::default_catalog().len(), 23);
    }

    #[test]
    fn corrupt_manifest_is_isolated() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "broken", "this is not toml {{{");
        let loaded = load_extensions(dir.path());
        assert!(loaded.tools.is_empty());
        assert_eq!(loaded.errors.len(), 1);
        assert!(loaded.errors[0].message.contains("TOML"));
        let catalog = ToolCatalog::with_extensions(loaded.tools);
        assert_eq!(catalog.len(), 23);
        assert_eq!(crate::default_catalog().len(), 23);
    }

    #[test]
    fn duplicate_builtin_id_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "dup", &sample_toml("JsonFormatter"));
        let loaded = load_extensions(dir.path());
        assert!(loaded.tools.is_empty());
        assert_eq!(loaded.errors.len(), 1);
        assert!(loaded.errors[0].message.contains("JsonFormatter"));
        assert_eq!(crate::default_catalog().len(), 23);
    }

    #[test]
    fn unknown_group_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(
            dir.path(),
            "bad_group",
            r#"
id = "Mystery"
display_name = "Mystery"
group = "Widgets"
"#,
        );
        let loaded = load_extensions(dir.path());
        assert!(loaded.tools.is_empty());
        assert!(loaded.errors[0].message.contains("未知分组"));
    }

    #[test]
    fn json_manifest_loads() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("json_echo");
        fs::create_dir_all(&ext_dir).unwrap();
        fs::write(
            ext_dir.join(MANIFEST_JSON),
            r#"{
                "id": "JsonEcho",
                "display_name": "JSON Echo",
                "group": "Text",
                "search_keywords": ["json-echo"]
            }"#,
        )
        .unwrap();
        let loaded = load_extensions(dir.path());
        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(loaded.tools[0].metadata().id.as_str(), "JsonEcho");
        assert!(loaded.tools[0].cli().is_none());
    }

    #[test]
    fn valid_and_corrupt_together_keep_builtins() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "echo_upper", &sample_toml("EchoUpper"));
        write_manifest(dir.path(), "broken", "???");
        let (catalog, errors) = catalog_with_extensions_dir(dir.path());
        assert_eq!(errors.len(), 1);
        assert_eq!(catalog.len(), 24);
        assert!(catalog
            .all_metadata()
            .iter()
            .any(|meta| meta.id.as_str() == "EchoUpper"));
        assert_eq!(crate::default_catalog().len(), 23);
    }

    #[test]
    fn second_extension_with_same_id_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "a", &sample_toml("EchoUpper"));
        write_manifest(dir.path(), "b", &sample_toml("EchoUpper"));
        let loaded = load_extensions(dir.path());
        assert_eq!(loaded.tools.len(), 1);
        assert_eq!(loaded.errors.len(), 1);
        assert!(loaded.errors[0].message.contains("EchoUpper"));
    }

    #[test]
    fn builtin_cli_name_collision_keeps_gui() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(
            dir.path(),
            "clash",
            r#"
id = "NotUuid"
display_name = "Not UUID"
group = "Generators"
cli = "uuid"
"#,
        );
        let loaded = load_extensions(dir.path());
        assert_eq!(loaded.tools.len(), 1);
        assert_eq!(loaded.tools[0].metadata().id.as_str(), "NotUuid");
        assert!(loaded.tools[0].cli().is_none());
        assert!(loaded.errors.iter().any(|err| err.message.contains("uuid")));
        assert_eq!(crate::default_catalog().all_cli().len(), 19);
    }
}
