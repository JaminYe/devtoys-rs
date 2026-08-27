mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_IMAGE};

use crate::color_blindness::TYPE_STATIC_IMAGE_FILE;

pub use cli::cli_tool;
pub use detector::{StaticImageFilesDetector, TYPE_STATIC_IMAGE_FILES};
pub use helper::{
    convert_image, convert_image_named, static_images_in_dir, ImageConvertError, ImageTargetFormat,
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
    vec![Box::new(StaticImageFilesDetector)]
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, ImageConverterView::new)
}
