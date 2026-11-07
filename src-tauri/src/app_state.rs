use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use reqwest::Client;
use tauri::{AppHandle, Manager};

use crate::{
    config_store::{AppConfig, ConfigStore},
    credentials::CredentialStore,
    provider_manager::ProviderManager,
    snapshot_store::SnapshotStore,
};

pub struct AppState {
    pub config: Arc<Mutex<AppConfig>>,
    pub config_store: ConfigStore,
    pub credentials: CredentialStore,
    pub http: Client,
    pub manager: ProviderManager,
}

impl AppState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
        let config_store = ConfigStore::new(app_data_dir.join("settings.json"));
        let config = Arc::new(Mutex::new(config_store.load()?));
        let http = Client::builder()
            .user_agent("AI-Usage-Floating-Ball/0.5")
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|error| format!("failed to create HTTP client: {error}"))?;
        let snapshot_store = SnapshotStore::new(app_data_dir.join("snapshots.json"));
        let credentials = CredentialStore;
        let manager = ProviderManager::new(
            app.clone(),
            config.clone(),
            credentials.clone(),
            http.clone(),
            snapshot_store,
        );

        Ok(Self {
            config,
            config_store,
            credentials,
            http,
            manager,
        })
    }
}
