use std::sync::Arc;
use std::time::Duration;

use devtoys_api::{GroupId, RawData, ToolId, ToolMetadata, JSON_FORMATTER_ID, TYPE_JSON, TYPE_TEXT};
use devtoys_core::{all_detectors, DetectionCoordinator, DetectionEngine, InMemoryClipboard};

fn meta(id: &'static str, accepted: &'static [&'static str]) -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(id),
        display_name: id,
        search_keywords: &[],
        group: GroupId::Formatters,
        searchable: true,
        favorable: true,
        accepted_types: accepted,
    }
}

fn test_engine() -> Arc<DetectionEngine> {
    let tools = vec![
        meta(JSON_FORMATTER_ID, &[TYPE_JSON]),
        meta("TextTool", &[TYPE_TEXT]),
    ];
    Arc::new(DetectionEngine::new(all_detectors(), &tools))
}

fn wait_for_detection(coordinator: &mut DetectionCoordinator, active_tool: Option<&str>) {
    for _ in 0..100 {
        if let Some(recs) = coordinator.poll(active_tool, true) {
            if !recs.is_empty() {
                return;
            }
        }
        if !coordinator.is_detecting() && !coordinator.recommendations().is_empty() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn test_detect_json_flow() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"key\": \"value\"}");
    coordinator.poll(None, true);

    wait_for_detection(&mut coordinator, None);

    let recs = coordinator.recommendations();
    assert!(!recs.is_empty(), "expected recommendations for JSON");
    assert!(
        recs.iter().any(|r| r.tool_id == JSON_FORMATTER_ID),
        "expected JsonFormatter recommendation"
    );

    assert_eq!(
        coordinator.last_clipboard(),
        Some(&RawData::text("{\"key\": \"value\"}"))
    );
}

#[test]
fn test_active_tool_filtering() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"key\": \"value\"}");
    // Poll with active_tool set to JsonFormatter
    coordinator.poll(Some(JSON_FORMATTER_ID), true);

    for _ in 0..100 {
        coordinator.poll(Some(JSON_FORMATTER_ID), true);
        if !coordinator.is_detecting() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let recs = coordinator.recommendations();
    assert!(
        !recs.iter().any(|r| r.tool_id == JSON_FORMATTER_ID),
        "JsonFormatter should be filtered out when active"
    );
}

#[test]
fn test_active_tool_dynamic_filtering() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"key\": \"value\"}");
    coordinator.poll(None, true);

    wait_for_detection(&mut coordinator, None);
    assert!(coordinator
        .recommendations()
        .iter()
        .any(|r| r.tool_id == JSON_FORMATTER_ID));

    // Navigating to the tool should filter it out immediately on poll
    let result = coordinator.poll(Some(JSON_FORMATTER_ID), true);
    assert!(
        result.is_none() || !result.unwrap().iter().any(|r| r.tool_id == JSON_FORMATTER_ID),
        "Dynamic switch to active tool must filter the recommendation"
    );
    assert!(!coordinator
        .recommendations()
        .iter()
        .any(|r| r.tool_id == JSON_FORMATTER_ID));
}

#[test]
fn test_active_tool_enter_leave_restores() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"key\": \"value\"}");
    coordinator.poll(None, true);
    wait_for_detection(&mut coordinator, None);

    assert!(coordinator
        .recommendations()
        .iter()
        .any(|r| r.tool_id == JSON_FORMATTER_ID));

    // Entering the tool excludes it from the projection (no permanent removal).
    let res = coordinator.poll(Some(JSON_FORMATTER_ID), true);
    assert!(
        res.is_none() || !res.unwrap().iter().any(|r| r.tool_id == JSON_FORMATTER_ID),
        "active tool must be excluded from projection"
    );

    // Leaving the tool with an unchanged clipboard restores the recommendation
    // purely from the cached raw hits — no re-detection required.
    let res = coordinator.poll(None, true);
    assert!(
        res.is_some_and(|recs| recs.iter().any(|r| r.tool_id == JSON_FORMATTER_ID)),
        "leaving the tool must restore the recommendation from cached raw hits"
    );
    assert!(coordinator
        .recommendations()
        .iter()
        .any(|r| r.tool_id == JSON_FORMATTER_ID));
}

