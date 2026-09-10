mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{GroupId, ToolId, ToolMetadata, TYPE_FILE, TYPE_TEXT};
pub use helper::{
    checksum_matches, checksum_matches_input, compute_hash, HashAlgorithm, HashError,
};
#[cfg(feature = "gui")]
pub use view::HashChecksumView;

pub const ID: &str = "HashAndChecksumGenerator";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "哈希 / 校验和",
        search_keywords: &["md5", "sha", "hmac", "checksum"],
        group: GroupId::Generators,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_TEXT, TYPE_FILE],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(HashChecksumView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct HashChecksumTool;

impl Tool for HashChecksumTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(HashChecksumTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_checksum_tool_implements_tool() {
        let tool = HashChecksumTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
