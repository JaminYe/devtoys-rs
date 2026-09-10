use std::fs;
use std::path::Path;

use crate::slot::ToolView;
use crate::ui;

use super::{decode_base64, encode_bytes, inspect_image};

pub struct Base64ImageView {
    input: String,
    preview: Option<String>,
    decoded: Option<Vec<u8>>,
    tex: Option<egui::TextureHandle>,
    error: Option<String>,
}

impl Base64ImageView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            preview: None,
            decoded: None,
            tex: None,
            error: None,
        }
    }

    fn refresh(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.preview = None;
            self.decoded = None;
            self.tex = None;
            return;
        }
        match decode_base64(&self.input) {
            Ok(bytes) => {
                self.error = None;
                self.preview = Some(inspect_image(&bytes).summary());
                self.decoded = Some(bytes);
                self.tex = None;
            }
            Err(err) => {
                self.preview = None;
                self.decoded = None;
                self.tex = None;
                self.error = Some(err.to_string());
            }
        }
    }
}

impl ToolView for Base64ImageView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui::copy_button(ui, Some(self.input.as_str()));
        });
        ui::error_label(ui, self.error.as_deref());
        if let Some(summary) = &self.preview {
            ui.label(format!("预览：{summary}"));
        }

        let mut changed = false;
        ui::split_2(
            ui,
            |ui| {
                changed = ui::labeled_code(
                    ui,
                    "Base64",
                    "b64-in",
                    &mut self.input,
                    "粘贴 Base64 或 data URI",
                    true,
                );
            },
            |ui| {
                if let Some(bytes) = self.decoded.as_deref() {
                    show_png(ui, "b64-preview", bytes, &mut self.tex);
                }
            },
        );
        if changed {
            self.refresh();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = if Path::new(payload).is_file() {
            match fs::read(payload) {
                Ok(bytes) => encode_bytes(&bytes),
                Err(_) => payload.to_string(),
            }
        } else {
            payload.to_string()
        };
        self.refresh();
    }

    fn on_image_received(&mut self, bytes: &[u8], _mime: Option<&str>) {
        self.input = encode_bytes(bytes);
        self.refresh();
    }

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        None
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        let _ = value;
    }
}

