mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_IMAGE};

pub use cli::cli_tool;
pub use detector::{
    is_static_image_path, split_paths, FileDetector, FilesDetector, ImageDetector,
    StaticImageFileDetector, STATIC_IMAGE_EXTENSIONS, TYPE_STATIC_IMAGE_FILE,
};
pub use helper::{simulate_color_blindness, ColorBlindnessError, SimulatedImages};
#[cfg(feature = "gui")]
pub use view::ColorBlindnessView;

pub const ID: &str = "ColorBlindnessSimulator";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "色盲模拟器",
        search_keywords: &["color", "色盲", "protanopia"],
        group: GroupId::Graphic,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_IMAGE, TYPE_STATIC_IMAGE_FILE],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(FilesDetector),
        Box::new(FileDetector),
        Box::new(ImageDetector),
        Box::new(StaticImageFileDetector),
    ]
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, ColorBlindnessView::new)
}
