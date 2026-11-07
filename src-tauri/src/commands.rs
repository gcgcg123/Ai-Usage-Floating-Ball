use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    provider_manager::UsageState,
    providers::{
        ProviderDefinition, ProviderProfile, ProviderSnapshot, ProviderType, fetch_snapshot,
        missing_credential_snapshot, provider_definition, provider_definitions,
    },
};

fn move_profile_entry(profiles: &mut [ProviderProfile], profile_id: &str, direction: i32) -> bool {
    let Some(index) = profiles.iter().position(|profile| profile.id == profile_id) else {
        return false;
    };
    let target = if direction < 0 {
        index.saturating_sub(1)
    } else {
        (index + 1).min(profiles.len().saturating_sub(1))
    };
    profiles.swap(index, target);
    true
}

#[tauri::command]
pub fn provider_catalog() -> Vec<ProviderDefinition> {
    provider_definitions()
}

#[tauri::command]
pub fn list_provider_profiles(state: State<'_, AppState>) -> Result<Vec<ProviderProfile>, String> {
    state
        .config
        .lock()
        .map_err(|_| "provider configuration lock is poisoned".to_string())
        .map(|config| config.profiles.clone())
}

#[tauri::command]
pub fn get_provider_profile(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ProviderProfile>, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "provider configuration lock is poisoned".to_string())?;
    Ok(config
        .profiles
        .iter()
        .find(|profile| profile.id == profile_id)
        .cloned())
}

#[tauri::command]
pub fn save_provider_profile(
    mut profile: ProviderProfile,
    api_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<ProviderProfile, String> {
    profile.label = profile.label.trim().to_string();
    if profile.label.is_empty() {
        return Err("Display name cannot be empty".into());
    }

    if profile.provider_id.trim().is_empty() {
        profile.provider_id = profile.provider_type.as_str().to_string();
    }
    let definition =
        provider_definition(&profile.provider_id).ok_or_else(|| "Unknown provider".to_string())?;
    profile.provider_type = definition.adapter;
    if definition.requires_team_id
        && profile
            .team_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err("Team ID is required for this provider".into());
    }

    profile.refresh_seconds = match profile.provider_type {
        ProviderType::Custom => profile.refresh_seconds.max(60),
        _ => profile.refresh_seconds.max(300),
    };
    let mut warning = profile
        .subscription_warning_percent
        .unwrap_or(20.0)
        .clamp(0.0, 100.0);
    let mut critical = profile
        .subscription_critical_percent
        .unwrap_or(5.0)
        .clamp(0.0, 100.0);
    if critical > warning {
        std::mem::swap(&mut warning, &mut critical);
    }
    profile.subscription_warning_percent = Some(warning);
    profile.subscription_critical_percent = Some(critical);
    profile.primary_currency = profile
        .primary_currency
        .map(|currency| currency.trim().to_ascii_uppercase())
        .filter(|currency| !currency.is_empty());

    if profile.id.trim().is_empty() {
        profile.id = Uuid::new_v4().to_string();
    }

    let mut config = state
        .config
        .lock()
        .map_err(|_| "provider configuration lock is poisoned".to_string())?;
    let existing_index = config
        .profiles
        .iter()
        .position(|existing| existing.id == profile.id);

    if existing_index.is_none() && api_key.as_deref().unwrap_or_default().trim().is_empty() {
        return Err("API Key is required for a new provider".into());
    }

    if let Some(api_key) = api_key.filter(|key| !key.trim().is_empty()) {
        state.credentials.set(&profile.id, api_key.trim())?;
    }

    if let Some(index) = existing_index {
        config.profiles[index] = profile.clone();
    } else {
        config.profiles.push(profile.clone());
    }
    if config.selected_profile_id.is_none() {
        config.selected_profile_id = Some(profile.id.clone());
    }
    state.config_store.save(&config)?;
    drop(config);
    state.manager.reload();
    Ok(profile)
}

#[tauri::command]
pub fn delete_provider_profile(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state
        .config
        .lock()
        .map_err(|_| "provider configuration lock is poisoned".to_string())?;
    let before = config.profiles.len();
    config.profiles.retain(|profile| profile.id != profile_id);
    if config.profiles.len() == before {
        return Err("Provider profile not found".into());
    }
    if config.selected_profile_id.as_deref() == Some(profile_id.as_str()) {
        config.selected_profile_id = config.profiles.first().map(|profile| profile.id.clone());
    }
    state.config_store.save(&config)?;
    drop(config);
    state.manager.reload();
    state.credentials.delete(&profile_id)
}

