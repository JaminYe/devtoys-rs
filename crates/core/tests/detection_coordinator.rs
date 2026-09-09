//! Real runtime smoke tests. Deterministic lifecycle tests live beside the
//! coordinator and exercise its public interface with controlled dependencies.
use std::sync::Arc;
use std::time::{Duration, Instant};

use devtoys_api::{GroupId, RawData, ToolId, ToolMetadata, JSON_FORMATTER_ID, TYPE_JSON};
use devtoys_core::{all_detectors, DetectionCoordinator, DetectionEngine, InMemoryClipboard};

fn test_engine() -> Arc<DetectionEngine> {
    Arc::new(DetectionEngine::new(
        all_detectors(),
        &[ToolMetadata {
            id: ToolId::new(JSON_FORMATTER_ID),
            display_name: "JSON",
            search_keywords: &[],
            group: GroupId::Formatters,
            searchable: true,
            favorable: true,
            accepted_types: &[TYPE_JSON],
        }],
    ))
}

#[test]
fn idle_monitoring_schedules_another_poll() {
    let mut coordinator =
        DetectionCoordinator::new(test_engine(), Box::new(InMemoryClipboard::new()));
    coordinator.poll(None, true);
    let delay = coordinator
        .next_poll_after()
        .expect("idle clipboard must keep polling");
    assert!(delay > Duration::ZERO && delay <= Duration::from_millis(800));
    coordinator.poll(None, false);
    assert_eq!(coordinator.next_poll_after(), None);
}

#[test]
fn real_worker_delivers_json_and_returns_to_idle() {
    let clipboard = InMemoryClipboard::new();
    let mut coordinator = DetectionCoordinator::new(test_engine(), Box::new(clipboard.clone()));
    clipboard.set_text(r#"{"key":1}"#);
    coordinator.poll(None, true);
    let deadline = Instant::now() + Duration::from_secs(5);
    while coordinator.is_detecting() {
        assert!(Instant::now() < deadline, "real worker did not complete");
        std::thread::yield_now();
        coordinator.poll(None, true);
    }
    assert_eq!(coordinator.recommendations().len(), 1);
    assert_eq!(coordinator.recommendations()[0].tool_id, JSON_FORMATTER_ID);
    assert_eq!(coordinator.recommendations()[0].payload, r#"{"key":1}"#);
    assert_eq!(
        coordinator.last_clipboard(),
        Some(&RawData::text(r#"{"key":1}"#))
    );
    assert!(coordinator.next_poll_after().is_some());
    coordinator.clear();
    assert!(coordinator.recommendations().is_empty());
    assert_eq!(coordinator.next_poll_after(), None);
}
