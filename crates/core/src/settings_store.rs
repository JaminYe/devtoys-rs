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

    /// Serialize first, write `settings.json.tmp`, then rename over the dest so a
    /// failed write cannot truncate a previously valid file.
    pub fn save(&self, settings: &AppSettings) -> Result<(), CoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(settings)?;
        let tmp = sibling(&self.path, ".tmp");
        std::fs::write(&tmp, json)?;
        if let Err(e) = replace_file(&tmp, &self.path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
        Ok(())
    }
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = path.as_os_str().to_os_string();
    raw.push(suffix);
    PathBuf::from(raw)
}

fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) if to.exists() => {
            let bak = sibling(to, ".bak");
            std::fs::rename(to, &bak)?;
            match std::fs::rename(from, to) {
                Ok(()) => {
                    let _ = std::fs::remove_file(&bak);
                    Ok(())
                }
                Err(e) => {
                    let _ = std::fs::rename(&bak, to);
                    Err(e)
                }
            }
        }
        Err(e) => Err(e),
    }
}
