//! Real core + catalog assembly: verifies the `date` type has a single
//! registration owner and yields exactly one DateConverter recommendation.
//!
//! This lives in the host because only the host can combine `devtoys-core`
//! detectors with `devtoys-tools` catalog detectors (core must not depend on
//! tools).

use std::sync::atomic::AtomicBool;

use devtoys_api::{Detector, RawData, TYPE_DATE};
use devtoys_core::{validate, DetectOptions, DetectionAssemblyIssue, DetectionEngine};
use devtoys_tools::default_catalog;

fn combined_detectors() -> Vec<Box<dyn Detector>> {
    let mut detectors = devtoys_core::all_detectors();
    detectors.extend(default_catalog().all_detectors());
    detectors
}

fn real_tools() -> Vec<devtoys_api::ToolMetadata> {
    default_catalog().all_metadata()
}

fn date_detector_count(detectors: &[Box<dyn Detector>]) -> usize {
    detectors
        .iter()
        .filter(|d| d.data_type().name == TYPE_DATE)
        .count()
}

#[test]
fn real_assembly_registers_date_exactly_once() {
    let detectors = combined_detectors();
    assert_eq!(
        date_detector_count(&detectors),
        1,
        "core + catalog must register `date` exactly once"
    );
}

#[test]
fn real_assembly_has_no_structural_type_graph_issues() {
    let detectors = combined_detectors();
    let tools = real_tools();
    let issues = validate(&detectors, &tools);
    for issue in &issues {
        // UnresolvedToolType is an audit finding; structural issues are fatal.
        assert!(
            matches!(issue, DetectionAssemblyIssue::UnresolvedToolType { .. }),
            "unexpected structural assembly issue: {issue:?}"
        );
    }
}

#[test]
fn real_assembly_date_string_yields_exactly_one_date_converter() {
    let detectors = combined_detectors();
    let tools = real_tools();
    let engine = DetectionEngine::new(detectors, &tools);
    let cancel = AtomicBool::new(false);
    let recs = engine.detect(
        &RawData::text("1710000000"),
        DetectOptions {
            strict: true,
            active_tool: None,
            enabled: true,
            cancel: &cancel,
        },
    );
    let date = recs
        .iter()
        .filter(|r| r.tool_id == "DateConverter")
        .collect::<Vec<_>>();
    assert_eq!(
        date.len(),
        1,
        "real core+catalog assembly must produce one DateConverter recommendation, got {recs:?}"
    );
    assert_eq!(date[0].data_type, TYPE_DATE);
}

// Ensures the core text parent still resolves in the real assembly (no dup/missing).
#[test]
fn real_assembly_text_parent_is_resolvable() {
    let detectors = combined_detectors();
    // `text` must appear exactly once and no detector claims a missing parent
    // that would drop it. Just assert the type exists as a root via validate.
    let tools = real_tools();
    let issues = validate(&detectors, &tools);
    assert!(
        issues
            .iter()
            .all(|i| matches!(i, DetectionAssemblyIssue::UnresolvedToolType { .. })),
        "structural issues must be empty in real assembly: {issues:?}"
    );
}
