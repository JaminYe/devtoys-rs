use devtoys_api::RawData;
use devtoys_core::{rgba_to_png, ClipboardSource, InMemoryClipboard, SystemClipboard};
use std::sync::Arc;

#[test]
fn test_in_memory_clipboard_empty_and_text() {
    let clip = InMemoryClipboard::new();
    assert_eq!(clip.read_raw(), None);

    clip.set_text("test string 123");
    let raw = clip.read_raw();
    assert_eq!(raw, Some(RawData::text("test string 123")));
    assert_eq!(raw.as_ref().and_then(|r| r.as_text()), Some("test string 123"));
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
        50, 75, 100, 255,   // px 1,0
        0, 0, 0, 255,       // px 0,1
        255, 255, 255, 255, // px 1,1
    ];
    let png = rgba_to_png(2, 2, &valid_pixels);
    assert!(png.is_some());
    let bytes = png.unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
}