#[tauri::command]
pub fn select_provider_profile(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state
        .config
        .lock()
        .map_err(|_| "provider configuration lock is poisoned".to_string())?;
    if !config
        .profiles
        .iter()
        .any(|profile| profile.id == profile_id)
    {
        return Err("Provider profile not found".into());
    }
    config.selected_profile_id = Some(profile_id);
    state.config_store.save(&config)?;
    drop(config);
    state.manager.emit_current_state();
    Ok(())
}

#[tauri::command]
pub fn move_provider_profile(
    profile_id: String,
    direction: i32,
    state: State<'_, AppState>,
) -> Result<Vec<ProviderProfile>, String> {
    if direction == 0 {
        return Ok(state.manager.state().profiles);
    }
    let mut config = state
        .config
        .lock()
        .map_err(|_| "provider configuration lock is poisoned".to_string())?;
    if !move_profile_entry(&mut config.profiles, &profile_id, direction) {
        return Err("Provider profile not found".into());
    }
    state.config_store.save(&config)?;
    let profiles = config.profiles.clone();
    drop(config);
    state.manager.emit_current_state();
    Ok(profiles)
}

#[tauri::command]
pub fn provider_has_credential(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    Ok(state.credentials.get(&profile_id)?.is_some())
}

#[tauri::command]
pub async fn test_provider_profile(
    profile: ProviderProfile,
    api_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<ProviderSnapshot, String> {
    let key = match api_key.filter(|key| !key.trim().is_empty()) {
        Some(key) => Some(key),
        None => state.credentials.get(&profile.id)?,
    };
    let Some(key) = key else {
        return Ok(missing_credential_snapshot(&profile));
    };
    Ok(fetch_snapshot(&state.http, &profile, key.trim()).await)
}

#[tauri::command]
pub async fn refresh_provider_profile(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<ProviderSnapshot, String> {
    state.manager.refresh_profile(&profile_id).await
}

#[tauri::command]
pub fn get_usage_state(state: State<'_, AppState>) -> UsageState {
    state.manager.state()
}

#[tauri::command]
pub async fn refresh_all_providers(state: State<'_, AppState>) -> Result<UsageState, String> {
    Ok(state.manager.refresh_all().await)
}

#[tauri::command]
pub fn pause_monitoring(state: State<'_, AppState>) {
    state.manager.pause();
}

#[tauri::command]
pub fn resume_monitoring(state: State<'_, AppState>) {
    state.manager.resume();
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("index.html".into()))
        .title("AI Usage Floating Ball Settings")
        .inner_size(780.0, 660.0)
        .min_inner_size(680.0, 540.0)
        .center()
        .build()
        .map_err(|error| format!("failed to open settings window: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|error| error.to_string())?;
    } else {
        manager.disable().map_err(|error| error.to_string())?;
    }
    manager.is_enabled().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::move_profile_entry;
    use crate::providers::{ProviderProfile, ProviderType, TrackingMode};

    fn profile(id: &str) -> ProviderProfile {
        ProviderProfile {
            id: id.to_string(),
            provider_id: "ollama".to_string(),
            provider_type: ProviderType::Ollama,
            label: id.to_string(),
            enabled: true,
            refresh_seconds: 300,
            tracking_mode: TrackingMode::Auto,
            subscription_warning_percent: Some(20.0),
            subscription_critical_percent: Some(5.0),
            full_bottle_amount: None,
            warning_threshold: Some(20.0),
            critical_threshold: Some(5.0),
            primary_currency: None,
            team_id: None,
            custom: None,
        }
    }

    #[test]
    fn move_swaps_and_clamps_at_edges() {
        let mut profiles = vec![profile("a"), profile("b"), profile("c")];

        assert!(move_profile_entry(&mut profiles, "b", -1));
        assert_eq!(profiles[0].id, "b");
        assert_eq!(profiles[1].id, "a");

        assert!(move_profile_entry(&mut profiles, "a", -1));
        assert_eq!(profiles[0].id, "a");

        assert!(move_profile_entry(&mut profiles, "b", 1));
        assert_eq!(profiles[2].id, "b");

        assert!(!move_profile_entry(&mut profiles, "missing", 1));
    }
}
