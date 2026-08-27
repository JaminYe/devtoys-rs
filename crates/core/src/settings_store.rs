use std::path::{Path, PathBuf};

use devtoys_api::AppSettings;

use crate::CoreError;

#[derive(Clone, Debug)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn user() -> Self {
        let dir = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("devtoys-rs");
        Self::in_dir(dir)
    }

    pub fn in_dir(dir: impl AsRef<Path>) -> Self {
        Self {
            path: dir.as_ref().join("settings.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> AppSettings {
        let Ok(bytes) = std::fs::read(&self.path) else {
            return AppSettings::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), CoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(settings)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}
