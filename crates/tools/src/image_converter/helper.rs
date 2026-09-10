use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::pnm::{PnmEncoder, PnmSubtype, SampleEncoding};
use image::{DynamicImage, ExtendedColorType, ImageFormat, ImageReader};

use super::is_static_image_path;

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
    #[error("无法写入文件")]
    Write,
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

/// TGA has no magic number, so `load_from_memory` / `guess_format` cannot identify it.
fn decode_tga(bytes: &[u8]) -> Result<DynamicImage, ImageConvertError> {
    let decoder = image::codecs::tga::TgaDecoder::new(Cursor::new(bytes))
        .map_err(|_| ImageConvertError::InvalidImage)?;
    DynamicImage::from_decoder(decoder).map_err(|_| ImageConvertError::InvalidImage)
}

fn decode_with_format(
    bytes: &[u8],
    format: ImageFormat,
) -> Result<DynamicImage, ImageConvertError> {
    ImageReader::with_format(Cursor::new(bytes), format)
        .decode()
        .map_err(|_| ImageConvertError::InvalidImage)
}

/// Formats with magic numbers go through `load_from_memory`; TGA falls back to `TgaDecoder`.
pub fn decode_image(bytes: &[u8]) -> Result<DynamicImage, ImageConvertError> {
    match image::load_from_memory(bytes) {
        Ok(img) => Ok(img),
        Err(_) => decode_tga(bytes),
    }
}

pub fn format_hint_from_path(path: &Path) -> Option<ImageFormat> {
    ImageFormat::from_path(path).ok()
}

pub fn decode_image_hinted(
    bytes: &[u8],
    hint: Option<ImageFormat>,
) -> Result<DynamicImage, ImageConvertError> {
    match hint {
        Some(ImageFormat::Tga) => {
            decode_with_format(bytes, ImageFormat::Tga).or_else(|_| decode_tga(bytes))
        }
        Some(format) => decode_with_format(bytes, format).or_else(|_| decode_image(bytes)),
        None => decode_image(bytes),
    }
}

/// Preview of our own encoded output: use the known target format, never `guess_format`.
pub fn decode_preview(
    bytes: &[u8],
    target: ImageTargetFormat,
) -> Result<DynamicImage, ImageConvertError> {
    decode_image_hinted(bytes, Some(target.image_format()))
}

pub fn preview_rgba(
    bytes: &[u8],
    target: ImageTargetFormat,
) -> Result<(u32, u32, Vec<u8>), ImageConvertError> {
    let img = decode_preview(bytes, target)?.into_rgba8();
    Ok((img.width(), img.height(), img.into_raw()))
}

/// Suggested file name for a clipboard/smart-paste image (no path).
pub fn clipboard_source_name(mime: Option<&str>) -> PathBuf {
    let ext = match mime {
        Some("image/jpeg" | "image/jpg") => "jpg",
        Some("image/bmp") => "bmp",
        Some("image/webp") => "webp",
        Some("image/tiff") => "tiff",
        Some("image/tga" | "image/x-tga") => "tga",
        Some("image/x-portable-bitmap" | "image/x-portable-anymap") => "pbm",
        _ => "png",
    };
    PathBuf::from(format!("clipboard.{ext}"))
}

pub fn convert_image(
    bytes: &[u8],
    format: ImageTargetFormat,
) -> Result<Vec<u8>, ImageConvertError> {
    encode(decode_image(bytes)?, format)
}

/// Prefer when the source path is known so `.tga` uses format-from-extension.
pub fn convert_image_from(
    bytes: &[u8],
    source: &Path,
    format: ImageTargetFormat,
) -> Result<Vec<u8>, ImageConvertError> {
    encode(
        decode_image_hinted(bytes, format_hint_from_path(source))?,
        format,
    )
}

