mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};

pub use helper::{convert, decode, encode, Conversion, HtmlError};
pub use cli::cli_tool;
#[cfg(feature = "gui")]
pub use view::HtmlView;

pub const ID: &str = "HtmlEncoderDecoder";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "HTML",
        search_keywords: &["html", "entity", "实体"],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    Vec::new()
}

#[cfg(feature = "gui")]
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, HtmlView::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_detection_not_wired() {
        assert!(metadata().accepted_types.is_empty());
        assert_eq!(metadata().id.as_str(), ID);
        assert_eq!(metadata().display_name, "HTML");
        assert_eq!(metadata().group, GroupId::EncodersDecoders);
        assert!(detectors().is_empty());
    }
}
