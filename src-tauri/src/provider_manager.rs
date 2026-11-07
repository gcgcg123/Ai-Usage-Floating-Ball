use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

use crate::{
    config_store::AppConfig,
    credentials::CredentialStore,
    providers::{
        Health, ProviderProfile, ProviderSnapshot, ProviderStatus, fetch_snapshot,
        missing_credential_snapshot,
    },
    snapshot_store::SnapshotStore,
};

const EVENT_PROVIDER_STATE: &str = "provider://state-updated";
const STALE_AFTER: Duration = Duration::from_secs(15 * 60);
const SUPERVISOR_TICK: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageState {
    pub profiles: Vec<ProviderProfile>,
    pub snapshots: Vec<ProviderSnapshot>,
    pub selected_profile_id: Option<String>,
    pub paused: bool,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct ProviderManager {
    inner: Arc<Inner>,
}

struct Inner {
    app: AppHandle,
    config: Arc<Mutex<AppConfig>>,
    credentials: CredentialStore,
    http: Client,
    snapshots: Mutex<HashMap<String, ProviderSnapshot>>,
    snapshot_store: SnapshotStore,
    failures: Mutex<HashMap<String, u32>>,
    running: Mutex<HashSet<String>>,
    tasks: Mutex<HashMap<String, JoinHandle<()>>>,
    paused: AtomicBool,
}

struct RunningGuard {
    inner: Arc<Inner>,
    profile_id: String,
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        if let Ok(mut running) = self.inner.running.lock() {
            running.remove(&self.profile_id);
        }
    }
}

impl ProviderManager {
    pub fn new(
        app: AppHandle,
        config: Arc<Mutex<AppConfig>>,
        credentials: CredentialStore,
        http: Client,
        snapshot_store: SnapshotStore,
    ) -> Self {
        let snapshots = {
            let config = config.lock().expect("config lock");
            reconcile_snapshots(snapshot_store.load(), &config)
        };
        let _ = snapshot_store.save(&snapshots);
        Self {
            inner: Arc::new(Inner {
                app,
                config,
                credentials,
                http,
                snapshots: Mutex::new(snapshots),
                snapshot_store,
                failures: Mutex::new(HashMap::new()),
                running: Mutex::new(HashSet::new()),
                tasks: Mutex::new(HashMap::new()),
                paused: AtomicBool::new(false),
            }),
        }
    }

    pub fn start(&self) {
        self.reload();
    }

    pub fn reload(&self) {
        let profiles = self.enabled_profiles();
        let enabled_ids: HashSet<String> =
            profiles.iter().map(|profile| profile.id.clone()).collect();

        if let Ok(mut snapshots) = self.inner.snapshots.lock() {
            snapshots.retain(|profile_id, _| enabled_ids.contains(profile_id));
        }
        if let Ok(mut failures) = self.inner.failures.lock() {
            failures.retain(|profile_id, _| enabled_ids.contains(profile_id));
        }

        if let Ok(mut tasks) = self.inner.tasks.lock() {
            for task in tasks.values() {
                task.abort();
            }
            tasks.clear();

            for profile in profiles {
                let profile_id = profile.id.clone();
                let inner = self.inner.clone();
                let task = tauri::async_runtime::spawn(async move {
                    run_profile_loop(inner, profile_id).await;
                });
                tasks.insert(profile.id, task);
            }
        }

        self.persist_and_emit();
    }

    pub fn pause(&self) {
        self.inner.paused.store(true, Ordering::SeqCst);
        self.emit_current_state();
    }

    pub fn resume(&self) {
        self.inner.paused.store(false, Ordering::SeqCst);
        self.emit_current_state();
    }

    pub fn state(&self) -> UsageState {
        build_state(&self.inner)
    }

    pub fn emit_current_state(&self) {
        let state = self.state();
        let _ = self.inner.app.emit(EVENT_PROVIDER_STATE, state);
    }

