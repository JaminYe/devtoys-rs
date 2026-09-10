use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use devtoys_api::{
    GroupId, RawData, ToolId, ToolMetadata, TYPE_FILE, TYPE_FILES, TYPE_IMAGE, TYPE_IMAGE_FILE,
    TYPE_JSON, TYPE_TEXT,
};
use devtoys_core::{
    all_detectors, files_from_os_paths, parse_cf_hdrop, rgba_to_png, ClipboardSource,
    DetectOptions, DetectionEngine, InMemoryClipboard, SystemClipboard,
};

#[test]
fn test_in_memory_clipboard_empty_and_text() {
    let clip = InMemoryClipboard::new();
    assert_eq!(clip.read_raw(), None);

    clip.set_text("test string 123");
    let raw = clip.read_raw();
    assert_eq!(raw, Some(RawData::text("test string 123")));
    assert_eq!(
        raw.as_ref().and_then(|r| r.as_text()),
        Some("test string 123")
    );
}

#[test]
fn test_in_memory_clipboard_image() {
    let clip = InMemoryClipboard::new();
    let img_bytes = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
    clip.set_image(img_bytes.clone(), Some("image/png".to_string()));

    let raw = clip.read_raw();
    assert!(raw.is_some());
    match raw.unwrap() {
        RawData::Image { bytes, mime } => {
            assert_eq!(bytes, img_bytes);
            assert_eq!(mime, Some("image/png".to_string()));
        }
        _ => panic!("Expected RawData::Image"),
    }
}

#[test]
fn test_in_memory_clipboard_clear_and_none() {
    let clip = InMemoryClipboard::new();
    clip.set_text("temporary text");
    assert!(clip.read_raw().is_some());

    clip.clear();
    assert_eq!(clip.read_raw(), None);

    clip.set_text("another text");
    assert!(clip.read_raw().is_some());

    clip.set(None);
    assert_eq!(clip.read_raw(), None);
}

#[test]
fn test_in_memory_clipboard_concurrent_access() {
    let clip = Arc::new(InMemoryClipboard::new());
    let mut handles = Vec::new();

    for i in 0..10 {
        let clip_clone = Arc::clone(&clip);
        handles.push(std::thread::spawn(move || {
            clip_clone.set_text(format!("message from thread {i}"));
            let _ = clip_clone.read_raw();
        }));
    }

    for h in handles {
        h.join().expect("thread should finish without panic");
    }

    // Must still be readable after thread execution
    let final_val = clip.read_raw();
    assert!(final_val.is_some());
}

#[test]
fn test_clipboard_source_trait_object() {
    let in_mem = InMemoryClipboard::new();
    in_mem.set_text("trait object check");

    let source: Box<dyn ClipboardSource> = Box::new(in_mem);
    assert_eq!(source.read_raw(), Some(RawData::text("trait object check")));
}

#[test]
fn test_system_clipboard_smoke() {
    let clip = SystemClipboard::new();
    // read_raw must not panic regardless of whether clipboard contains text, image, or is empty/inaccessible
    let result = clip.read_raw();
    // Result may be Some or None depending on the host OS clipboard state during test run,
    // but the call must succeed safely without panicking.
    if let Some(data) = result {
        match data {
            RawData::Text(t) => assert!(!t.is_empty()),
            RawData::Image { bytes, mime } => {
                assert!(!bytes.is_empty());
                assert_eq!(mime, Some("image/png".to_string()));
            }
            RawData::Files(f) => assert!(!f.is_empty()),
        }
    }

    let default_clip = SystemClipboard::default();
    let _ = default_clip.read_raw();
}

fn known_rgba_pixels() -> (u32, u32, Vec<u8>, [[u8; 4]; 4]) {
    let pixels = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];
    let mut bytes = Vec::with_capacity(16);
    for px in pixels {
        bytes.extend_from_slice(&px);
    }
    (2, 2, bytes, pixels)
}

fn image_tool() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new("ImageConverter"),
        display_name: "图片格式转换器",
        search_keywords: &[],
        group: GroupId::Graphic,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_IMAGE],
    }
}

