use std::sync::atomic::AtomicBool;

use devtoys_api::{
    DetectedPayload, Detector, GroupId, RawData, ToolId, ToolMetadata, JSON_FORMATTER_ID,
    TYPE_DATE, TYPE_FILE, TYPE_FILES, TYPE_IMAGE, TYPE_IMAGE_FILE, TYPE_JSON, TYPE_JSON_ARRAY,
    TYPE_TEXT, TYPE_XML, TYPE_XSD,
};
use devtoys_core::{
    all_detectors, Base64ImageDetector, Base64TextDetector, DateDetector, DetectOptions,
    DetectionEngine, GzipDetector, JsonArrayDetector, JsonDetector, Recommendation, TextDetector,
    XmlDetector, XsdDetector,
};

const JSON_TYPES: &[&str] = &[TYPE_JSON];
const TEXT_TYPES: &[&str] = &[TYPE_TEXT];

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

fn json_tool() -> ToolMetadata {
    meta(JSON_FORMATTER_ID, JSON_TYPES)
}

fn text_tool() -> ToolMetadata {
    meta("TextTool", TEXT_TYPES)
}

fn engine(tools: &[ToolMetadata]) -> DetectionEngine {
    DetectionEngine::new(all_detectors(), tools)
}

fn detect(
    engine: &DetectionEngine,
    raw: &str,
    strict: bool,
    active_tool: Option<&str>,
    enabled: bool,
    cancelled: bool,
) -> Vec<Recommendation> {
    let cancel = AtomicBool::new(cancelled);
    engine.detect(
        &RawData::text(raw),
        DetectOptions {
            strict,
            active_tool,
            enabled,
            cancel: &cancel,
        },
    )
}

fn text_parent(s: &str) -> DetectedPayload {
    DetectedPayload {
        type_name: TYPE_TEXT.to_string(),
        value: s.to_string(),
    }
}

fn json_parent(s: &str) -> DetectedPayload {
    DetectedPayload {
        type_name: TYPE_JSON.to_string(),
        value: s.to_string(),
    }
}

fn xml_parent(s: &str) -> DetectedPayload {
    DetectedPayload {
        type_name: TYPE_XML.to_string(),
        value: s.to_string(),
    }
}

fn json_detected(input: &str) -> bool {
    JsonDetector
        .detect(&RawData::text(input), Some(&text_parent(input)))
        .is_some()
}

fn json_array_detected(input: &str) -> bool {
    JsonArrayDetector
        .detect(&RawData::text(input), Some(&json_parent(input)))
        .is_some()
}

fn detect_raw(engine: &DetectionEngine, raw: &RawData, strict: bool) -> Vec<Recommendation> {
    let cancel = AtomicBool::new(false);
    engine.detect(
        raw,
        DetectOptions {
            strict,
            active_tool: None,
            enabled: true,
            cancel: &cancel,
        },
    )
}

#[test]
fn text_detector_rejects_empty_accepts_whitespace() {
    assert!(TextDetector.detect(&RawData::text(""), None).is_none());
    let space = TextDetector.detect(&RawData::text(" "), None).unwrap();
    assert_eq!(space.type_name, TYPE_TEXT);
    assert_eq!(space.value, " ");
    let hello = TextDetector
        .detect(&RawData::text("hello world"), None)
        .unwrap();
    assert_eq!(hello.value, "hello world");
}