    pub async fn refresh_profile(&self, profile_id: &str) -> Result<ProviderSnapshot, String> {
        let profile = self
            .profile(profile_id)
            .ok_or_else(|| "Provider profile not found".to_string())?;
        if !profile.enabled {
            return Err("Provider profile is disabled".into());
        }
        Ok(refresh_once(&self.inner, profile).await.0)
    }

    pub async fn refresh_all(&self) -> UsageState {
        let profiles = self.enabled_profiles();
        let mut tasks = Vec::new();
        for profile in profiles {
            let inner = self.inner.clone();
            tasks.push(tauri::async_runtime::spawn(async move {
                let _ = refresh_once(&inner, profile).await;
            }));
        }
        for task in tasks {
            let _ = task.await;
        }
        self.state()
    }

    fn profile(&self, profile_id: &str) -> Option<ProviderProfile> {
        self.inner
            .config
            .lock()
            .ok()?
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .cloned()
    }

    fn enabled_profiles(&self) -> Vec<ProviderProfile> {
        self.inner
            .config
            .lock()
            .map(|config| {
                config
                    .profiles
                    .iter()
                    .filter(|profile| profile.enabled)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn persist_and_emit(&self) {
        if let Ok(snapshots) = self.inner.snapshots.lock() {
            let _ = self.inner.snapshot_store.save(&snapshots);
        }
        self.emit_current_state();
    }
}

async fn run_profile_loop(inner: Arc<Inner>, profile_id: String) {
    let mut next_due = SystemTime::now();

    while let Some(profile) = inner.config.lock().ok().and_then(|config| {
        config
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .cloned()
    }) {
        if !profile.enabled {
            break;
        }

        if inner.paused.load(Ordering::SeqCst) {
            sleep(SUPERVISOR_TICK).await;
            next_due = SystemTime::now();
            continue;
        }

        let now = SystemTime::now();
        if now >= next_due {
            let (_, delay) = refresh_once(&inner, profile).await;
            next_due = SystemTime::now() + delay;
        } else {
            let wait = next_due
                .duration_since(now)
                .unwrap_or_default()
                .min(SUPERVISOR_TICK);
            sleep(wait).await;
        }
    }
}

async fn refresh_once(
    inner: &Arc<Inner>,
    profile: ProviderProfile,
) -> (ProviderSnapshot, Duration) {
    let profile_id = profile.id.clone();
    let already_running = inner
        .running
        .lock()
        .map(|mut running| !running.insert(profile_id.clone()))
        .unwrap_or(false);
    if already_running {
        for _ in 0..200 {
            sleep(Duration::from_millis(100)).await;
            let still_running = inner
                .running
                .lock()
                .map(|running| running.contains(&profile_id))
                .unwrap_or(false);
            if !still_running {
                break;
            }
        }
        let snapshot = inner
            .snapshots
            .lock()
            .ok()
            .and_then(|snapshots| snapshots.get(&profile_id).cloned())
            .unwrap_or_else(|| missing_credential_snapshot(&profile));
        return (
            snapshot,
            Duration::from_secs(profile.refresh_seconds.max(30)),
        );
    }
    let _running_guard = RunningGuard {
        inner: inner.clone(),
        profile_id: profile.id.clone(),
    };

    let snapshot = match inner.credentials.get(&profile.id) {
        Ok(Some(api_key)) => fetch_snapshot(&inner.http, &profile, api_key.trim()).await,
        Ok(None) => missing_credential_snapshot(&profile),
        Err(error) => {
            let mut snapshot = missing_credential_snapshot(&profile);
            if let Some(info) = snapshot.error.as_mut() {
                info.message = format!("無法讀取 Windows Credential Manager：{error}");
                info.action = "請確認 Windows 憑證服務後重新測試。".into();
            }
            snapshot
        }
    };

    let failure_count = {
        let mut failures = inner.failures.lock().expect("failure counter lock");
        if snapshot.last_success_at.is_some() && snapshot.error.is_none() {
            failures.remove(&profile.id);
            0
        } else {
            let count = failures.entry(profile.id.clone()).or_insert(0);
            *count = count.saturating_add(1);
            *count
        }
    };

    let delay = backoff_delay(
        snapshot.status,
        snapshot
            .error
            .as_ref()
            .and_then(|error| error.retry_after_seconds),
        failure_count,
        profile.refresh_seconds.max(30),
    );
    let now = Utc::now();
    let next = now + chrono::Duration::seconds(delay.as_secs() as i64);

    let previous = inner
        .snapshots
        .lock()
        .ok()
        .and_then(|snapshots| snapshots.get(&profile.id).cloned());
    let mut merged = merge_with_last_good(previous.as_ref(), snapshot, now);
    merged.consecutive_failures = failure_count;
    merged.next_refresh_at = Some(next.to_rfc3339());
    merged.retry_at = merged.error.as_ref().map(|_| next.to_rfc3339());
    merged.stale = is_stale(&merged, now);
    let changed = snapshots_differ(previous.as_ref(), &merged);

    if let Ok(mut snapshots) = inner.snapshots.lock() {
        snapshots.insert(profile.id.clone(), merged.clone());
        let _ = inner.snapshot_store.save(&snapshots);
    }
    if changed {
        let state = build_state(inner);
        let _ = inner.app.emit(EVENT_PROVIDER_STATE, state);
    }

    (merged, delay)
}

fn merge_with_last_good(
    previous: Option<&ProviderSnapshot>,
    new_snapshot: ProviderSnapshot,
    now: DateTime<Utc>,
) -> ProviderSnapshot {
    if new_snapshot.last_success_at.is_some() {
        return new_snapshot;
    }

    let Some(previous) = previous else {
        return new_snapshot;
    };
    if previous.last_success_at.is_none() {
        return new_snapshot;
    }
    if previous.provider_id != new_snapshot.provider_id
        || previous.tracking_mode != new_snapshot.tracking_mode
    {
        return new_snapshot;
    }

    let mut merged = previous.clone();
    merged.provider_id = new_snapshot.provider_id;
    merged.profile_id = new_snapshot.profile_id;
    merged.provider_type = new_snapshot.provider_type;
    merged.tracking_mode = new_snapshot.tracking_mode;
    merged.label = new_snapshot.label;
    merged.status = new_snapshot.status;
    merged.fetched_at = now.to_rfc3339();
    merged.error = new_snapshot.error;
    merged.presentation.health = Health::Error;
    merged.stale = true;
    merged
}

fn build_state(inner: &Inner) -> UsageState {
    let config = inner.config.lock().expect("config lock");
    let mut snapshots = inner.snapshots.lock().expect("snapshot lock");
    let now = Utc::now();
    for snapshot in snapshots.values_mut() {
        snapshot.stale = is_stale(snapshot, now);
    }
    let order: HashMap<&str, usize> = config
        .profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| (profile.id.as_str(), index))
        .collect();
    let mut snapshot_values: Vec<_> = snapshots.values().cloned().collect();
    snapshot_values.sort_by_key(|snapshot| {
        order
            .get(snapshot.profile_id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });

    UsageState {
        profiles: config.profiles.clone(),
        snapshots: snapshot_values,
        selected_profile_id: config.selected_profile_id.clone(),
        paused: inner.paused.load(Ordering::SeqCst),
        updated_at: now.to_rfc3339(),
    }
}

fn reconcile_snapshots(
    mut snapshots: HashMap<String, ProviderSnapshot>,
    config: &AppConfig,
) -> HashMap<String, ProviderSnapshot> {
    let expected: HashMap<&str, String> = config
        .profiles
        .iter()
        .map(|profile| (profile.id.as_str(), profile.effective_provider_id()))
        .collect();
    snapshots.retain(|profile_id, snapshot| {
        snapshot.profile_id == *profile_id
            && expected
                .get(profile_id.as_str())
                .is_some_and(|provider_id| provider_id == &snapshot.provider_id)
    });
    snapshots
}

fn is_stale(snapshot: &ProviderSnapshot, now: DateTime<Utc>) -> bool {
    let Some(last_success_at) = snapshot.last_success_at.as_deref() else {
        return true;
    };
    let Ok(last_success_at) = DateTime::parse_from_rfc3339(last_success_at) else {
        return true;
    };
    let elapsed = now.signed_duration_since(last_success_at.with_timezone(&Utc));
    elapsed.num_seconds() >= STALE_AFTER.as_secs() as i64
}

fn snapshots_differ(previous: Option<&ProviderSnapshot>, current: &ProviderSnapshot) -> bool {
    let Some(previous) = previous else {
        return true;
    };

    previous.status != current.status
        || previous.label != current.label
        || previous.tracking_mode != current.tracking_mode
        || previous.fetched_at != current.fetched_at
        || previous.last_success_at != current.last_success_at
        || previous.next_refresh_at != current.next_refresh_at
        || previous.consecutive_failures != current.consecutive_failures
        || previous.windows != current.windows
        || previous.balances != current.balances
        || previous.usage != current.usage
        || previous.presentation != current.presentation
        || previous.error != current.error
        || previous.stale != current.stale
        || previous.retry_at != current.retry_at
}

pub fn backoff_delay(
    status: ProviderStatus,
    retry_after_seconds: Option<u64>,
    failure_count: u32,
    refresh_seconds: u64,
) -> Duration {
    match status {
        ProviderStatus::Ok => Duration::from_secs(refresh_seconds.max(30)),
        ProviderStatus::AuthError
        | ProviderStatus::PermissionError
        | ProviderStatus::SchemaError => Duration::from_secs(30 * 60),
        ProviderStatus::InsufficientBalance => Duration::from_secs(10 * 60),
        ProviderStatus::RateLimited => Duration::from_secs(
            retry_after_seconds
                .unwrap_or_else(|| {
                    (60_u64).saturating_mul(2_u64.saturating_pow(failure_count.min(5)))
                })
                .clamp(30, 30 * 60),
        ),
        ProviderStatus::ProviderError => Duration::from_secs(
            (60_u64)
                .saturating_mul(2_u64.saturating_pow(failure_count.min(4)))
                .min(15 * 60),
        ),
        ProviderStatus::Timeout | ProviderStatus::Offline => Duration::from_secs(30),
        ProviderStatus::Disabled | ProviderStatus::Unknown => {
            Duration::from_secs(refresh_seconds.max(30))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{
        Presentation, PresentationMode, ProviderProfile, ProviderType, TrackingMode, UsageWindow,
        capabilities_for,
    };

    fn snapshot(status: ProviderStatus, last_success_at: Option<String>) -> ProviderSnapshot {
        ProviderSnapshot {
            provider_id: "ollama".into(),
            profile_id: "profile".into(),
            provider_type: ProviderType::Ollama,
            tracking_mode: TrackingMode::Auto,
            label: "Ollama".into(),
            status,
            fetched_at: "2026-11-07T09:45:00Z".into(),
            last_success_at,
            stale: false,
            consecutive_failures: 0,
            retry_at: None,
            next_refresh_at: None,
            capabilities: capabilities_for(ProviderType::Ollama),
            windows: vec![UsageWindow {
                id: "session".into(),
                label: "Session".into(),
                used_percent: Some(20.0),
                remaining_percent: Some(80.0),
                resets_at: None,
                detail: None,
            }],
            balances: vec![],
            usage: None,
            presentation: Presentation {
                mode: PresentationMode::Quota,
                level_percent: 80.0,
                metric_label: "Session".into(),
                metric_value: "80% left".into(),
                health: Health::Safe,
                primary_window_id: Some("session".into()),
            },
            error: None,
        }
    }

    #[test]
    fn auth_errors_use_long_backoff() {
        assert_eq!(
            backoff_delay(ProviderStatus::AuthError, None, 1, 300),
            Duration::from_secs(1800)
        );
    }

    #[test]
    fn retry_after_is_respected_for_rate_limits() {
        assert_eq!(
            backoff_delay(ProviderStatus::RateLimited, Some(12), 1, 300),
            Duration::from_secs(30)
        );
        assert_eq!(
            backoff_delay(ProviderStatus::RateLimited, Some(600), 1, 300),
            Duration::from_secs(600)
        );
    }

    #[test]
    fn failures_grow_with_provider_failures() {
        assert_eq!(
            backoff_delay(ProviderStatus::ProviderError, None, 1, 300),
            Duration::from_secs(120)
        );
        assert_eq!(
            backoff_delay(ProviderStatus::ProviderError, None, 4, 300),
            Duration::from_secs(900)
        );
    }

    #[test]
    fn failed_refresh_preserves_last_good_data() {
        let previous = snapshot(ProviderStatus::Ok, Some("2026-11-07T09:45:00Z".into()));
        let mut failed = snapshot(ProviderStatus::AuthError, None);
        failed.windows.clear();
        failed.error = Some(crate::providers::ProviderErrorInfo {
            code: "authentication_error".into(),
            message: "invalid key".into(),
            action: "replace key".into(),
            retry_after_seconds: None,
        });

        let merged = merge_with_last_good(
            Some(&previous),
            failed,
            DateTime::parse_from_rfc3339("2026-11-07T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(merged.windows.len(), 1);
        assert_eq!(merged.last_success_at, previous.last_success_at);
        assert_eq!(merged.presentation.level_percent, 80.0);
        assert_eq!(merged.presentation.health, Health::Error);
        assert!(merged.stale);
    }

    #[test]
    fn failed_refresh_never_crosses_provider_or_tracking_mode() {
        let previous = snapshot(ProviderStatus::Ok, Some("2026-11-07T09:45:00Z".into()));
        let mut changed = snapshot(ProviderStatus::AuthError, None);
        changed.provider_id = "deepseek".into();
        changed.provider_type = ProviderType::DeepSeek;
        changed.tracking_mode = TrackingMode::Balance;
        changed.windows.clear();

        let merged = merge_with_last_good(
            Some(&previous),
            changed,
            DateTime::parse_from_rfc3339("2026-11-07T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        );
        assert_eq!(merged.provider_id, "deepseek");
        assert!(merged.windows.is_empty());
    }

    #[test]
    fn refresh_timestamp_changes_are_observable() {
        let previous = snapshot(ProviderStatus::Ok, Some("2026-11-07T09:45:00Z".into()));
        let mut current = previous.clone();
        current.fetched_at = "2026-11-07T09:50:00Z".into();
        current.last_success_at = Some("2026-11-07T09:50:00Z".into());

        assert!(snapshots_differ(Some(&previous), &current));
    }

    #[test]
    fn mismatched_snapshot_identity_is_removed() {
        let profile = ProviderProfile {
            id: "profile".into(),
            provider_id: "ollama".into(),
            provider_type: ProviderType::Ollama,
            label: "Ollama".into(),
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
        };
        let config = AppConfig {
            version: 1,
            profiles: vec![profile],
            selected_profile_id: Some("profile".into()),
        };
        let mut wrong = snapshot(ProviderStatus::Ok, Some("2026-11-07T09:45:00Z".into()));
        wrong.provider_id = "deepseek".into();
        let mut snapshots = HashMap::new();
        snapshots.insert("profile".into(), wrong);

        assert!(reconcile_snapshots(snapshots, &config).is_empty());
    }
}
