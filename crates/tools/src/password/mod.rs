mod cli;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
use devtoys_api::{GroupId, ToolId, ToolMetadata};
pub use helper::{generate_password, PasswordError, PasswordOptions};
#[cfg(feature = "gui")]
pub use view::PasswordView;

pub const ID: &str = "PasswordGenerator";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "密码",
        search_keywords: &["password", "密码"],
        group: GroupId::Generators,
        searchable: true,
        favorable: true,
        accepted_types: &[],
    }
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(PasswordView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PasswordTool;

impl Tool for PasswordTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn cli(&self) -> Option<CliTool> {
        Some(cli_tool())
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(PasswordTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_tool_implements_tool() {
        let tool = PasswordTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
