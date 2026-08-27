mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};

pub use helper::{
    decode_jwt, encode_jwt, JwtAlgorithm, JwtDecodeOptions, JwtDecoded, JwtEncodeOptions, JwtError,
};
#[cfg(feature = "gui")]
pub use view::JwtView;

pub const ID: &str = "JsonWebTokenEncoderDecoder";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "JWT",
        search_keywords: &["jwt", "token"],
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
    crate::slot::ToolHandle::open(window, cx, JwtView::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_recommend_from_json() {
        assert!(
            metadata().accepted_types.is_empty(),
            "JWT must not be recommended from JSON clipboard"
        );
        assert_eq!(metadata().id.as_str(), ID);
        assert_eq!(metadata().display_name, "JWT");
        assert_eq!(metadata().group, GroupId::EncodersDecoders);
        assert!(!metadata().search_keywords.contains(&"json"));
        assert!(detectors().is_empty());
    }
}
