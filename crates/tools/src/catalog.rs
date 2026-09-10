use std::sync::LazyLock;

#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
use devtoys_api::{Detector, ToolMetadata};

/// Unified tool trait encapsulating metadata, CLI, detectors, and GUI view factory.
pub trait Tool: Send + Sync {
    /// Returns the static metadata for this tool.
    fn metadata(&self) -> ToolMetadata;

    /// Returns smart detectors provided by this tool, if any.
    fn detectors(&self) -> Vec<Box<dyn Detector>> {
        Vec::new()
    }

    /// Creates a GUI view instance for this tool, if supported.
    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        None
    }
}

static DEFAULT_CATALOG: LazyLock<ToolCatalog> = LazyLock::new(build_default_catalog);

/// Returns the global shared default [`ToolCatalog`] containing all 23 business tools.
pub fn default_catalog() -> &'static ToolCatalog {
    &DEFAULT_CATALOG
}

/// Catalog storing tool instances implementing [`Tool`].
pub struct ToolCatalog {
    tools: Vec<Box<dyn Tool>>,
}

// Safety: Every tool registered in ToolCatalog implements Tool, which requires Send + Sync.
unsafe impl Send for ToolCatalog {}
unsafe impl Sync for ToolCatalog {}

impl ToolCatalog {
    /// Returns the global shared default [`ToolCatalog`].
    pub fn default_catalog() -> &'static Self {
        default_catalog()
    }

    /// Creates a catalog from a vector of boxed tools.
    pub fn new(tools: Vec<Box<dyn Tool>>) -> Self {
        Self { tools }
    }

    /// Creates an empty catalog.
    pub fn empty() -> Self {
        Self { tools: Vec::new() }
    }

    /// Registers a tool into the catalog.
    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.tools.push(Box::new(tool));
    }

    /// Registers a boxed tool into the catalog.
    pub fn register_boxed(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Returns a slice of all registered tools.
    pub fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    /// Returns number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns true if no tools are registered.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Gathers metadata for all registered tools.
    pub fn all_metadata(&self) -> Vec<ToolMetadata> {
        self.tools.iter().map(|tool| tool.metadata()).collect()
    }

    /// Gathers all smart detectors from registered tools.
    pub fn all_detectors(&self) -> Vec<Box<dyn Detector>> {
        self.tools
            .iter()
            .flat_map(|tool| tool.detectors())
            .collect()
    }

    /// Opens a GUI view for a tool by its unique ID.
    #[cfg(feature = "gui")]
    pub fn open_view(&self, id: &str) -> Option<ToolHandle> {
        for tool in &self.tools {
            if tool.metadata().id.as_str() == id {
                return tool.create_view();
            }
        }
        None
    }
}

impl Default for ToolCatalog {
    fn default() -> Self {
        build_default_catalog()
    }
}

/// Builds the default [`ToolCatalog`] containing all 23 business tools.
fn build_default_catalog() -> ToolCatalog {
    let mut catalog = ToolCatalog::empty();

    catalog.register(crate::cron_parser::CronParserTool);
    catalog.register(crate::date_converter::DateConverterTool);
    catalog.register(crate::json_table::JsonTableTool);
    catalog.register(crate::json_yaml::JsonYamlTool);
    catalog.register(crate::number_base::NumberBaseTool);
    catalog.register(crate::base64_text::Base64TextTool);
    catalog.register(crate::base64_image::Base64ImageTool);
    catalog.register(crate::jwt::JwtTool);
    catalog.register(crate::url::UrlTool);
    catalog.register(crate::json_formatter::JsonFormatterTool);
    catalog.register(crate::sql_formatter::SqlFormatterTool);
    catalog.register(crate::xml_formatter::XmlFormatterTool);
    catalog.register(crate::hash_checksum::HashChecksumTool);
    catalog.register(crate::password::PasswordTool);
    catalog.register(crate::uuid_gen::UuidGenTool);
    catalog.register(crate::image_converter::ImageConverterTool);
    catalog.register(crate::jsonpath::JsonpathTool);
    catalog.register(crate::regex_tester::RegexTesterTool);
    catalog.register(crate::text_analyzer::TextAnalyzerTool);
    catalog.register(crate::text_compare::TextCompareTool);
    catalog.register(crate::escape_unescape::EscapeUnescapeTool);
    catalog.register(crate::list_compare::ListCompareTool);
    catalog.register(crate::markdown_preview::MarkdownPreviewTool);

    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use devtoys_api::{GroupId, ToolId};

    struct DummyTool;
    impl Tool for DummyTool {
        fn metadata(&self) -> ToolMetadata {
            ToolMetadata {
                id: ToolId::new("Dummy"),
                display_name: "Dummy Tool",
                search_keywords: &[],
                group: GroupId::Generators,
                searchable: true,
                favorable: false,
                accepted_types: &[],
            }
        }
    }

    #[test]
    fn default_catalog_has_expected_tools() {
        let catalog = default_catalog();
        assert_eq!(catalog.len(), 23);
        assert_eq!(catalog.tools().len(), 23);
        assert_eq!(catalog.all_metadata().len(), 23);
        assert_eq!(ToolCatalog::default_catalog().len(), 23);
    }

    #[test]
    fn custom_tool_registration() {
        let mut catalog = ToolCatalog::empty();
        catalog.register(DummyTool);
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.all_metadata()[0].id.as_str(), "Dummy");
    }

    #[test]
    fn default_catalog_has_detectors() {
        let catalog = default_catalog();
        assert!(!catalog.all_detectors().is_empty());
    }

    #[cfg(feature = "gui")]
    #[test]
    fn default_catalog_opens_all_views() {
        let catalog = default_catalog();
        for tool in catalog.tools() {
            let id = tool.metadata().id;
            assert!(
                catalog.open_view(id.as_str()).is_some(),
                "Failed to open view for tool {}",
                id.as_str()
            );
        }
        assert!(catalog.open_view("NonExistentToolId").is_none());
    }

    #[test]
    fn all_tools_registered_natively() {
        let catalog = default_catalog();
        assert_eq!(catalog.len(), 23);
        for tool in catalog.tools() {
            let meta = tool.metadata();
            assert!(!meta.id.as_str().is_empty());
            assert!(!meta.display_name.is_empty());
        }
    }
}