#[test]
fn test_debounce_same_text() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"a\": 1}");
    coordinator.poll(None, true);
    assert_eq!(coordinator.detect_gen(), 1);

    wait_for_detection(&mut coordinator, None);

    // Wait past debounce threshold (> 800ms) with same content
    std::thread::sleep(Duration::from_millis(850));
    coordinator.poll(None, true);

    // Same text should NOT trigger a new generation
    assert_eq!(
        coordinator.detect_gen(),
        1,
        "Same text after debounce must not increment generation"
    );

    // Now change content and wait past debounce
    clipboard.set_text("[1, 2, 3]");
    std::thread::sleep(Duration::from_millis(850));
    coordinator.poll(None, true);

    assert_eq!(
        coordinator.detect_gen(),
        2,
        "New text after debounce must trigger new generation"
    );
}

#[test]
fn test_disabled_clears_and_cancels() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"a\": 1}");
    coordinator.poll(None, true);

    wait_for_detection(&mut coordinator, None);
    assert!(!coordinator.recommendations().is_empty());

    // Disabling detection clears recommendations immediately
    let res = coordinator.poll(None, false);
    assert!(res.is_none());
    assert!(coordinator.recommendations().is_empty());
    assert!(!coordinator.is_detecting());
}

#[test]
fn test_disable_reenable_redetects() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"a\": 1}");
    coordinator.poll(None, true);
    wait_for_detection(&mut coordinator, None);
    assert!(!coordinator.recommendations().is_empty());

    // Disable: cancels and clears the raw hit cache + projection.
    let res = coordinator.poll(None, false);
    assert!(res.is_none());
    assert!(coordinator.recommendations().is_empty());
    assert!(!coordinator.is_detecting());

    // Re-enable: clear() resets last_clipboard so a fresh detection starts once
    // the clipboard changes past the debounce window.
    coordinator.clear();
    clipboard.set_text("{\"new\": [1, 2, 3]}");
    std::thread::sleep(Duration::from_millis(850));
    coordinator.poll(None, true);
    wait_for_detection(&mut coordinator, None);
    assert!(!coordinator.recommendations().is_empty());
}

#[test]
fn test_clear_method() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"a\": 1}");
    coordinator.poll(None, true);
    wait_for_detection(&mut coordinator, None);

    assert!(coordinator.last_clipboard().is_some());
    assert!(!coordinator.recommendations().is_empty());

    coordinator.clear();
    assert!(coordinator.last_clipboard().is_none());
    assert!(coordinator.recommendations().is_empty());
    assert!(!coordinator.is_detecting());
}

#[test]
fn test_clipboard_emptied_clears_recommendations() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    clipboard.set_text("{\"a\": 1}");
    coordinator.poll(None, true);
    wait_for_detection(&mut coordinator, None);
    assert!(!coordinator.recommendations().is_empty());

    // Emptied clipboard after debounce period
    clipboard.clear();
    std::thread::sleep(Duration::from_millis(850));
    coordinator.poll(None, true);

    assert!(coordinator.last_clipboard().is_none());
    assert!(coordinator.recommendations().is_empty());
}

#[test]
fn test_generational_counter_stale_discard() {
    let engine = test_engine();
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(engine, Box::new(clipboard.clone()));

    // Start detection for generation 1 with JSON
    coordinator.start_detect(RawData::text("{\"old\": 1}"));
    assert_eq!(coordinator.detect_gen(), 1);

    // Immediately supersede with generation 2 plain text
    coordinator.start_detect(RawData::text("just plain text"));
    assert_eq!(coordinator.detect_gen(), 2);

    wait_for_detection(&mut coordinator, None);

    // Stale generation 1 result must not be applied
    let recs = coordinator.recommendations();
    assert!(
        recs.iter().all(|r| r.tool_id != JSON_FORMATTER_ID),
        "stale JSON recommendation from gen 1 should be discarded"
    );
}
