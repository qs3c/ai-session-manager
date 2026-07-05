use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub device_id: String,
    #[serde(default)]
    pub repo_url: Option<String>,
}

impl AppConfig {
    pub fn load_or_init(config_dir: &Path) -> Result<AppConfig> {
        let path = config_dir.join("config.json");
        if path.exists() {
            let text = fs::read_to_string(&path)?;
            return Ok(serde_json::from_str(&text)?);
        }
        let config = AppConfig {
            device_id: uuid::Uuid::new_v4().to_string(),
            repo_url: None,
        };
        config.save(config_dir)?;
        Ok(config)
    }

    pub fn save(&self, config_dir: &Path) -> Result<()> {
        fs::create_dir_all(config_dir)?;
        fs::write(
            config_dir.join("config.json"),
            serde_json::to_string_pretty(self)?,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_load_generates_device_id_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let c1 = AppConfig::load_or_init(dir.path()).unwrap();
        assert_eq!(c1.device_id.len(), 36);
        assert!(c1.repo_url.is_none());

        let c2 = AppConfig::load_or_init(dir.path()).unwrap();
        assert_eq!(c1.device_id, c2.device_id);
    }

    #[test]
    fn save_and_reload_repo_url() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = AppConfig::load_or_init(dir.path()).unwrap();
        c.repo_url = Some("git@github.com:me/ai-sessions.git".into());
        c.save(dir.path()).unwrap();
        let back = AppConfig::load_or_init(dir.path()).unwrap();
        assert_eq!(
            back.repo_url.as_deref(),
            Some("git@github.com:me/ai-sessions.git")
        );
    }
}
