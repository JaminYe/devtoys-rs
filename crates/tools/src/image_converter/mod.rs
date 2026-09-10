mod detector;
mod execute;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_IMAGE};

pub use detector::{
    is_static_image_path, StaticImageFileDetector, StaticImageFilesDetector,
    TYPE_STATIC_IMAGE_FILE, TYPE_STATIC_IMAGE_FILES,
};
pub use execute::{
    convert_memory, convert_paths, parse_paths, save_batch, save_one, ConversionBatch,
    FailedConversion, SaveReport, SaveStatus, SavedItem, SuccessfulConversion,
};
pub use helper::{
    convert_image, convert_image_from, convert_image_named, preview_rgba, static_images_in_dir,
    ImageConvertError, ImageTargetFormat,
};
#[cfg(feature = "gui")]
pub use view::ImageConverterView;

pub const ID: &str = "ImageConverter";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "图片格式转换器",
        search_keywords: &["png", "jpeg", "webp", "bmp"],
        group: GroupId::Graphic,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_IMAGE, TYPE_STATIC_IMAGE_FILE, TYPE_STATIC_IMAGE_FILES],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(StaticImageFileDetector),
        Box::new(StaticImageFilesDetector),
    ]
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(ImageConverterView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ImageConverterTool;

impl Tool for ImageConverterTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn detectors(&self) -> Vec<Box<dyn Detector>> {
        detectors()
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(ImageConverterTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_converter_tool_implements_tool() {
        let tool = ImageConverterTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(!tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
