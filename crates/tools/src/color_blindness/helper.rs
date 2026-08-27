use std::io::Cursor;

use image::{DynamicImage, ImageFormat, RgbaImage};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ColorBlindnessError {
    #[error("无法解码图像")]
    InvalidImage,
    #[error("无法编码图像")]
    Encode,
}

/// PNG bytes for the original plus three Brettel 1997 (severity=1) simulations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulatedImages {
    pub original: Vec<u8>,
    pub protanopia: Vec<u8>,
    pub deuteranopia: Vec<u8>,
    pub tritanopia: Vec<u8>,
}

/// sRGB, severity 1, applied in linear RGB.
const PROTANOPIA: [[f32; 3]; 3] = [
    [0.152286, 1.052583, -0.204868],
    [0.114503, 0.786281, 0.099216],
    [-0.003882, -0.048116, 1.051998],
];
const DEUTERANOPIA: [[f32; 3]; 3] = [
    [0.367322, 0.860646, -0.227968],
    [0.280085, 0.672501, 0.047413],
    [-0.011820, 0.042940, 0.968881],
];
const TRITANOPIA: [[f32; 3]; 3] = [
    [1.255528, -0.076749, -0.178779],
    [-0.078411, 0.930809, 0.147602],
    [0.004733, 0.691367, 0.303900],
];

pub fn simulate_color_blindness(bytes: &[u8]) -> Result<SimulatedImages, ColorBlindnessError> {
    let decoded = image::load_from_memory(bytes).map_err(|_| ColorBlindnessError::InvalidImage)?;
    let rgba = decoded.to_rgba8();
    Ok(SimulatedImages {
        original: encode_png(&rgba)?,
        protanopia: encode_png(&apply_matrix(&rgba, PROTANOPIA))?,
        deuteranopia: encode_png(&apply_matrix(&rgba, DEUTERANOPIA))?,
        tritanopia: encode_png(&apply_matrix(&rgba, TRITANOPIA))?,
    })
}

fn apply_matrix(src: &RgbaImage, matrix: [[f32; 3]; 3]) -> RgbaImage {
    let mut out = src.clone();
    for pixel in out.pixels_mut() {
        let r = srgb_to_linear(pixel[0]);
        let g = srgb_to_linear(pixel[1]);
        let b = srgb_to_linear(pixel[2]);
        pixel[0] = linear_to_srgb(dot(matrix[0], r, g, b));
        pixel[1] = linear_to_srgb(dot(matrix[1], r, g, b));
        pixel[2] = linear_to_srgb(dot(matrix[2], r, g, b));
    }
    out
}

fn dot(row: [f32; 3], r: f32, g: f32, b: f32) -> f32 {
    row[0] * r + row[1] * g + row[2] * b
}

fn srgb_to_linear(channel: u8) -> f32 {
    let c = channel as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(channel: f32) -> u8 {
    let c = channel.clamp(0.0, 1.0);
    let encoded = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, ColorBlindnessError> {
    let mut buf = Vec::new();
    DynamicImage::ImageRgba8(img.clone())
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .map_err(|_| ColorBlindnessError::Encode)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn red_2x2_png() -> Vec<u8> {
        let img = RgbImage::from_pixel(2, 2, Rgb([255, 0, 0]));
        let mut buf = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn protanopia_png_decodes_and_differs_from_red() {
        let src = red_2x2_png();
        let got = simulate_color_blindness(&src).unwrap();
        let original = image::load_from_memory(&got.original).unwrap().to_rgb8();
        let protan = image::load_from_memory(&got.protanopia).unwrap().to_rgb8();
        assert_eq!(original.dimensions(), (2, 2));
        assert_eq!(protan.dimensions(), (2, 2));
        assert_eq!(*original.get_pixel(0, 0), Rgb([255, 0, 0]));
        assert_ne!(
            protan.as_raw(),
            original.as_raw(),
            "Brettel protanopia matrix maps linear red away from (255,0,0)"
        );
        image::load_from_memory(&got.deuteranopia).unwrap();
        image::load_from_memory(&got.tritanopia).unwrap();
    }

    #[test]
    fn invalid_bytes_are_err() {
        let err = simulate_color_blindness(b"not-an-image").unwrap_err();
        assert_eq!(err, ColorBlindnessError::InvalidImage);
        assert!(!err.to_string().contains("not-an-image"));
    }

    #[test]
    fn metadata_and_cli_match_spec() {
        let meta = crate::color_blindness::metadata();
        assert_eq!(meta.id.as_str(), crate::color_blindness::ID);
        assert_eq!(meta.display_name, "色盲模拟器");
        assert_eq!(meta.group, devtoys_api::GroupId::Graphic);
        assert_eq!(meta.search_keywords, &["color", "色盲", "protanopia"]);
        assert_eq!(meta.accepted_types, &["image", "StaticImageFile"]);
        let cli = crate::color_blindness::cli_tool();
        assert_eq!(cli.name, "colorblindsimulator");
        assert!(cli.aliases.contains(&"cbs"));
        assert_eq!(crate::color_blindness::detectors().len(), 4);
        assert!(crate::color_blindness::STATIC_IMAGE_EXTENSIONS.contains(&".png"));
    }
}
