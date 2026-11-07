use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::providers::ProviderProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub profiles: Vec<ProviderProfile>,
    pub selected_profile_id: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: Vec::new(),
            selected_profile_id: None,
        }
    }
}

#[derive(Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<AppConfig, String> {
        if !self.path.exists() {
            return Ok(AppConfig::default());
        }
        let raw = fs::read_to_string(&self.path)
            .map_err(|error| format!("failed to read config: {error}"))?;
        match serde_json::from_str(&raw) {
            Ok(config) => Ok(config),
            Err(error) => {
                let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
                let backup = self
                    .path
                    .with_file_name(format!("settings.corrupt-{timestamp}.json"));
                let backup_message = match fs::copy(&self.path, &backup) {
                    Ok(_) => format!(" Corrupt config copied to {}", backup.display()),
                    Err(copy_error) => {
                        format!(" Failed to preserve corrupt config: {copy_error}")
                    }
                };
                eprintln!("Config parse failed: {error}.{backup_message}");
                Ok(AppConfig::default())
            }
        }
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create config directory: {error}"))?;
        }
        let serialized = serde_json::to_vec_pretty(config)
            .map_err(|error| format!("failed to serialize config: {error}"))?;
        fs::write(&self.path, serialized)
            .map_err(|error| format!("failed to write config: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{ProviderType, TrackingMode};

    #[test]
    fn round_trips_profiles_without_secrets() {
        let directory = std::env::temp_dir().join(format!(
            "ai-usage-liquidball-config-test-{}",
            uuid::Uuid::new_v4()
        ));
        let store = ConfigStore::new(directory.join("settings.json"));
        let config = AppConfig {
            version: 1,
            selected_profile_id: Some("profile-1".into()),
            profiles: vec![ProviderProfile {
                id: "profile-1".into(),
                provider_id: "ollama".into(),
                provider_type: ProviderType::Ollama,
                label: "Ollama Personal".into(),
                enabled: true,
                refresh_seconds: 300,
                tracking_mode: TrackingMode::Auto,
                subscription_warning_percent: Some(20.0),
                subscription_critical_percent: Some(5.0),
                full_bottle_amount: None,
                warning_threshold: None,
                critical_threshold: None,
                primary_currency: None,
                team_id: None,
                custom: None,
            }],
        };

        store.save(&config).unwrap();
        let loaded = store.load().unwrap();
        let serialized = serde_json::to_string(&loaded).unwrap();
        assert_eq!(loaded.profiles.len(), 1);
        assert!(serialized.contains("profile-1"));
        assert!(!serialized.to_ascii_lowercase().contains("apikey"));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn corrupt_config_falls_back_and_preserves_backup() {
        let directory = std::env::temp_dir().join(format!(
            "ai-usage-liquidball-corrupt-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(&path, "{broken").unwrap();
        let store = ConfigStore::new(path);

        let loaded = store.load().unwrap();
        assert!(loaded.profiles.is_empty());
        assert_eq!(
            fs::read_dir(&directory)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("settings.corrupt-")
                })
                .count(),
            1
        );

        let _ = fs::remove_dir_all(directory);
    }
}