#[test]
fn json_detector_requires_parent_and_matches_upstream_cases() {
    assert!(JsonDetector.detect(&RawData::text("{}"), None).is_none());

    let cases = [
        ("\"foo\"", true),
        ("123", false),
        ("   {  }  ", true),
        ("   [  ]  ", true),
        ("   { \"foo\": 123 }  ", true),
        ("   bar { \"foo\": 123 }  ", false),
        ("", false),
        (" ", false),
        ("{ \"title\": \"example glossary\" }", true),
        ("[ \"title\", \"example glossary\" ]", true),
        (
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
            false,
        ),
        (
            "foo :\n  bar :\n    - boo: 1\n    - rab: 2\n    - plop: 3",
            false,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(
            json_detected(input),
            expected,
            "json detector mismatch for {input:?}"
        );
    }
}

#[test]
fn engine_strict_json_recommends_only_json_tool() {
    let tools = vec![json_tool(), text_tool()];
    let recs = detect(
        &engine(&tools),
        r#"{ "json": 123 }"#,
        true,
        None,
        true,
        false,
    );
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].tool_id, JSON_FORMATTER_ID);
    assert_eq!(recs[0].data_type, TYPE_JSON);
    assert_eq!(recs[0].payload, r#"{ "json": 123 }"#);
}

#[test]
fn engine_nonstrict_json_includes_parent_text_tool() {
    let tools = vec![json_tool(), text_tool()];
    let recs = detect(
        &engine(&tools),
        r#"{ "json": 123 }"#,
        false,
        None,
        true,
        false,
    );
    assert_eq!(
        recs.iter()
            .map(|r| (r.tool_id.as_str(), r.data_type.as_str()))
            .collect::<Vec<_>>(),
        vec![(JSON_FORMATTER_ID, TYPE_JSON), ("TextTool", TYPE_TEXT)]
    );
    assert_eq!(recs[1].payload, r#"{ "json": 123 }"#);
}

#[test]
fn engine_nonstrict_includes_active_tool() {
    let tools = vec![json_tool(), text_tool()];
    // The engine no longer filters active_tool — even the active tool is
    // recommended. active_tool exclusion is owned by the coordinator.
    let recs = detect(
        &engine(&tools),
        r#"{ "json": 123 }"#,
        false,
        Some(JSON_FORMATTER_ID),
        true,
        false,
    );
    assert_eq!(
        recs.iter()
            .map(|r| (r.tool_id.as_str(), r.data_type.as_str()))
            .collect::<Vec<_>>(),
        vec![(JSON_FORMATTER_ID, TYPE_JSON), ("TextTool", TYPE_TEXT)]
    );

    let recs = detect(
        &engine(&tools),
        r#"{ "json": 123 }"#,
        false,
        Some("TextTool"),
        true,
        false,
    );
    assert_eq!(
        recs.iter()
            .map(|r| (r.tool_id.as_str(), r.data_type.as_str()))
            .collect::<Vec<_>>(),
        vec![(JSON_FORMATTER_ID, TYPE_JSON), ("TextTool", TYPE_TEXT)]
    );

    // Strict mode still returns only the exact JSON type, regardless of
    // active_tool.
    let recs = detect(
        &engine(&tools),
        r#"{ "json": 123 }"#,
        true,
        Some(JSON_FORMATTER_ID),
        true,
        false,
    );
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].tool_id, JSON_FORMATTER_ID);
}

#[test]
fn engine_disabled_returns_empty() {
    let tools = vec![json_tool(), text_tool()];
    let recs = detect(
        &engine(&tools),
        r#"{ "json": 123 }"#,
        false,
        None,
        false,
        false,
    );
    assert!(recs.is_empty());
}

#[test]
fn engine_text_without_accepting_tool_is_empty() {
    let tools = vec![json_tool()];
    let recs = detect(&engine(&tools), "hello world", false, None, true, false);
    assert!(recs.is_empty());
}

#[test]
fn engine_cancelled_returns_empty() {
    let tools = vec![json_tool(), text_tool()];
    let recs = detect(
        &engine(&tools),
        r#"{ "json": 123 }"#,
        false,
        None,
        true,
        true,
    );
    assert!(recs.is_empty());
}

#[test]
fn engine_integer_is_text_not_json() {
    let tools = vec![json_tool(), text_tool()];
    let recs = detect(&engine(&tools), "123", true, None, true, false);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].tool_id, "TextTool");
    assert_eq!(recs[0].data_type, TYPE_TEXT);
}

