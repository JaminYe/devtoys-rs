mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
pub use detector::{
    is_static_image_path, split_paths, FileDetector, FilesDetector, ImageDetector,
    StaticImageFileDetector, STATIC_IMAGE_EXTENSIONS, TYPE_STATIC_IMAGE_FILE,
};
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata, TYPE_IMAGE};
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
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(ColorBlindnessView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ColorBlindnessTool;

impl Tool for ColorBlindnessTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn cli(&self) -> Option<CliTool> {
        Some(cli_tool())
    }

    fn detectors(&self) -> Vec<Box<dyn Detector>> {
        vec![Box::new(StaticImageFileDetector)]
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(ColorBlindnessTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_blindness_tool_implements_tool() {
        let tool = ColorBlindnessTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert_eq!(tool.detectors().len(), 1);
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