fn show_png(
    ui: &mut egui::Ui,
    name: &'static str,
    bytes: &[u8],
    cache: &mut Option<egui::TextureHandle>,
) {
    if cache.is_none() {
        if let Some(color) = ui::png_image(bytes) {
            *cache = Some(ui.ctx().load_texture(name, color, Default::default()));
        }
    }
    if let Some(tex) = cache.as_ref() {
        ui.add(egui::Image::new(tex).max_size(ui.available_size()));
    } else {
        let info = inspect_image(bytes);
        if info.mime == "image/svg+xml" {
            ui.label(format!("{} 已解码（无法栅格化预览）", info.label));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use devtoys_api::DetectedPayload;
    use image::{Rgba, RgbaImage};
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

    /// Distinct 2×2 pixels used to prove the view received real image bytes.
    const PIXELS: [[u8; 4]; 4] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];

    const OTHER_PIXELS: [[u8; 4]; 4] = [
        [10, 20, 30, 255],
        [40, 50, 60, 255],
        [70, 80, 90, 255],
        [100, 110, 120, 255],
    ];

    fn png_from_pixels(pixels: &[[u8; 4]; 4]) -> Vec<u8> {
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba(pixels[0]));
        img.put_pixel(1, 0, Rgba(pixels[1]));
        img.put_pixel(0, 1, Rgba(pixels[2]));
        img.put_pixel(1, 1, Rgba(pixels[3]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    fn known_png() -> Vec<u8> {
        png_from_pixels(&PIXELS)
    }

    fn other_png() -> Vec<u8> {
        png_from_pixels(&OTHER_PIXELS)
    }

    fn assert_pixels(bytes: &[u8], pixels: &[[u8; 4]; 4]) {
        let img = image::load_from_memory(bytes).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (2, 2));
        assert_eq!(img.get_pixel(0, 0).0, pixels[0]);
        assert_eq!(img.get_pixel(1, 0).0, pixels[1]);
        assert_eq!(img.get_pixel(0, 1).0, pixels[2]);
        assert_eq!(img.get_pixel(1, 1).0, pixels[3]);
    }

    fn independent_decode(b64: &str) -> Vec<u8> {
        STANDARD
            .decode(b64.trim().as_bytes())
            .expect("view output must be standard Base64")
    }

    fn unique_png_path() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "b64img-view-{}-{}",
            std::process::id(),
            DIR_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("in.png")
    }

    #[test]
    fn image_bytes_encode_and_preview_known_pixels() {
        let png = known_png();
        let mut view = Base64ImageView::new();
        view.on_image_received(&png, Some("image/png"));

        assert!(view.error.is_none(), "valid PNG must not error");
        assert_ne!(
            view.input, "image/png",
            "MIME must not be treated as Base64"
        );
        assert!(!view.input.is_empty());

        let decoded = independent_decode(&view.input);
        assert_eq!(decoded, png, "output Base64 must round-trip to input bytes");
        assert_pixels(&decoded, &PIXELS);

        assert_eq!(view.decoded.as_deref(), Some(png.as_slice()));
        let summary = view.preview.as_deref().expect("preview from encoded image");
        assert!(summary.contains("PNG"), "{summary}");
        assert!(summary.contains("2×2"), "{summary}");
    }

    #[test]
    fn detected_payload_bytes_do_not_use_mime_as_base64() {
        let png = known_png();
        let mut view = Base64ImageView::new();
        view.on_detected_data(
            &DetectedPayload::new("image", "image/png")
                .with_bytes(png.clone(), Some("image/png".into())),
        );

        assert_ne!(view.input, "image/png");
        let decoded = independent_decode(&view.input);
        assert_pixels(&decoded, &PIXELS);
        assert_eq!(view.decoded.as_deref(), Some(png.as_slice()));
        assert!(view.error.is_none());
    }

    #[test]
    fn switching_text_and_image_clears_stale_result() {
        let png_a = known_png();
        let png_b = other_png();
        let b64_a = STANDARD.encode(&png_a);
        let mut view = Base64ImageView::new();

        view.on_data_received(&b64_a);
        assert_eq!(view.decoded.as_deref(), Some(png_a.as_slice()));
        assert_pixels(&independent_decode(&view.input), &PIXELS);

        view.on_image_received(&png_b, Some("image/png"));
        assert_ne!(view.input, b64_a);
        let decoded_b = independent_decode(&view.input);
        assert_eq!(decoded_b, png_b);
        assert_pixels(&decoded_b, &OTHER_PIXELS);
        assert_eq!(view.decoded.as_deref(), Some(png_b.as_slice()));
        assert_ne!(view.decoded.as_deref(), Some(png_a.as_slice()));

        view.on_data_received(&b64_a);
        let decoded_a = independent_decode(&view.input);
        assert_eq!(decoded_a, png_a);
        assert_pixels(&decoded_a, &PIXELS);
        assert_eq!(view.decoded.as_deref(), Some(png_a.as_slice()));
        assert_ne!(view.decoded.as_deref(), Some(png_b.as_slice()));
        assert!(view.error.is_none());
    }

    #[test]
    fn data_uri_text_still_decodes() {
        let png = known_png();
        let uri = format!("data:image/png;base64,{}", STANDARD.encode(&png));
        let mut view = Base64ImageView::new();
        view.on_data_received(&uri);

        assert_eq!(view.decoded.as_deref(), Some(png.as_slice()));
        assert_pixels(view.decoded.as_deref().unwrap(), &PIXELS);
        assert!(view.error.is_none());
    }

    #[test]
    fn image_file_path_still_encodes() {
        let png = known_png();
        let path = unique_png_path();
        std::fs::write(&path, &png).unwrap();

        let mut view = Base64ImageView::new();
        view.on_data_received(path.to_str().unwrap());

        let decoded = independent_decode(&view.input);
        assert_eq!(decoded, png);
        assert_pixels(&decoded, &PIXELS);
        assert_eq!(view.decoded.as_deref(), Some(png.as_slice()));
        assert!(view.error.is_none());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn svg_data_uri_decodes_as_success_not_error() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let uri = format!("data:image/svg+xml;base64,{}", STANDARD.encode(svg));
        let mut view = Base64ImageView::new();
        view.on_data_received(&uri);

        assert!(view.error.is_none(), "legal SVG must not be InvalidImage");
        assert_eq!(view.decoded.as_deref(), Some(svg.as_slice()));
        let summary = view.preview.as_deref().expect("SVG summary after decode");
        assert!(summary.contains("SVG"), "{summary}");
        assert!(!summary.contains("非法"), "{summary}");
    }

    #[test]
    fn persistable_options_is_none_even_with_image() {
        let png = known_png();
        let mut view = Base64ImageView::new();
        view.on_image_received(&png, Some("image/png"));
        assert!(!view.input.is_empty());
        assert!(view.decoded.is_some());
        assert!(
            view.persistable_options().is_none(),
            "Base64 image has no non-sensitive options"
        );
    }

    #[test]
    fn restore_options_does_not_apply_content() {
        let png = known_png();
        let mut view = Base64ImageView::new();
        view.on_image_received(&png, Some("image/png"));
        let before_input = view.input.clone();
        let before_decoded = view.decoded.clone();
        view.restore_options(&serde_json::json!({
            "input": "injected-base64",
            "bytes": [1, 2, 3],
            "format": "png",
            "path": r"C:\secret\in.png"
        }));
        assert_eq!(view.input, before_input);
        assert_eq!(view.decoded, before_decoded);
        assert!(view.persistable_options().is_none());
    }
}
