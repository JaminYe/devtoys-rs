use crate::groups::GroupId;

pub const JSON_FORMATTER_ID: &str = "JsonFormatter";
pub const SETTINGS_ID: &str = "Settings";

/// Stable English ID. Display names are Chinese and live on [`ToolMetadata`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToolId(pub String);

impl ToolId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ToolId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Static registration record for a GUI tool.
///
/// `searchable` / `favorable` false is how system items (settings) stay out of
/// search and the favorites list. First milestone only registers JSON Formatter
/// as an openable business tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolMetadata {
    pub id: ToolId,
    pub display_name: &'static str,
    pub search_keywords: &'static [&'static str],
    pub group: GroupId,
    pub searchable: bool,
    pub favorable: bool,
    pub accepted_types: &'static [&'static str],
}

impl ToolMetadata {
    /// Returns whether this tool supports compact overlay mode by default.
    /// Visually dense tools (such as Text Compare and Markdown Preview) return false.
    pub fn supports_compact_overlay(&self) -> bool {
        self.id.as_str() != "TextCompare" && self.id.as_str() != "MarkdownPreview"
    }
}
