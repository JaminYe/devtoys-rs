use std::sync::atomic::AtomicBool;

use devtoys_api::{
    DataTypeSpec, DetectedPayload, Detector, GroupId, RawData, ToolId, ToolMetadata, TYPE_DATE,
    TYPE_TEXT,
};
use devtoys_core::{
    all_detectors, validate, DateDetector, DetectOptions, DetectionAssemblyIssue, DetectionEngine,
    Recommendation, TextDetector,
};

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

/// A detector that re-registers the core `date` type, producing a duplicate.
struct DuplicateDateDetector;

impl Detector for DuplicateDateDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_DATE,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, _parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        None
    }
}

/// A detector whose parent is not present in the detector set.
struct OrphanDetector;

impl Detector for OrphanDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: "orphan",
            parent: Some("ghost"),
        }
    }

    fn detect(&self, _raw: &RawData, _parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        None
    }
}

#[test]
fn validate_reports_duplicate_type_names() {
    let detectors: Vec<Box<dyn Detector>> =
        vec![Box::new(DateDetector), Box::new(DuplicateDateDetector)];
    let issues = validate(&detectors, &[]);
    assert!(
        issues.contains(&DetectionAssemblyIssue::DuplicateType {
            type_name: TYPE_DATE.to_string()
        }),
        "expected DuplicateType for {TYPE_DATE}, got {issues:?}"
    );
}

#[test]
fn validate_reports_missing_parent() {
    let detectors: Vec<Box<dyn Detector>> = vec![Box::new(OrphanDetector)];
    let issues = validate(&detectors, &[]);
    assert!(
        issues.contains(&DetectionAssemblyIssue::MissingParent {
            type_name: "orphan".to_string(),
            parent_name: "ghost".to_string()
        }),
        "expected MissingParent for orphan, got {issues:?}"
    );
}

#[test]
fn validate_reports_unresolved_tool_type() {
    let tools = vec![meta("OddTool", &["noSuchType"])];
    let detectors: Vec<Box<dyn Detector>> = vec![Box::new(TextDetector)];
    let issues = validate(&detectors, &tools);
    assert!(
        issues.contains(&DetectionAssemblyIssue::UnresolvedToolType {
            tool_id: "OddTool".to_string(),
            type_name: "noSuchType".to_string()
        }),
        "expected UnresolvedToolType, got {issues:?}"
    );
}

#[test]
fn validate_clean_set_produces_no_issues() {
    let issues = validate(&all_detectors(), &[]);
    assert!(
        issues.is_empty(),
        "core detectors alone must assemble cleanly: {issues:?}"
    );
}

#[test]
fn core_all_detectors_yield_exactly_one_date_recommendation() {
    let tools = vec![
        meta("DateConverter", &[TYPE_DATE]),
        meta("TextTool", &[TYPE_TEXT]),
    ];
    let engine = DetectionEngine::new(all_detectors(), &tools);
    let cancel = AtomicBool::new(false);
    let recs: Vec<Recommendation> = engine.detect(
        &RawData::text("1710000000"),
        DetectOptions {
            strict: true,
            active_tool: None,
            enabled: true,
            cancel: &cancel,
        },
    );
    let date_recs: Vec<_> = recs
        .iter()
        .filter(|r| r.tool_id == "DateConverter")
        .collect();
    assert_eq!(
        date_recs.len(),
        1,
        "core assembly must produce exactly one DateConverter recommendation, got {recs:?}"
    );
    assert_eq!(date_recs[0].data_type, TYPE_DATE);
}
