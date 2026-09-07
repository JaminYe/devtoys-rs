use clap::Command;
use devtoys_api::{GroupId, JSON_FORMATTER_ID, SETTINGS_ID};
#[cfg(feature = "gui")]
use devtoys_tools::open_gui_tool;
use devtoys_tools::{all_cli_tools, all_tools, build_cli, default_catalog, ToolCatalog};
#[test]
fn thirty_business_tools_are_registered() {
    assert_eq!(all_tools().len(), 30);
}

#[test]
fn default_catalog_direct_access() {
    let catalog = default_catalog();
    assert_eq!(catalog.tools().len(), 30);
    assert_eq!(catalog.len(), 30);
    assert!(!catalog.is_empty());
    assert_eq!(catalog.all_metadata().len(), 30);
    assert_eq!(catalog.all_cli().len(), 26);
    assert!(!catalog.all_detectors().is_empty());

    assert_eq!(ToolCatalog::default_catalog().len(), 30);
}

#[cfg(feature = "gui")]
#[test]
fn default_catalog_and_open_gui_tool_open_view() {
    let catalog = default_catalog();
    assert!(catalog.open_view(JSON_FORMATTER_ID).is_some());
    assert!(open_gui_tool(JSON_FORMATTER_ID).is_some());
    assert!(catalog.open_view("NonExistentTool").is_none());
    assert!(open_gui_tool("NonExistentTool").is_none());
}

#[test]
fn json_formatter_is_registered_in_formatters() {
    let tools = all_tools();
    let json = tools
        .iter()
        .find(|tool| tool.id.as_str() == JSON_FORMATTER_ID)
        .expect("JsonFormatter must stay registered");
    assert_eq!(json.display_name, "JSON");
    assert_eq!(json.group, GroupId::Formatters);
    assert!(json.searchable);
    assert!(json.favorable);
    assert_eq!(json.accepted_types, &["json"]);
}

#[test]
fn settings_is_not_in_the_business_catalog() {
    assert!(all_tools()
        .iter()
        .all(|tool| tool.id.as_str() != SETTINGS_ID));
}

#[test]
fn cli_commands_share_gui_ids() {
    let tools = all_tools();
    let gui_ids: Vec<&str> = tools.iter().map(|tool| tool.id.as_str()).collect();
    let cli = all_cli_tools();
    assert!(cli.iter().any(|cmd| {
        cmd.tool_id == JSON_FORMATTER_ID
            && cmd.name == "JsonFormatter"
            && cmd.aliases.contains(&"Jsonf")
    }));
    for cmd in &cli {
        assert!(
            gui_ids.contains(&cmd.tool_id),
            "CLI {} must share a GUI id",
            cmd.name
        );
    }
}

#[test]
fn jsonf_alias_dispatches_to_json_formatter() {
    let cmd: Command = build_cli();
    let matches = cmd
        .try_get_matches_from(["devtoys-cli", "Jsonf", "-i", "{}"])
        .expect("Jsonf alias should parse");
    assert_eq!(matches.subcommand_name(), Some("JsonFormatter"));
}
