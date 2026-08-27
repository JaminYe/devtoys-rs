use devtoys_api::{GroupId, ToolId, ToolMetadata, SETTINGS_ID};
use devtoys_core::{SearchOutcome, ToolRegistry};

const JSON_KEYWORDS: &[&str] = &["json", "pretty"];
const JSON_TYPES: &[&str] = &["json"];
const TEXT_KEYWORDS: &[&str] = &["plain"];
const SETTINGS_KEYWORDS: &[&str] = &["settings", "偏好"];

fn meta(
    id: &'static str,
    display_name: &'static str,
    keywords: &'static [&'static str],
    group: GroupId,
    searchable: bool,
    favorable: bool,
    accepted: &'static [&'static str],
) -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(id),
        display_name,
        search_keywords: keywords,
        group,
        searchable,
        favorable,
        accepted_types: accepted,
    }
}

fn sample_registry() -> ToolRegistry {
    ToolRegistry::new(vec![
        meta(
            "JsonFormatter",
            "JSON 格式化",
            JSON_KEYWORDS,
            GroupId::Formatters,
            true,
            true,
            JSON_TYPES,
        ),
        meta(
            "TextTool",
            "文本工具",
            TEXT_KEYWORDS,
            GroupId::Text,
            true,
            true,
            &[],
        ),
        meta(
            SETTINGS_ID,
            "设置",
            SETTINGS_KEYWORDS,
            GroupId::Text,
            false,
            false,
            &[],
        ),
    ])
}

fn hit_ids(outcome: SearchOutcome<'_>) -> Vec<&str> {
    match outcome {
        SearchOutcome::Hits(hits) => hits.iter().map(|t| t.id.as_str()).collect(),
        other => panic!("expected Hits, got {other:?}"),
    }
}

#[test]
fn empty_or_whitespace_query_is_idle() {
    let registry = sample_registry();
    assert_eq!(registry.search(""), SearchOutcome::Idle);
    assert_eq!(registry.search("   "), SearchOutcome::Idle);
    assert_eq!(registry.search("\t\n"), SearchOutcome::Idle);
}

#[test]
fn unmatched_query_is_empty() {
    let registry = sample_registry();
    assert_eq!(registry.search("xyzzy"), SearchOutcome::Empty);
}

#[test]
fn matches_display_name_id_and_keywords_case_insensitive() {
    let registry = sample_registry();
    assert_eq!(hit_ids(registry.search("json 格式")), vec!["JsonFormatter"]);
    assert_eq!(
        hit_ids(registry.search("JSONFORMATTER")),
        vec!["JsonFormatter"]
    );
    assert_eq!(hit_ids(registry.search("Pretty")), vec!["JsonFormatter"]);
    assert_eq!(hit_ids(registry.search("文本")), vec!["TextTool"]);
    assert_eq!(hit_ids(registry.search("json")), vec!["JsonFormatter"]);
}

#[test]
fn not_searchable_tools_are_omitted() {
    let registry = sample_registry();
    assert_eq!(registry.search("设置"), SearchOutcome::Empty);
    assert_eq!(registry.search("settings"), SearchOutcome::Empty);
    assert_eq!(registry.search("偏好"), SearchOutcome::Empty);
    assert_eq!(hit_ids(registry.search("json")), vec!["JsonFormatter"]);
}

#[test]
fn get_all_and_in_group_by_registration_order() {
    let registry = sample_registry();
    assert_eq!(registry.all().len(), 3);
    assert_eq!(
        registry.get("JsonFormatter").map(|t| t.display_name),
        Some("JSON 格式化")
    );
    assert!(registry.get("missing").is_none());

    let formatters = registry.in_group(GroupId::Formatters);
    assert_eq!(
        formatters.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        vec!["JsonFormatter"]
    );
    let text = registry.in_group(GroupId::Text);
    assert_eq!(
        text.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        vec!["TextTool", SETTINGS_ID]
    );
    assert!(registry.in_group(GroupId::Converters).is_empty());
}
