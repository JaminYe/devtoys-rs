use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, ImageFormat};
use qrcode::render::svg;
use qrcode::QrCode;
use rqrr::PreparedImage;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum QrError {
    #[error("无法生成二维码")]
    EncodeFailed,
    #[error("无法识别二维码")]
    DecodeFailed,
    #[error("非法图像")]
    InvalidImage,
}

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff", "tga", "ico", "pbm", "pgm", "ppm",
];

pub fn encode_svg(text: &str) -> Result<String, QrError> {
    let code = QrCode::new(text.as_bytes()).map_err(|_| QrError::EncodeFailed)?;
    let svg = code.render::<svg::Color<'_>>().build();
    if svg.is_empty() {
        return Err(QrError::EncodeFailed);
    }
    Ok(svg)
}

pub fn encode_png(text: &str) -> Result<Vec<u8>, QrError> {
    let code = QrCode::new(text.as_bytes()).map_err(|_| QrError::EncodeFailed)?;
    let img = code.render::<image::Luma<u8>>().build();
    let dynamic = DynamicImage::ImageLuma8(img);
    let mut buf = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, ImageFormat::Png)
        .map_err(|_| QrError::EncodeFailed)?;
    Ok(buf.into_inner())
}

pub fn decode_image_bytes(bytes: &[u8]) -> Result<String, QrError> {
    let img = image::load_from_memory(bytes).map_err(|_| QrError::InvalidImage)?;
    decode_dynamic(&img)
}

pub fn decode_image_path(path: &Path) -> Result<String, QrError> {
    let img = image::open(path).map_err(|_| QrError::InvalidImage)?;
    decode_dynamic(&img)
}

pub fn is_existing_image_file(path: &str) -> bool {
    let path = Path::new(path);
    if !path.is_file() {
        return false;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if IMAGE_EXTS.iter().any(|x| ext.eq_ignore_ascii_case(x)) {
            return true;
        }
    }
    image::open(path).is_ok()
}

fn decode_dynamic(img: &DynamicImage) -> Result<String, QrError> {
    let luma = img.to_luma8();
    let mut prepared = PreparedImage::prepare(luma);
    let grids = prepared.detect_grids();
    if grids.is_empty() {
        return Err(QrError::DecodeFailed);
    }
    let mut texts = Vec::new();
    for grid in grids {
        let (_, content) = grid.decode().map_err(|_| QrError::DecodeFailed)?;
        texts.push(content);
    }
    if texts.is_empty() {
        return Err(QrError::DecodeFailed);
    }
    Ok(texts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_hello_svg_contains_svg_tag() {
        let svg = encode_svg("hello").unwrap();
        assert!(!svg.is_empty());
        assert!(
            svg.contains("<svg"),
            "SVG output should contain <svg, got {} bytes",
            svg.len()
        );
    }

    #[test]
    fn invalid_image_decode_is_err() {
        let err = decode_image_bytes(b"not-an-image").unwrap_err();
        assert_eq!(err, QrError::InvalidImage);
        assert!(!err.to_string().contains("not-an-image"));
    }

    #[test]
    fn png_roundtrip_hello() {
        let png = encode_png("hello").unwrap();
        assert!(!png.is_empty());
        let text = decode_image_bytes(&png).unwrap();
        assert_eq!(text, "hello");
    }
}