pub fn convert_image_named(bytes: &[u8], format: &str) -> Result<Vec<u8>, ImageConvertError> {
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
    if format == ImageTargetFormat::Pbm {
        return encode_pbm(img);
    }
    let img = match format {
        ImageTargetFormat::Jpeg => DynamicImage::ImageRgb8(img.to_rgb8()),
        _ => img,
    };
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), format.image_format())
        .map_err(|_| ImageConvertError::Encode)?;
    Ok(buf)
}

/// PBM is 1-bit. Threshold luma to 0/1 and write P4 (binary bitmap), not PAM/P7.
fn encode_pbm(img: DynamicImage) -> Result<Vec<u8>, ImageConvertError> {
    let luma = img.to_luma8();
    let (width, height) = luma.dimensions();
    let bits: Vec<u8> = luma
        .into_raw()
        .into_iter()
        .map(|v| u8::from(v >= 128))
        .collect();
    let mut buf = Vec::new();
    PnmEncoder::new(&mut buf)
        .with_subtype(PnmSubtype::Bitmap(SampleEncoding::Binary))
        .encode(bits.as_slice(), width, height, ExtendedColorType::L8)
        .map_err(|_| ImageConvertError::Encode)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use std::path::PathBuf;

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
    fn clipboard_source_name_uses_mime_extension() {
        assert_eq!(
            clipboard_source_name(Some("image/jpeg")).as_os_str(),
            "clipboard.jpg"
        );
        assert_eq!(
            clipboard_source_name(Some("image/png")).as_os_str(),
            "clipboard.png"
        );
        assert_eq!(clipboard_source_name(None).as_os_str(), "clipboard.png");
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
    fn metadata_matches_spec() {
        let meta = crate::image_converter::metadata();
        assert_eq!(meta.id.as_str(), crate::image_converter::ID);
        assert_eq!(meta.display_name, "图片格式转换器");
        assert_eq!(meta.group, devtoys_api::GroupId::Graphic);
        assert_eq!(meta.search_keywords, &["png", "jpeg", "webp", "bmp"]);
        assert_eq!(
            meta.accepted_types,
            &["image", "StaticImageFile", "StaticImageFiles"]
        );
        assert_eq!(crate::image_converter::detectors().len(), 2);
    }

    #[test]
    fn invalid_bytes_are_err() {
        let err = convert_image(b"nope", ImageTargetFormat::Png).unwrap_err();
        assert_eq!(err, ImageConvertError::InvalidImage);
    }

    static DIR_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn rgb_2x2_png() -> (Vec<u8>, [u8; 3], [u8; 3]) {
        let a = Rgb([10, 20, 30]);
        let b = Rgb([40, 50, 60]);
        let mut img = RgbImage::new(2, 2);
        img.put_pixel(0, 0, a);
        img.put_pixel(1, 0, b);
        img.put_pixel(0, 1, a);
        img.put_pixel(1, 1, b);
        let mut buf = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        (buf, a.0, b.0)
    }

    fn unique_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "imgconv-tga-{}-{}-{}",
            std::process::id(),
            DIR_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            label
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn assert_independent_tga(tga: &[u8], rgb_a: [u8; 3], rgb_b: [u8; 3]) {
        let via_reader = ImageReader::with_format(Cursor::new(tga), ImageFormat::Tga)
            .decode()
            .expect("ImageReader Tga")
            .to_rgb8();
        assert_eq!(via_reader.dimensions(), (2, 2));
        assert_eq!(via_reader.get_pixel(0, 0).0, rgb_a);
        assert_eq!(via_reader.get_pixel(1, 0).0, rgb_b);

        let decoder = image::codecs::tga::TgaDecoder::new(Cursor::new(tga)).expect("TgaDecoder");
        let via_codec = DynamicImage::from_decoder(decoder).unwrap().to_rgb8();
        assert_eq!(via_codec.dimensions(), (2, 2));
        assert_eq!(via_codec.get_pixel(0, 0).0, rgb_a);
        assert_eq!(via_codec.get_pixel(1, 1).0, rgb_b);
    }

    #[test]
    fn png_tga_png_roundtrip_in_temp_dir() {
        let (png, rgb_a, rgb_b) = rgb_2x2_png();
        let dir = unique_dir("roundtrip");
        let png_path = dir.join("src.png");
        std::fs::write(&png_path, &png).unwrap();

        let tga = convert_image(&png, ImageTargetFormat::Tga).unwrap();
        let tga_path = dir.join("out.tga");
        std::fs::write(&tga_path, &tga).unwrap();

        assert!(image::guess_format(&tga).is_err());
        assert!(image::load_from_memory(&tga).is_err());
        assert_independent_tga(&tga, rgb_a, rgb_b);

        let tga_on_disk = std::fs::read(&tga_path).unwrap();
        let png_back = convert_image_from(&tga_on_disk, &tga_path, ImageTargetFormat::Png).unwrap();
        let loaded = image::load_from_memory(&png_back).unwrap().to_rgb8();
        assert_eq!(loaded.dimensions(), (2, 2));
        assert_eq!(loaded.get_pixel(0, 0).0, rgb_a);
        assert_eq!(loaded.get_pixel(1, 0).0, rgb_b);

        let png_unhinted = convert_image(&tga_on_disk, ImageTargetFormat::Png).unwrap();
        let unhinted = image::load_from_memory(&png_unhinted).unwrap().to_rgb8();
        assert_eq!(unhinted.get_pixel(0, 0).0, rgb_a);

        let bmp = convert_image_from(&tga_on_disk, &tga_path, ImageTargetFormat::Bmp).unwrap();
        assert_eq!(image::guess_format(&bmp).unwrap(), ImageFormat::Bmp);
        let bmp_img = image::load_from_memory(&bmp).unwrap().to_rgb8();
        assert_eq!(bmp_img.dimensions(), (2, 2));
        assert_eq!(bmp_img.get_pixel(1, 0).0, rgb_b);

        let batch = crate::image_converter::convert_paths(
            &[tga_path.to_string_lossy().into_owned()],
            ImageTargetFormat::Png,
        );
        assert_eq!(batch.succeeded(), 1);
        assert_eq!(batch.failed(), 0);
        let from_paths = image::load_from_memory(&batch.successes[0].bytes)
            .unwrap()
            .to_rgb8();
        assert_eq!(from_paths.get_pixel(0, 0).0, rgb_a);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_decodes_tga_without_guess_format() {
        let (png, rgb_a, rgb_b) = rgb_2x2_png();
        let tga = convert_image(&png, ImageTargetFormat::Tga).unwrap();
        assert!(image::guess_format(&tga).is_err());
        assert!(image::load_from_memory(&tga).is_err());

        let (w, h, rgba) = preview_rgba(&tga, ImageTargetFormat::Tga).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgba[0..3], &rgb_a);
        assert_eq!(rgba[3], 255);
        let stride = (w as usize) * 4;
        assert_eq!(&rgba[4..7], &rgb_b);

        let png_preview = preview_rgba(&png, ImageTargetFormat::Png).unwrap();
        assert_eq!(png_preview.0, 2);
        assert_eq!(&png_preview.2[0..3], &rgb_a);
        assert_eq!(rgba.len(), stride * h as usize);
    }

    #[test]
    fn corrupt_tga_fails_clearly() {
        let dir = unique_dir("bad");
        let path = dir.join("bad.tga");
        std::fs::write(&path, b"not a targa").unwrap();
        let bytes = std::fs::read(&path).unwrap();

        let err = convert_image_from(&bytes, &path, ImageTargetFormat::Png).unwrap_err();
        assert_eq!(err, ImageConvertError::InvalidImage);
        assert_eq!(err.to_string(), "无法解码图像");

        let err = convert_image(&bytes, ImageTargetFormat::Png).unwrap_err();
        assert_eq!(err, ImageConvertError::InvalidImage);

        let err = preview_rgba(&bytes, ImageTargetFormat::Tga).unwrap_err();
        assert_eq!(err, ImageConvertError::InvalidImage);

        let (png, _, _) = rgb_2x2_png();
        let tga = convert_image(&png, ImageTargetFormat::Tga).unwrap();
        let truncated = &tga[..18.min(tga.len())];
        let trunc_path = dir.join("trunc.tga");
        std::fs::write(&trunc_path, truncated).unwrap();
        let err = convert_image_from(truncated, &trunc_path, ImageTargetFormat::Png).unwrap_err();
        assert_eq!(err, ImageConvertError::InvalidImage);
        assert_eq!(err.to_string(), "无法解码图像");

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn high_contrast_2x2_png() -> Vec<u8> {
        let mut img = RgbImage::new(2, 2);
        img.put_pixel(0, 0, Rgb([0, 0, 0]));
        img.put_pixel(1, 0, Rgb([255, 255, 255]));
        img.put_pixel(0, 1, Rgb([255, 255, 255]));
        img.put_pixel(1, 1, Rgb([0, 0, 0]));
        let mut buf = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn pbm_encodes_p4_magic_and_roundtrips_to_png() {
        let png = high_contrast_2x2_png();
        let pbm = convert_image(&png, ImageTargetFormat::Pbm).unwrap();
        assert_eq!(ImageTargetFormat::Pbm.extension(), "pbm");
        assert!(
            pbm.starts_with(b"P4"),
            "expected P4 magic, got {:?}",
            pbm.get(..pbm.len().min(8))
        );
        assert!(!pbm.starts_with(b"P1"));
        assert!(!pbm.starts_with(b"P5"));
        assert!(!pbm.starts_with(b"P6"));
        assert!(!pbm.starts_with(b"P7"));
        assert_eq!(image::guess_format(&pbm).unwrap(), ImageFormat::Pnm);

        let png_back = convert_image(&pbm, ImageTargetFormat::Png).unwrap();
        assert_eq!(image::guess_format(&png_back).unwrap(), ImageFormat::Png);
        let loaded = image::load_from_memory(&png_back).unwrap().to_rgb8();
        assert_eq!(loaded.dimensions(), (2, 2));

        let hinted =
            convert_image_from(&pbm, Path::new("out.pbm"), ImageTargetFormat::Png).unwrap();
        assert_eq!(image::guess_format(&hinted).unwrap(), ImageFormat::Png);

        let dir = unique_dir("pbm");
        let src = dir.join("src.png");
        std::fs::write(&src, &png).unwrap();
        let batch = crate::image_converter::convert_paths(
            &[src.to_string_lossy().into_owned()],
            ImageTargetFormat::Pbm,
        );
        assert_eq!(batch.succeeded(), 1);
        assert!(batch.successes[0].bytes.starts_with(b"P4"));
        let dest = dir.join("out.pbm");
        crate::image_converter::save_one(&batch.successes[0], &dest).unwrap();
        assert_eq!(dest.extension().and_then(|e| e.to_str()), Some("pbm"));
        let saved = std::fs::read(&dest).unwrap();
        assert!(saved.starts_with(b"P4"));
        assert!(!saved.starts_with(b"P7"));
        let from_file = convert_image_from(&saved, &dest, ImageTargetFormat::Png).unwrap();
        assert_eq!(image::guess_format(&from_file).unwrap(), ImageFormat::Png);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ordinary_ascii_pbm_decodes_to_png() {
        let p1 = b"P1\n2 2\n1 0\n0 1\n";
        let png = convert_image(p1, ImageTargetFormat::Png).unwrap();
        assert_eq!(image::guess_format(&png).unwrap(), ImageFormat::Png);
        let loaded = image::load_from_memory(&png).unwrap().to_rgb8();
        assert_eq!(loaded.dimensions(), (2, 2));
    }
}
