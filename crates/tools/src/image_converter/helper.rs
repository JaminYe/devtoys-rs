use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageFormat};

use crate::color_blindness::is_static_image_path;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ImageConvertError {
    #[error("无法解码图像")]
    InvalidImage,
    #[error("未知格式")]
    UnknownFormat,
    #[error("无法编码图像")]
    Encode,
    #[error("无法读取输入")]
    Read,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageTargetFormat {
    Bmp,
    Jpeg,
    Pbm,
    #[default]
    Png,
    Tga,
    Tiff,
    Webp,
}

impl ImageTargetFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Bmp" | "bmp" | "BMP" => Some(Self::Bmp),
            "Jpeg" | "jpeg" | "JPEG" | "jpg" | "JPG" => Some(Self::Jpeg),
            "Pbm" | "pbm" | "PBM" => Some(Self::Pbm),
            "Png" | "png" | "PNG" => Some(Self::Png),
            "Tga" | "tga" | "TGA" => Some(Self::Tga),
            "Tiff" | "tiff" | "TIFF" => Some(Self::Tiff),
            "Webp" | "webp" | "WEBP" => Some(Self::Webp),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bmp => "Bmp",
            Self::Jpeg => "Jpeg",
            Self::Pbm => "Pbm",
            Self::Png => "Png",
            Self::Tga => "Tga",
            Self::Tiff => "Tiff",
            Self::Webp => "Webp",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Bmp => "bmp",
            Self::Jpeg => "jpg",
            Self::Pbm => "pbm",
            Self::Png => "png",
            Self::Tga => "tga",
            Self::Tiff => "tiff",
            Self::Webp => "webp",
        }
    }

    fn image_format(self) -> ImageFormat {
        match self {
            Self::Bmp => ImageFormat::Bmp,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Pbm => ImageFormat::Pnm,
            Self::Png => ImageFormat::Png,
            Self::Tga => ImageFormat::Tga,
            Self::Tiff => ImageFormat::Tiff,
            Self::Webp => ImageFormat::WebP,
        }
    }
}

pub fn convert_image(bytes: &[u8], format: ImageTargetFormat) -> Result<Vec<u8>, ImageConvertError> {
    let img = image::load_from_memory(bytes).map_err(|_| ImageConvertError::InvalidImage)?;
    encode(img, format)
}

pub fn convert_image_named(
    bytes: &[u8],
    format: &str,
) -> Result<Vec<u8>, ImageConvertError> {
    let format = ImageTargetFormat::parse(format).ok_or(ImageConvertError::UnknownFormat)?;
    convert_image(bytes, format)
}

/// Direct children only; never recurse.
pub fn static_images_in_dir(dir: &Path) -> Result<Vec<PathBuf>, ImageConvertError> {
    let entries = std::fs::read_dir(dir).map_err(|_| ImageConvertError::Read)?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| ImageConvertError::Read)?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.to_str().is_some_and(is_static_image_path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn encode(img: DynamicImage, format: ImageTargetFormat) -> Result<Vec<u8>, ImageConvertError> {
    let img = match format {
        ImageTargetFormat::Jpeg => DynamicImage::ImageRgb8(img.to_rgb8()),
        ImageTargetFormat::Pbm => DynamicImage::ImageLuma8(img.to_luma8()),
        _ => img,
    };
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), format.image_format())
        .map_err(|_| ImageConvertError::Encode)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn rgb_1x1_png() -> Vec<u8> {
        let img = RgbImage::from_pixel(1, 1, Rgb([10, 20, 30]));
        let mut buf = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn png_to_jpeg_loads_as_jpeg() {
        let jpeg = convert_image(&rgb_1x1_png(), ImageTargetFormat::Jpeg).unwrap();
        assert_eq!(image::guess_format(&jpeg).unwrap(), ImageFormat::Jpeg);
        let loaded = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_eq!(loaded.dimensions(), (1, 1));
    }

    #[test]
    fn unknown_format_is_err() {
        let err = convert_image_named(&rgb_1x1_png(), "gif").unwrap_err();
        assert_eq!(err, ImageConvertError::UnknownFormat);
        assert!(!err.to_string().contains("gif"));
        assert_eq!(ImageTargetFormat::Jpeg.as_str(), "Jpeg");
        assert_eq!(ImageTargetFormat::Png.extension(), "png");
    }

    #[test]
    fn metadata_and_cli_match_spec() {
        let meta = crate::image_converter::metadata();
        assert_eq!(meta.id.as_str(), crate::image_converter::ID);
        assert_eq!(meta.display_name, "图片格式转换器");
        assert_eq!(meta.group, devtoys_api::GroupId::Graphic);
        assert_eq!(meta.search_keywords, &["png", "jpeg", "webp", "bmp"]);
        assert_eq!(
            meta.accepted_types,
            &["image", "StaticImageFile", "StaticImageFiles"]
        );
        let cli = crate::image_converter::cli_tool();
        assert_eq!(cli.name, "imageconverter");
        assert!(cli.aliases.contains(&"imgconv"));
        assert_eq!(crate::image_converter::detectors().len(), 1);
    }

    #[test]
    fn invalid_bytes_are_err() {
        let err = convert_image(b"nope", ImageTargetFormat::Png).unwrap_err();
        assert_eq!(err, ImageConvertError::InvalidImage);
    }
}
