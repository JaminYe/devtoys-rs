mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{GroupId, ToolId, ToolMetadata};
pub use helper::{generate_uuid, UuidError, UuidOptions, UuidVersion};
#[cfg(feature = "gui")]
pub use view::UuidGenView;

pub const ID: &str = "UUIDGenerator";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "UUID",
        search_keywords: &["guid", "uuid"],
        group: GroupId::Generators,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(UuidGenView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct UuidGenTool;

impl Tool for UuidGenTool {
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
    Box::new(UuidGenTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_gen_tool_implements_tool() {
        let tool = UuidGenTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