fn json_tool() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new("JsonFormatter"),
        display_name: "JSON",
        search_keywords: &[],
        group: GroupId::Formatters,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_JSON],
    }
}

fn hash_tool() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new("HashAndChecksumGenerator"),
        display_name: "哈希 / 校验和",
        search_keywords: &[],
        group: GroupId::Generators,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_TEXT, TYPE_FILE],
    }
}

fn base64_image_tool() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new("Base64ImageEncoderDecoder"),
        display_name: "Base64 图片",
        search_keywords: &[],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_IMAGE, TYPE_IMAGE_FILE],
    }
}

fn image_converter_files_tool() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new("ImageConverter"),
        display_name: "图片格式转换器",
        search_keywords: &[],
        group: GroupId::Graphic,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_IMAGE, TYPE_IMAGE_FILE],
    }
}

fn files_sink_tool() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new("FilesSink"),
        display_name: "FilesSink",
        search_keywords: &[],
        group: GroupId::Graphic,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_FILES],
    }
}

fn file_routing_tools() -> Vec<ToolMetadata> {
    vec![
        image_converter_files_tool(),
        hash_tool(),
        base64_image_tool(),
        json_tool(),
        files_sink_tool(),
    ]
}

fn detect_files(files: Vec<String>) -> Vec<devtoys_core::Recommendation> {
    let clip = InMemoryClipboard::new();
    clip.set_files(files);
    let raw = clip.read_raw().expect("clipboard files");
    let engine = DetectionEngine::new(all_detectors(), &file_routing_tools());
    engine.detect(
        &raw,
        DetectOptions {
            strict: false,
            active_tool: None,
            enabled: true,
            cancel: &AtomicBool::new(false),
        },
    )
}

fn rec_ids(recs: &[devtoys_core::Recommendation]) -> Vec<&str> {
    recs.iter().map(|r| r.tool_id.as_str()).collect()
}

/// Clipboard source → detect → recommend keeps real PNG pixels, not the MIME string.
#[test]
fn clipboard_png_detect_recommend_keeps_known_pixels() {
    let (w, h, rgba, pixels) = known_rgba_pixels();
    let png = rgba_to_png(w, h, &rgba).expect("encode known pixels");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));

    let clip = InMemoryClipboard::new();
    clip.set_image(png.clone(), Some("image/png".to_string()));
    let raw = clip.read_raw().expect("clipboard image");

    let engine = DetectionEngine::new(all_detectors(), &[image_tool()]);
    let recs = engine.detect(
        &raw,
        DetectOptions {
            strict: false,
            active_tool: None,
            enabled: true,
            cancel: &AtomicBool::new(false),
        },
    );
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].tool_id, "ImageConverter");
    assert_eq!(recs[0].data_type, TYPE_IMAGE);
    assert_eq!(recs[0].payload, "image/png");
    let bytes = recs[0]
        .bytes
        .as_ref()
        .expect("recommendation must carry image bytes");
    assert_eq!(bytes, &png);
    assert_ne!(bytes.as_slice(), b"image/png");

    let loaded = image::load_from_memory(bytes).unwrap().to_rgba8();
    assert_eq!(loaded.dimensions(), (w, h));
    assert_eq!(loaded.get_pixel(0, 0).0, pixels[0]);
    assert_eq!(loaded.get_pixel(1, 0).0, pixels[1]);
    assert_eq!(loaded.get_pixel(0, 1).0, pixels[2]);
    assert_eq!(loaded.get_pixel(1, 1).0, pixels[3]);

    let pasted = recs[0].paste_payload(Some(&raw));
    assert_eq!(pasted.image_bytes(), Some(png.as_slice()));
    assert_eq!(pasted.mime.as_deref(), Some("image/png"));
}

