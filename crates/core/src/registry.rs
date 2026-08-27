use devtoys_api::{GroupId, ToolMetadata};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchOutcome<'a> {
    Idle,
    Empty,
    Hits(Vec<&'a ToolMetadata>),
}

#[derive(Clone, Debug)]
pub struct ToolRegistry {
    tools: Vec<ToolMetadata>,
}

impl ToolRegistry {
    pub fn new(tools: Vec<ToolMetadata>) -> Self {
        Self { tools }
    }

    pub fn get(&self, id: &str) -> Option<&ToolMetadata> {
        self.tools.iter().find(|tool| tool.id.as_str() == id)
    }

    pub fn all(&self) -> &[ToolMetadata] {
        &self.tools
    }

    pub fn in_group(&self, group: GroupId) -> Vec<&ToolMetadata> {
        self.tools
            .iter()
            .filter(|tool| tool.group == group)
            .collect()
    }

    pub fn search(&self, query: &str) -> SearchOutcome<'_> {
        let query = query.trim();
        if query.is_empty() {
            return SearchOutcome::Idle;
        }
        let needle = query.to_lowercase();
        let hits: Vec<&ToolMetadata> = self
            .tools
            .iter()
            .filter(|tool| tool.searchable && tool_matches(tool, &needle))
            .collect();
        if hits.is_empty() {
            SearchOutcome::Empty
        } else {
            SearchOutcome::Hits(hits)
        }
    }
}

fn tool_matches(tool: &ToolMetadata, needle: &str) -> bool {
    tool.display_name.to_lowercase().contains(needle)
        || tool.id.as_str().to_lowercase().contains(needle)
        || tool
            .search_keywords
            .iter()
            .any(|keyword| keyword.to_lowercase().contains(needle))
}