#[test]
fn json_array_detector_requires_non_empty_array_of_objects() {
    assert!(JsonArrayDetector
        .detect(&RawData::text(r#"[{ "foo": 1 }]"#), None)
        .is_none());
    assert!(json_array_detected(r#"[{ "foo": 123 }]"#));
    assert!(json_array_detected(
        r#"   [{ "foo": 123 }, { "bar": 456 }]  "#
    ));
    assert!(!json_array_detected("[]"));
    assert!(!json_array_detected("   [  ]  "));
    assert!(!json_array_detected(r#"["title", "example glossary"]"#));
    assert!(!json_array_detected(r#"{ "foo": 123 }"#));
    assert!(!json_array_detected("123"));
}

#[test]
fn engine_json_array_of_objects_recommends_json_array_tool() {
    let tools = vec![
        meta("JsonTable", &[TYPE_JSON_ARRAY]),
        json_tool(),
        text_tool(),
    ];
    let recs = detect(
        &engine(&tools),
        r#"[{ "foo": 123 }, { "bar": 456 }]"#,
        true,
        None,
        true,
        false,
    );
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].tool_id, "JsonTable");
    assert_eq!(recs[0].data_type, TYPE_JSON_ARRAY);
}

#[test]
fn engine_empty_json_array_is_json_not_json_array() {
    let tools = vec![
        meta("JsonTable", &[TYPE_JSON_ARRAY]),
        json_tool(),
        text_tool(),
    ];
    let recs = detect(&engine(&tools), "[]", true, None, true, false);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].tool_id, JSON_FORMATTER_ID);
    assert_eq!(recs[0].data_type, TYPE_JSON);
}

#[test]
fn xml_detector_matches_tag_structure_not_json() {
    assert!(XmlDetector
        .detect(&RawData::text("<a></a>"), Some(&text_parent("<a></a>")))
        .is_some());
    assert!(XmlDetector
        .detect(&RawData::text("<xml />"), Some(&text_parent("<xml />")))
        .is_some());
    assert!(XmlDetector
        .detect(&RawData::text("<a></a>"), None)
        .is_none());
    assert!(XmlDetector
        .detect(&RawData::text("<>"), Some(&text_parent("<>")))
        .is_none());
    assert!(XmlDetector
        .detect(
            &RawData::text(r#"{ "json": 123 }"#),
            Some(&text_parent(r#"{ "json": 123 }"#))
        )
        .is_none());
}

#[test]
fn engine_xml_is_xml_not_json() {
    let tools = vec![meta("XmlTool", &[TYPE_XML]), json_tool(), text_tool()];
    let recs = detect(&engine(&tools), "<a></a>", true, None, true, false);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].tool_id, "XmlTool");
    assert_eq!(recs[0].data_type, TYPE_XML);
}

#[test]
fn xsd_detector_requires_schema_marker_and_xml_parent() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="a" type="xs:string"/></xs:schema>"#;
    assert!(XsdDetector
        .detect(&RawData::text(xsd), Some(&xml_parent(xsd)))
        .is_some());
    assert!(XsdDetector.detect(&RawData::text(xsd), None).is_none());
    assert!(XsdDetector
        .detect(&RawData::text("<a></a>"), Some(&xml_parent("<a></a>")))
        .is_none());
}

#[test]
fn engine_xsd_is_xsd_child_of_xml() {
    let input = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="a" type="xs:string"/></xs:schema>"#;
    let tools = vec![
        meta("XsdTool", &[TYPE_XSD]),
        meta("XmlTool", &[TYPE_XML]),
        text_tool(),
    ];
    let recs = detect(&engine(&tools), input, true, None, true, false);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].data_type, TYPE_XSD);
    assert_eq!(recs[0].tool_id, "XsdTool");

    let recs = detect(&engine(&tools), input, false, None, true, false);
    assert_eq!(
        recs.iter()
            .map(|r| r.data_type.as_str())
            .collect::<Vec<_>>(),
        vec![TYPE_XSD, TYPE_XML]
    );
}

#[test]
fn date_detector_unix_and_iso_rejects_words() {
    assert!(DateDetector
        .detect(
            &RawData::text("1710000000"),
            Some(&text_parent("1710000000"))
        )
        .is_some());
    assert!(DateDetector
        .detect(
            &RawData::text("1710000000000"),
            Some(&text_parent("1710000000000"))
        )
        .is_some());
    assert!(DateDetector
        .detect(
            &RawData::text("2024-03-09T12:00:00Z"),
            Some(&text_parent("2024-03-09T12:00:00Z"))
        )
        .is_some());
    assert!(DateDetector
        .detect(&RawData::text("hello"), Some(&text_parent("hello")))
        .is_none());
    assert!(DateDetector
        .detect(&RawData::text("123"), Some(&text_parent("123")))
        .is_none());
}

#[test]
fn engine_unix_seconds_detected_as_date() {
    let tools = vec![meta("DateTool", &[TYPE_DATE]), text_tool()];
    let recs = detect(&engine(&tools), "1710000000", true, None, true, false);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].tool_id, "DateTool");
    assert_eq!(recs[0].data_type, TYPE_DATE);
}

