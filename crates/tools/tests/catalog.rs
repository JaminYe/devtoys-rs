use devtoys_api::{GroupId, JSON_FORMATTER_ID, SETTINGS_ID};
#[cfg(feature = "gui")]
use devtoys_tools::open_gui_tool;
use devtoys_tools::{all_tools, default_catalog, ToolCatalog};
#[test]
fn twenty_three_business_tools_are_registered() {
    assert_eq!(all_tools().len(), 23);
}

#[test]
fn default_catalog_direct_access() {
    let catalog = default_catalog();
    assert_eq!(catalog.tools().len(), 23);
    assert_eq!(catalog.len(), 23);
    assert!(!catalog.is_empty());
    assert_eq!(catalog.all_metadata().len(), 23);
    assert!(!catalog.all_detectors().is_empty());

    assert_eq!(ToolCatalog::default_catalog().len(), 23);
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
fn base64_image_registers_file_detector() {
    let catalog = default_catalog();
    let tool = catalog
        .all_metadata()
        .into_iter()
        .find(|tool| tool.id.as_str() == "Base64ImageEncoderDecoder")
        .expect("Base64 image tool");
    assert!(tool.accepted_types.contains(&"Base64ImageFile"));
    assert!(catalog
        .all_detectors()
        .iter()
        .any(|detector| detector.data_type().name == "Base64ImageFile"
            && detector.data_type().parent == Some("file")));
}
