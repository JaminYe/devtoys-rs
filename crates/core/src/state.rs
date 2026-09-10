use devtoys_api::{AppSettings, ThemePreference, ToolMetadata};

use crate::{CoreError, SettingsStore, ToolRegistry};

pub struct AppState {
    settings: AppSettings,
    registry: ToolRegistry,
    store: SettingsStore,
}

impl AppState {
    pub fn bootstrap(tools: Vec<ToolMetadata>, store: SettingsStore) -> Self {
        let settings = store.load();
        Self {
            registry: ToolRegistry::new(tools),
            settings,
            store,
        }
    }

    pub fn settings(&self) -> &AppSettings {
        &self.settings
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn persist(&self) -> Result<(), CoreError> {
        self.store.save(&self.settings)
    }

    pub fn toggle_favorite(&mut self, id: &str) -> Result<(), CoreError> {
        let favorable = self
            .registry
            .get(id)
            .ok_or_else(|| CoreError::UnknownTool(id.to_string()))?
            .favorable;
        if !favorable {
            return Err(CoreError::NotFavorable(id.to_string()));
        }
        if let Some(pos) = self.settings.favorites.iter().position(|item| item == id) {
            self.settings.favorites.remove(pos);
        } else {
            self.settings.favorites.push(id.to_string());
        }
        self.persist()
    }

    pub fn is_favorite(&self, id: &str) -> bool {
        self.settings.favorites.iter().any(|item| item == id)
    }

    pub fn favorite_tools(&self) -> Vec<&ToolMetadata> {
        self.settings
            .favorites
            .iter()
            .filter_map(|id| self.registry.get(id))
            .collect()
    }

    pub fn open_tool(&mut self, _id: &str) -> Result<(), CoreError> {
        Ok(())
    }

    pub fn set_theme(&mut self, theme: ThemePreference) -> Result<(), CoreError> {
        self.settings.theme = theme;
        self.persist()
    }

    pub fn set_smart_detection_enabled(&mut self, enabled: bool) -> Result<(), CoreError> {
        self.settings.smart_detection_enabled = enabled;
        self.persist()
    }

    pub fn set_smart_detection_paste(&mut self, enabled: bool) -> Result<(), CoreError> {
        self.settings.smart_detection_paste = enabled;
        self.persist()
    }

    pub fn set_tool_options(
        &mut self,
        tool_id: impl Into<String>,
        value: serde_json::Value,
    ) -> Result<(), CoreError> {
        self.settings.set_tool_options(tool_id, value);
        self.persist()
    }
}