#[test]
fn engine_files_and_image_via_raw_data() {
    let tools = vec![
        meta("FilesTool", &[TYPE_FILES]),
        meta("FileTool", &[TYPE_FILE]),
        meta("ImageFileTool", &[TYPE_IMAGE_FILE]),
        meta("ImageTool", &[TYPE_IMAGE]),
    ];
    let eng = engine(&tools);

    let recs = detect_raw(
        &eng,
        &RawData::Files(vec!["a.txt".into(), "b.txt".into()]),
        true,
    );
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].data_type, TYPE_FILES);

    let recs = detect_raw(&eng, &RawData::Files(vec!["notes.txt".into()]), true);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].data_type, TYPE_FILE);

    let recs = detect_raw(&eng, &RawData::Files(vec!["pic.PNG".into()]), true);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].data_type, TYPE_IMAGE_FILE);

    let recs = detect_raw(
        &eng,
        &RawData::Image {
            bytes: vec![0x89, 0x50, 0x4E, 0x47],
            mime: Some("image/png".into()),
        },
        true,
    );
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].data_type, TYPE_IMAGE);

    let recs = detect_raw(
        &eng,
        &RawData::Image {
            bytes: vec![],
            mime: None,
        },
        true,
    );
    assert!(recs.is_empty());

    let recs = detect_raw(&eng, &RawData::Files(vec![]), true);
    assert!(recs.is_empty());
}

#[test]
fn base64_text_gzip_and_image_detectors() {
    let hello = "aGVsbG8gd29ybGQ=";
    assert!(Base64TextDetector
        .detect(&RawData::text(hello), Some(&text_parent(hello)))
        .is_some());
    assert!(Base64TextDetector
        .detect(
            &RawData::text("hello world everyone"),
            Some(&text_parent("hello world everyone"))
        )
        .is_none());
    assert!(Base64TextDetector
        .detect(&RawData::text("YWJj"), Some(&text_parent("YWJj")))
        .is_none());

    let gzip = "H4sIAAAAAAAAC/NIzcnJBwCCidH3BQAAAA==";
    assert!(GzipDetector
        .detect(&RawData::text(gzip), Some(&text_parent(gzip)))
        .is_some());

    let uri = "data:image/png;base64,iVBORw0KGgo=";
    assert!(Base64ImageDetector
        .detect(&RawData::text(uri), Some(&text_parent(uri)))
        .is_some());
    let png = "iVBORw0KGgo=";
    assert!(Base64ImageDetector
        .detect(&RawData::text(png), Some(&text_parent(png)))
        .is_some());
}