#[test]
fn clipboard_image_mime_only_recommendation_falls_back_to_raw_bytes() {
    let (w, h, rgba, pixels) = known_rgba_pixels();
    let png = rgba_to_png(w, h, &rgba).unwrap();
    let raw = RawData::Image {
        bytes: png.clone(),
        mime: Some("image/png".into()),
    };
    let hit = devtoys_core::Recommendation::new("ImageConverter", TYPE_IMAGE, "image/png");
    assert!(hit.bytes.is_none());
    let pasted = hit.paste_payload(Some(&raw));
    let bytes = pasted.image_bytes().expect("clipboard fallback");
    assert_eq!(bytes, png.as_slice());
    let loaded = image::load_from_memory(bytes).unwrap().to_rgba8();
    assert_eq!(loaded.dimensions(), (w, h));
    assert_eq!(loaded.get_pixel(0, 0).0, pixels[0]);
}

#[test]
fn text_clipboard_smart_paste_still_recommends_json() {
    let clip = InMemoryClipboard::new();
    clip.set_text(r#"{"key":1}"#);
    let raw = clip.read_raw().unwrap();
    let engine = DetectionEngine::new(all_detectors(), &[json_tool(), image_tool()]);
    let recs = engine.detect(
        &raw,
        DetectOptions {
            strict: false,
            active_tool: None,
            enabled: true,
            cancel: &AtomicBool::new(false),
        },
    );
    assert!(recs.iter().any(|r| r.tool_id == "JsonFormatter"));
    assert!(recs.iter().all(|r| r.bytes.is_none()));
    let json = recs.iter().find(|r| r.tool_id == "JsonFormatter").unwrap();
    let pasted = json.paste_payload(Some(&raw));
    assert_eq!(pasted.value, r#"{"key":1}"#);
    assert!(pasted.image_bytes().is_none());
}

/// InMemoryClipboard Files is the detector path, not OS clipboard E2E.
#[test]
fn clipboard_single_image_file_recommends_converter_base64_and_hash() {
    let path = r"C:\Users\图片\foo bar.png";
    let recs = detect_files(vec![path.into()]);
    let ids = rec_ids(&recs);
    assert!(ids.contains(&"ImageConverter"), "{ids:?}");
    assert!(ids.contains(&"Base64ImageEncoderDecoder"), "{ids:?}");
    assert!(ids.contains(&"HashAndChecksumGenerator"), "{ids:?}");
    assert!(!ids.contains(&"JsonFormatter"), "{ids:?}");
    assert!(!ids.contains(&"FilesSink"), "{ids:?}");
    for rec in &recs {
        assert_eq!(rec.payload, path);
        let pasted = rec.paste_payload(None);
        assert_eq!(pasted.value, path);
        assert!(pasted.image_bytes().is_none());
    }
}

#[test]
fn clipboard_multi_image_and_mixed_files_keep_real_paths() {
    let png_a = r"D:\shots\a 1.png";
    let png_b = r"D:\shots\b.png";
    let txt = r"D:\notes\说明.txt";

    let multi = detect_files(vec![png_a.into(), png_b.into()]);
    assert_eq!(rec_ids(&multi), vec!["FilesSink"]);
    assert_eq!(multi[0].data_type, TYPE_FILES);
    assert_eq!(multi[0].payload, format!("{png_a}\n{png_b}"));
    assert_eq!(
        multi[0].paste_payload(None).value,
        format!("{png_a}\n{png_b}")
    );

    let mixed = detect_files(vec![png_a.into(), txt.into()]);
    assert!(mixed.iter().all(|r| r.data_type == TYPE_FILES), "{mixed:?}");
    assert_eq!(mixed[0].payload, format!("{png_a}\n{txt}"));
    let mixed_ids = rec_ids(&mixed);
    assert!(!mixed_ids.contains(&"ImageConverter"), "{mixed_ids:?}");
    assert!(
        !mixed_ids.contains(&"Base64ImageEncoderDecoder"),
        "{mixed_ids:?}"
    );
    assert!(
        !mixed_ids.contains(&"HashAndChecksumGenerator"),
        "{mixed_ids:?}"
    );
}

#[test]
fn clipboard_missing_file_still_routes_by_extension() {
    let missing_png = r"C:\does not exist\gone.png";
    let recs = detect_files(vec![missing_png.into()]);
    assert!(rec_ids(&recs).contains(&"ImageConverter"));
    assert!(recs.iter().all(|r| r.payload == missing_png));

    let missing_txt = r"C:\does not exist\gone.txt";
    let recs = detect_files(vec![missing_txt.into()]);
    assert!(rec_ids(&recs).contains(&"HashAndChecksumGenerator"));
    assert!(!rec_ids(&recs).contains(&"ImageConverter"));
    let hash = recs
        .iter()
        .find(|r| r.tool_id == "HashAndChecksumGenerator")
        .unwrap();
    assert_eq!(hash.data_type, TYPE_FILE);
    assert_eq!(hash.payload, missing_txt);
}

#[test]
fn clipboard_path_text_is_text_not_files() {
    let clip = InMemoryClipboard::new();
    clip.set_text(r"C:\Users\图片\foo.png");
    let raw = clip.read_raw().unwrap();
    assert_eq!(raw.as_text(), Some(r"C:\Users\图片\foo.png"));
    assert!(raw.as_files().is_none());

    let engine = DetectionEngine::new(all_detectors(), &file_routing_tools());
    let recs = engine.detect(
        &raw,
        DetectOptions {
            strict: false,
            active_tool: None,
            enabled: true,
            cancel: &AtomicBool::new(false),
        },
    );
    assert!(recs
        .iter()
        .all(|r| r.data_type != TYPE_FILES && r.data_type != TYPE_FILE));
    assert!(recs
        .iter()
        .all(|r| r.data_type != TYPE_IMAGE && r.data_type != TYPE_IMAGE_FILE));
}

#[test]
fn clipboard_pixel_image_is_not_files() {
    let (w, h, rgba, _) = known_rgba_pixels();
    let png = rgba_to_png(w, h, &rgba).unwrap();
    let clip = InMemoryClipboard::new();
    clip.set_image(png.clone(), Some("image/png".into()));
    match clip.read_raw() {
        Some(RawData::Image { bytes, .. }) => assert_eq!(bytes, png),
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn clipboard_cf_hdrop_parser_is_not_system_e2e() {
    // Mocked CF_HDROP bytes; does not prove Explorer clipboard round-trip.
    let mut payload = vec![0u8; 20];
    payload[0..4].copy_from_slice(&20u32.to_le_bytes());
    payload[16..20].copy_from_slice(&1i32.to_le_bytes());
    for unit in r"C:\Users\图片\foo bar.png".encode_utf16().chain([0, 0]) {
        payload.extend_from_slice(&unit.to_le_bytes());
    }
    assert_eq!(
        parse_cf_hdrop(&payload).unwrap(),
        vec![r"C:\Users\图片\foo bar.png".to_string()]
    );
    match files_from_os_paths([std::path::Path::new(r"C:\missing 文件.txt")]) {
        Some(RawData::Files(files)) => assert_eq!(files, vec![r"C:\missing 文件.txt"]),
        other => panic!("expected Files, got {other:?}"),
    }
}

#[test]
fn test_rgba_to_png_dirty_and_corrupt_data() {
    // 1. Zero dimensions
    assert_eq!(rgba_to_png(0, 0, &[]), None);
    assert_eq!(rgba_to_png(0, 4, &[0; 16]), None);
    assert_eq!(rgba_to_png(4, 0, &[0; 16]), None);

    // 2. Truncated buffer (declared 2x2 = 16 bytes, given 15)
    assert_eq!(rgba_to_png(2, 2, &[255; 15]), None);

    // 3. Excess buffer (declared 1x1 = 4 bytes, given 8)
    assert_eq!(rgba_to_png(1, 1, &[255; 8]), None);

    // 4. Valid conversion
    let valid_pixels = vec![
        100, 150, 200, 255, // px 0,0
        50, 75, 100, 255, // px 1,0
        0, 0, 0, 255, // px 0,1
        255, 255, 255, 255, // px 1,1
    ];
    let png = rgba_to_png(2, 2, &valid_pixels);
    assert!(png.is_some());
    let bytes = png.unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
}
