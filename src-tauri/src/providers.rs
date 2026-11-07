use chrono::Utc;
use reqwest::{
    Client, StatusCode,
    header::{ACCEPT, AUTHORIZATION, CACHE_CONTROL, PRAGMA, RETRY_AFTER},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

const OLLAMA_BALANCE_URL: &str = "https://ollama.com/api/balance";
const DEEPSEEK_BALANCE_URL: &str = "https://api.deepseek.com/user/balance";
const OPENROUTER_KEY_URL: &str = "https://openrouter.ai/api/v1/key";
const OPENROUTER_CREDITS_URL: &str = "https://openrouter.ai/api/v1/credits";
const MOONSHOT_GLOBAL_BALANCE_URL: &str = "https://api.moonshot.ai/v1/users/me/balance";
const MOONSHOT_CN_BALANCE_URL: &str = "https://api.moonshot.cn/v1/users/me/balance";
const XAI_PREPAID_BALANCE_URL: &str =
    "https://management-api.x.ai/v1/billing/teams/{team_id}/prepaid/balance";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderType {
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "deepseek")]
    DeepSeek,
    #[serde(rename = "openrouter")]
    OpenRouter,
    #[serde(rename = "moonshot")]
    Moonshot,
    #[serde(rename = "xai_management")]
    XaiManagement,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackingMode {
    #[default]
    Auto,
    Subscription,
    Balance,
}

impl ProviderType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::DeepSeek => "deepseek",
            Self::OpenRouter => "openrouter",
            Self::Moonshot => "moonshot",
            Self::XaiManagement => "xai_management",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSupport {
    Native,
    ManagementKey,
    InferenceUsageOnly,
    CustomOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    Quota,
    Balance,
    KeyLimit,
    UsageOnly,
    Hybrid,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Ok,
    AuthError,
    PermissionError,
    RateLimited,
    InsufficientBalance,
    ProviderError,
    SchemaError,
    Offline,
    Timeout,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Health {
    Safe,
    Warning,
    Critical,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub balance: bool,
    pub quota: bool,
    pub reset_time: bool,
    pub per_key_limit: bool,
    pub monthly_credits: bool,
    pub multi_currency: bool,
    pub usage_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub id: String,
    pub label: String,
    pub used_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub resets_at: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceEntry {
    pub amount: f64,
    pub currency: String,
    pub total: Option<f64>,
    pub granted: Option<f64>,
    pub topped_up: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyUsage {
    pub daily: Option<f64>,
    pub weekly: Option<f64>,
    pub monthly: Option<f64>,
    pub all_time: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Presentation {
    pub mode: PresentationMode,
    pub level_percent: f64,
    pub metric_label: String,
    pub metric_value: String,
    pub health: Health,
    pub primary_window_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub key_url: &'static str,
    pub icon: &'static str,
    pub brand_color: &'static str,
    pub adapter: ProviderType,
    pub auth_label: &'static str,
    pub support: ProviderSupport,
    pub default_refresh_seconds: u64,
    pub requires_team_id: bool,
    pub capabilities: ProviderCapabilities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderErrorInfo {
    pub code: String,
    pub message: String,
    pub action: String,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub provider_id: String,
    pub profile_id: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    #[serde(default)]
    pub tracking_mode: TrackingMode,
    pub label: String,
    pub status: ProviderStatus,
    pub fetched_at: String,
    pub last_success_at: Option<String>,
    #[serde(default)]
    pub stale: bool,
    #[serde(default)]
    pub consecutive_failures: u32,
    pub retry_at: Option<String>,
    pub next_refresh_at: Option<String>,
    pub capabilities: ProviderCapabilities,
    pub windows: Vec<UsageWindow>,
    pub balances: Vec<BalanceEntry>,
    pub usage: Option<KeyUsage>,
    pub presentation: Presentation,
    pub error: Option<ProviderErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderConfig {
    pub url: Option<String>,
    pub auth_header: Option<String>,
    pub auth_scheme: Option<String>,
    pub remaining_path: Option<String>,
    pub limit_path: Option<String>,
    pub used_path: Option<String>,
    pub currency_path: Option<String>,
    pub resets_at_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    #[serde(default)]
    pub provider_id: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    pub label: String,
    pub enabled: bool,
    pub refresh_seconds: u64,
    #[serde(default)]
    pub tracking_mode: TrackingMode,
    #[serde(default)]
    pub subscription_warning_percent: Option<f64>,
    #[serde(default)]
    pub subscription_critical_percent: Option<f64>,
    pub full_bottle_amount: Option<f64>,
    pub warning_threshold: Option<f64>,
    pub critical_threshold: Option<f64>,
    pub primary_currency: Option<String>,
    #[serde(default)]
    pub team_id: Option<String>,
    pub custom: Option<CustomProviderConfig>,
}

impl ProviderProfile {
    pub fn effective_provider_id(&self) -> String {
        if self.provider_id.trim().is_empty() {
            self.provider_type.as_str().to_string()
        } else {
            self.provider_id.clone()
        }
    }
}

#[derive(Debug, Error)]
enum AdapterError {
    #[error("authentication failed")]
    Authentication,
    #[error("permission denied")]
    Permission,
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("rate limited")]
    RateLimited { retry_after_seconds: Option<u64> },
    #[error("timeout")]
    Timeout,
    #[error("offline")]
    Offline,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("schema error: {0}")]
    Schema(String),
}

impl AdapterError {
    fn to_info(&self) -> ProviderErrorInfo {
        match self {
            Self::Authentication => ProviderErrorInfo {
                code: "authentication_error".into(),
                message: "API Key 無效或已失效。".into(),
                action: "請在 Settings 重新輸入並測試 API Key。".into(),
                retry_after_seconds: None,
            },
            Self::Permission => ProviderErrorInfo {
                code: "permission_error".into(),
                message: "目前的 API Key 沒有讀取此資料的權限。".into(),
                action: "請建立具有餘額／用量讀取權限的 Key。".into(),
                retry_after_seconds: None,
            },
            Self::InsufficientBalance => ProviderErrorInfo {
                code: "insufficient_balance".into(),
                message: "帳戶餘額不足。".into(),
                action: "請開啟供應商網站充值或降低用量。".into(),
                retry_after_seconds: None,
            },
            Self::RateLimited {
                retry_after_seconds,
            } => ProviderErrorInfo {
                code: "rate_limited".into(),
                message: "供應商暫時限制查詢頻率。".into(),
                action: "App 會自動退避重試，也可以稍後手動重新整理。".into(),
                retry_after_seconds: *retry_after_seconds,
            },
            Self::Timeout => ProviderErrorInfo {
                code: "timeout".into(),
                message: "連線逾時。".into(),
                action: "請檢查網路後按 Retry；其他供應商仍會繼續更新。".into(),
                retry_after_seconds: None,
            },
            Self::Offline => ProviderErrorInfo {
                code: "offline".into(),
                message: "目前無法連線到供應商。".into(),
                action: "恢復網路後會自動重試，期間保留上次成功資料。".into(),
                retry_after_seconds: None,
            },
            Self::Provider(message) => ProviderErrorInfo {
                code: "provider_error".into(),
                message: message.clone(),
                action: "供應商服務暫時異常，請稍後重試。".into(),
                retry_after_seconds: None,
            },
            Self::Schema(message) => ProviderErrorInfo {
                code: "schema_error".into(),
                message: format!("供應商回傳格式無法解析：{message}"),
                action: "請保留上次資料並檢查是否有 App／Adapter 更新。".into(),
                retry_after_seconds: None,
            },
        }
    }

    fn status(&self) -> ProviderStatus {
        match self {
            Self::Authentication => ProviderStatus::AuthError,
            Self::Permission => ProviderStatus::PermissionError,
            Self::InsufficientBalance => ProviderStatus::InsufficientBalance,
            Self::RateLimited { .. } => ProviderStatus::RateLimited,
            Self::Timeout => ProviderStatus::Timeout,
            Self::Offline => ProviderStatus::Offline,
            Self::Provider(_) => ProviderStatus::ProviderError,
            Self::Schema(_) => ProviderStatus::SchemaError,
        }
    }
}

pub fn capabilities_for(provider_type: ProviderType) -> ProviderCapabilities {
    match provider_type {
        ProviderType::Ollama => ProviderCapabilities {
            balance: true,
            quota: true,
            reset_time: true,
            per_key_limit: false,
            monthly_credits: true,
            multi_currency: false,
            usage_only: false,
        },
        ProviderType::DeepSeek => ProviderCapabilities {
            balance: true,
            quota: false,
            reset_time: false,
            per_key_limit: false,
            monthly_credits: false,
            multi_currency: true,
            usage_only: false,
        },
        ProviderType::OpenRouter => ProviderCapabilities {
            balance: true,
            quota: true,
            reset_time: true,
            per_key_limit: true,
            monthly_credits: true,
            multi_currency: false,
            usage_only: false,
        },
        ProviderType::Moonshot => ProviderCapabilities {
            balance: true,
            quota: false,
            reset_time: false,
            per_key_limit: false,
            monthly_credits: false,
            multi_currency: false,
            usage_only: false,
        },
        ProviderType::XaiManagement => ProviderCapabilities {
            balance: true,
            quota: false,
            reset_time: false,
            per_key_limit: false,
            monthly_credits: false,
            multi_currency: false,
            usage_only: false,
        },
        ProviderType::Custom => ProviderCapabilities {
            balance: true,
            quota: true,
            reset_time: true,
            per_key_limit: false,
            monthly_credits: false,
            multi_currency: true,
            usage_only: true,
        },
    }
}

pub fn provider_definitions() -> Vec<ProviderDefinition> {
    vec![
        ProviderDefinition {
            id: "ollama",
            label: "Ollama",
            description: "Session、Weekly、Monthly credits 與 credit balance",
            key_url: "https://ollama.com/settings/keys",
            icon: "/providers/ollama.png",
            brand_color: "#55D7BC",
            adapter: ProviderType::Ollama,
            auth_label: "Ollama API Key",
            support: ProviderSupport::Native,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::Ollama),
        },
        ProviderDefinition {
            id: "deepseek",
            label: "DeepSeek",
            description: "CNY／USD 可用餘額",
            key_url: "https://platform.deepseek.com/api_keys",
            icon: "/providers/deepseek.png",
            brand_color: "#4D6BFE",
            adapter: ProviderType::DeepSeek,
            auth_label: "DeepSeek API Key",
            support: ProviderSupport::Native,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::DeepSeek),
        },
        ProviderDefinition {
            id: "openrouter",
            label: "OpenRouter",
            description: "Key limit、credits 與每日／每月 usage",
            key_url: "https://openrouter.ai/keys",
            icon: "/providers/openrouter.png",
            brand_color: "#5B69E8",
            adapter: ProviderType::OpenRouter,
            auth_label: "OpenRouter API Key",
            support: ProviderSupport::Native,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::OpenRouter),
        },
        ProviderDefinition {
            id: "kimi",
            label: "Kimi",
            description: "國際站 available、voucher 與 cash balance",
            key_url: "https://platform.kimi.ai/console/api-keys",
            icon: "/providers/kimi.png",
            brand_color: "#111827",
            adapter: ProviderType::Moonshot,
            auth_label: "Kimi API Key（國際站）",
            support: ProviderSupport::Native,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::Moonshot),
        },
        ProviderDefinition {
            id: "kimi-cn",
            label: "Kimi 中國區",
            description: "中國站 available、voucher 與 cash balance",
            key_url: "https://platform.kimi.com/console/api-keys",
            icon: "/providers/kimi.png",
            brand_color: "#111827",
            adapter: ProviderType::Moonshot,
            auth_label: "Kimi API Key（中國站）",
            support: ProviderSupport::Native,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::Moonshot),
        },
        ProviderDefinition {
            id: "xai",
            label: "Grok / xAI",
            description: "Management API 的 prepaid credit balance",
            key_url: "https://console.x.ai/team/default/api-keys",
            icon: "/providers/xai.png",
            brand_color: "#E5E7EB",
            adapter: ProviderType::XaiManagement,
            auth_label: "xAI Management Key",
            support: ProviderSupport::ManagementKey,
            default_refresh_seconds: 300,
            requires_team_id: true,
            capabilities: capabilities_for(ProviderType::XaiManagement),
        },
        ProviderDefinition {
            id: "qwen",
            label: "Qwen / DashScope",
            description: "普通 API Key 主要提供 inference usage；帳務需 Custom endpoint 或雲端權限",
            key_url: "https://bailian.console.aliyun.com/",
            icon: "/providers/qwen.png",
            brand_color: "#615CED",
            adapter: ProviderType::Custom,
            auth_label: "Custom endpoint credentials",
            support: ProviderSupport::InferenceUsageOnly,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::Custom),
        },
        ProviderDefinition {
            id: "groq",
            label: "Groq",
            description: "目前以 Custom API 或 Gateway 統計為主",
            key_url: "https://console.groq.com/keys",
            icon: "/providers/groq.png",
            brand_color: "#F55036",
            adapter: ProviderType::Custom,
            auth_label: "Custom endpoint credentials",
            support: ProviderSupport::CustomOnly,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::Custom),
        },
        ProviderDefinition {
            id: "mistral",
            label: "Mistral",
            description: "目前以 Custom API 或 Gateway 統計為主",
            key_url: "https://console.mistral.ai/api-keys/",
            icon: "/providers/mistral.png",
            brand_color: "#FA500F",
            adapter: ProviderType::Custom,
            auth_label: "Custom endpoint credentials",
            support: ProviderSupport::CustomOnly,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::Custom),
        },
        ProviderDefinition {
            id: "together",
            label: "Together AI",
            description: "目前以 Custom API 或 Gateway 統計為主",
            key_url: "https://api.together.ai/settings/api-keys",
            icon: "/providers/together.png",
            brand_color: "#F24BE8",
            adapter: ProviderType::Custom,
            auth_label: "Custom endpoint credentials",
            support: ProviderSupport::CustomOnly,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::Custom),
        },
        ProviderDefinition {
            id: "custom",
            label: "Custom API",
            description: "自訂 GET endpoint、認證 header 與 JSON 欄位映射",
            key_url: "",
            icon: "/providers/generic.svg",
            brand_color: "#8B95A5",
            adapter: ProviderType::Custom,
            auth_label: "Custom endpoint credentials",
            support: ProviderSupport::CustomOnly,
            default_refresh_seconds: 300,
            requires_team_id: false,
            capabilities: capabilities_for(ProviderType::Custom),
        },
    ]
}

pub fn provider_definition(provider_id: &str) -> Option<ProviderDefinition> {
    provider_definitions()
        .into_iter()
        .find(|definition| definition.id == provider_id)
}

pub fn status_for_http(code: u16) -> ProviderStatus {
    match code {
        200..=299 => ProviderStatus::Ok,
        401 => ProviderStatus::AuthError,
        402 => ProviderStatus::InsufficientBalance,
        403 => ProviderStatus::PermissionError,
        429 => ProviderStatus::RateLimited,
        500..=599 => ProviderStatus::ProviderError,
        _ => ProviderStatus::ProviderError,
    }
}

pub async fn fetch_snapshot(
    client: &Client,
    profile: &ProviderProfile,
    api_key: &str,
) -> ProviderSnapshot {
    let result = match profile.provider_type {
        ProviderType::Ollama => fetch_ollama(client, profile, api_key).await,
        ProviderType::DeepSeek => fetch_deepseek(client, profile, api_key).await,
        ProviderType::OpenRouter => fetch_openrouter(client, profile, api_key).await,
        ProviderType::Moonshot => fetch_moonshot(client, profile, api_key).await,
        ProviderType::XaiManagement => fetch_xai_management(client, profile, api_key).await,
        ProviderType::Custom => fetch_custom(client, profile, api_key).await,
    };

    match result {
        Ok(snapshot) => snapshot,
        Err(error) => snapshot_from_error(profile, error),
    }
}

pub fn missing_credential_snapshot(profile: &ProviderProfile) -> ProviderSnapshot {
    snapshot_from_error(profile, AdapterError::Authentication)
}

async fn fetch_ollama(
    client: &Client,
    profile: &ProviderProfile,
    api_key: &str,
) -> Result<ProviderSnapshot, AdapterError> {
    let data = get_json(client, OLLAMA_BALANCE_URL, api_key).await?;
    parse_ollama(profile, &data)
}

async fn fetch_deepseek(
    client: &Client,
    profile: &ProviderProfile,
    api_key: &str,
) -> Result<ProviderSnapshot, AdapterError> {
    let data = get_json(client, DEEPSEEK_BALANCE_URL, api_key).await?;
    parse_deepseek(profile, &data)
}

async fn fetch_openrouter(
    client: &Client,
    profile: &ProviderProfile,
    api_key: &str,
) -> Result<ProviderSnapshot, AdapterError> {
    let key_data = get_json(client, OPENROUTER_KEY_URL, api_key).await?;
    let credits_data = get_json(client, OPENROUTER_CREDITS_URL, api_key).await.ok();
    parse_openrouter(profile, &key_data, credits_data.as_ref())
}

async fn fetch_moonshot(
    client: &Client,
    profile: &ProviderProfile,
    api_key: &str,
) -> Result<ProviderSnapshot, AdapterError> {
    let provider_id = profile.effective_provider_id();
    let (url, currency) = if provider_id == "kimi-cn" {
        (MOONSHOT_CN_BALANCE_URL, "CNY")
    } else {
        (MOONSHOT_GLOBAL_BALANCE_URL, "USD")
    };
    let data = get_json(client, url, api_key).await?;
    parse_moonshot(profile, &data, currency)
}

async fn fetch_xai_management(
    client: &Client,
    profile: &ProviderProfile,
    management_key: &str,
) -> Result<ProviderSnapshot, AdapterError> {
    let team_id = profile
        .team_id
        .as_deref()
        .map(str::trim)
        .filter(|team_id| !team_id.is_empty())
        .ok_or_else(|| AdapterError::Schema("xAI Team ID is required".into()))?;
    if !team_id
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(AdapterError::Schema(
            "xAI Team ID has invalid characters".into(),
        ));
    }

    let url = XAI_PREPAID_BALANCE_URL.replace("{team_id}", team_id);
    let data = get_json(client, &url, management_key).await?;
    parse_xai_balance(profile, &data)
}

async fn fetch_custom(
    client: &Client,
    profile: &ProviderProfile,
    api_key: &str,
) -> Result<ProviderSnapshot, AdapterError> {
    let config = profile
        .custom
        .as_ref()
        .ok_or_else(|| AdapterError::Schema("Custom API configuration is missing".into()))?;
    let url = config
        .url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .ok_or_else(|| AdapterError::Schema("Custom API URL is missing".into()))?;
    let parsed_url = reqwest::Url::parse(url)
        .map_err(|error| AdapterError::Schema(format!("invalid custom URL: {error}")))?;
    let is_local = matches!(
        parsed_url.host_str(),
        Some("localhost" | "127.0.0.1" | "::1")
    );
    if parsed_url.scheme() != "https" && !is_local {
        return Err(AdapterError::Schema(
            "custom URL must use HTTPS unless it targets localhost".into(),
        ));
    }

    let header_name = config
        .auth_header
        .as_deref()
        .unwrap_or(AUTHORIZATION.as_str());
    let auth_value = match config.auth_scheme.as_deref().unwrap_or("bearer") {
        "raw" => api_key.to_string(),
        _ => format!("Bearer {api_key}"),
    };
    let response = client
        .get(url)
        .header(header_name, auth_value)
        .header(ACCEPT, "application/json")
        .header(CACHE_CONTROL, "no-cache")
        .header(PRAGMA, "no-cache")
        .send()
        .await
        .map_err(map_reqwest_error)?;
    let data = decode_json_response(response).await?;
    parse_custom(profile, &data)
}

async fn get_json(client: &Client, url: &str, api_key: &str) -> Result<Value, AdapterError> {
    let response = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .header(ACCEPT, "application/json")
        .header(CACHE_CONTROL, "no-cache")
        .header(PRAGMA, "no-cache")
        .send()
        .await
        .map_err(map_reqwest_error)?;
    decode_json_response(response).await
}

async fn decode_json_response(response: reqwest::Response) -> Result<Value, AdapterError> {
    let status = response.status();
    if !status.is_success() {
        let retry_after_seconds = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        return Err(AdapterError::from_http(status, retry_after_seconds));
    }

    let body = response.text().await.map_err(map_reqwest_error)?;
    serde_json::from_str(&body).map_err(|error| AdapterError::Schema(error.to_string()))
}

impl AdapterError {
    fn from_http(status: StatusCode, retry_after_seconds: Option<u64>) -> Self {
        match status.as_u16() {
            401 => Self::Authentication,
            402 => Self::InsufficientBalance,
            403 => Self::Permission,
            429 => Self::RateLimited {
                retry_after_seconds,
            },
            _ => Self::Provider(format!("HTTP {}", status.as_u16())),
        }
    }
}

fn map_reqwest_error(error: reqwest::Error) -> AdapterError {
    if error.is_timeout() {
        AdapterError::Timeout
    } else if error.is_connect() || error.is_request() {
        AdapterError::Offline
    } else {
        AdapterError::Provider("網路請求失敗".into())
    }
}

fn snapshot_from_error(profile: &ProviderProfile, error: AdapterError) -> ProviderSnapshot {
    ProviderSnapshot {
        provider_id: profile.effective_provider_id(),
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        tracking_mode: profile.tracking_mode,
        label: profile.label.clone(),
        status: error.status(),
        fetched_at: Utc::now().to_rfc3339(),
        last_success_at: None,
        stale: true,
        consecutive_failures: 1,
        retry_at: None,
        next_refresh_at: None,
        capabilities: capabilities_for(profile.provider_type),
        windows: Vec::new(),
        balances: Vec::new(),
        usage: None,
        presentation: Presentation {
            mode: PresentationMode::Error,
            level_percent: 0.0,
            metric_label: "連線錯誤".into(),
            metric_value: "無法讀取".into(),
            health: Health::Error,
            primary_window_id: None,
        },
        error: Some(error.to_info()),
    }
}

fn parse_ollama(profile: &ProviderProfile, data: &Value) -> Result<ProviderSnapshot, AdapterError> {
    let root = data
        .as_object()
        .ok_or_else(|| AdapterError::Schema("root is not an object".into()))?;
    let included = root
        .get("included")
        .and_then(Value::as_object)
        .ok_or_else(|| AdapterError::Schema("missing included".into()))?;

    let mut windows = Vec::new();
    for (id, label) in [("session", "Session"), ("weekly", "Weekly")] {
        if let Some(window) = included.get(id).and_then(Value::as_object)
            && let Some(remaining) = number_field(window, "remaining_percent")
        {
            windows.push(UsageWindow {
                id: id.into(),
                label: label.into(),
                used_percent: Some(clamp_percent(100.0 - remaining)),
                remaining_percent: Some(clamp_percent(remaining)),
                resets_at: string_field(window, "resets_at"),
                detail: None,
            });
        }
    }

    if let (Some(balance), Some(allowance)) = (
        number_field(included, "balance_usd"),
        number_field(included, "allowance_usd"),
    ) && allowance > 0.0
    {
        let remaining = clamp_percent(balance / allowance * 100.0);
        windows.push(UsageWindow {
            id: "monthly".into(),
            label: "Monthly credits".into(),
            used_percent: Some(clamp_percent(100.0 - remaining)),
            remaining_percent: Some(remaining),
            resets_at: included
                .get("period")
                .and_then(Value::as_object)
                .and_then(|period| string_field(period, "until")),
            detail: Some(format!("${:.2} remaining of ${:.2}", balance, allowance)),
        });
    }

    let mut balances = Vec::new();
    if let Some(balance) = root
        .get("purchased")
        .and_then(Value::as_object)
        .and_then(|purchased| number_field(purchased, "balance_usd"))
    {
        balances.push(BalanceEntry {
            amount: balance,
            currency: "USD".into(),
            total: None,
            granted: None,
            topped_up: Some(balance),
        });
    }

    if windows.is_empty() && balances.is_empty() {
        return Err(AdapterError::Schema(
            "included and purchased data are empty".into(),
        ));
    }

    let presentation = ollama_presentation(profile, &windows, &balances);
    Ok(snapshot_success(
        profile,
        windows,
        balances,
        None,
        presentation,
    ))
}

fn parse_deepseek(
    profile: &ProviderProfile,
    data: &Value,
) -> Result<ProviderSnapshot, AdapterError> {
    let root = data
        .as_object()
        .ok_or_else(|| AdapterError::Schema("root is not an object".into()))?;
    let infos = root
        .get("balance_infos")
        .and_then(Value::as_array)
        .ok_or_else(|| AdapterError::Schema("missing balance_infos".into()))?;

    let mut balances = Vec::new();
    for info in infos {
        let Some(info) = info.as_object() else {
            continue;
        };
        let Some(currency) = string_field(info, "currency") else {
            continue;
        };
        let Some(total) = number_field(info, "total_balance") else {
            continue;
        };
        balances.push(BalanceEntry {
            amount: total,
            currency,
            total: Some(total),
            granted: number_field(info, "granted_balance"),
            topped_up: number_field(info, "topped_up_balance"),
        });
    }

    if balances.is_empty() {
        return Err(AdapterError::Schema("no valid balance_infos".into()));
    }

    let available = root
        .get("is_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let presentation = balance_presentation(profile, &balances, available);
    let mut snapshot = snapshot_success(profile, Vec::new(), balances, None, presentation);

    if !available {
        snapshot.status = ProviderStatus::InsufficientBalance;
        snapshot.error = Some(AdapterError::InsufficientBalance.to_info());
    }

    Ok(snapshot)
}

fn parse_openrouter(
    profile: &ProviderProfile,
    key_data: &Value,
    credits_data: Option<&Value>,
) -> Result<ProviderSnapshot, AdapterError> {
    let key_root = key_data
        .get("data")
        .unwrap_or(key_data)
        .as_object()
        .ok_or_else(|| AdapterError::Schema("key data is not an object".into()))?;

    let limit = number_field(key_root, "limit");
    let limit_remaining = number_field(key_root, "limit_remaining");
    let limit_reset = string_field(key_root, "limit_reset");
    let usage = KeyUsage {
        daily: number_field(key_root, "usage_daily"),
        weekly: number_field(key_root, "usage_weekly"),
        monthly: number_field(key_root, "usage_monthly"),
        all_time: number_field(key_root, "usage"),
    };

    let mut windows = Vec::new();
    let mut balances = Vec::new();
    if let (Some(limit), Some(remaining)) = (limit, limit_remaining)
        && limit > 0.0
    {
        let remaining_percent = clamp_percent(remaining / limit * 100.0);
        windows.push(UsageWindow {
            id: "key_limit".into(),
            label: "Key limit".into(),
            used_percent: Some(clamp_percent(100.0 - remaining_percent)),
            remaining_percent: Some(remaining_percent),
            resets_at: None,
            detail: limit_reset.map(|reset| format!("Reset type: {reset}")),
        });
    }

    if let Some(credits_data) = credits_data
        && let Some(credits_root) = credits_data.get("data").unwrap_or(credits_data).as_object()
        && let (Some(total), Some(used)) = (
            number_field(credits_root, "total_credits"),
            number_field(credits_root, "total_usage"),
        )
    {
        let remaining = (total - used).max(0.0);
        balances.push(BalanceEntry {
            amount: remaining,
            currency: "USD".into(),
            total: Some(total),
            granted: None,
            topped_up: None,
        });
    }

    if windows.is_empty() && balances.is_empty() && usage.all_time.is_none() {
        return Err(AdapterError::Schema(
            "no key limit, credits, or usage data".into(),
        ));
    }

    let presentation = openrouter_presentation(profile, &windows, &balances, &usage);
    let mut snapshot = snapshot_success(profile, windows, balances, Some(usage), presentation);

    if limit_remaining == Some(0.0) {
        snapshot.status = ProviderStatus::InsufficientBalance;
        snapshot.error = Some(AdapterError::InsufficientBalance.to_info());
    }

    Ok(snapshot)
}

fn parse_moonshot(
    profile: &ProviderProfile,
    data: &Value,
    currency: &str,
) -> Result<ProviderSnapshot, AdapterError> {
    let root = data
        .as_object()
        .ok_or_else(|| AdapterError::Schema("root is not an object".into()))?;
    if root.get("status").and_then(Value::as_bool) == Some(false) {
        return Err(AdapterError::Provider(
            "Kimi balance request was not successful".into(),
        ));
    }
    let balance_data = root
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(|| AdapterError::Schema("missing data".into()))?;
    let available = number_field(balance_data, "available_balance")
        .ok_or_else(|| AdapterError::Schema("missing available_balance".into()))?;
    let balances = vec![BalanceEntry {
        amount: available,
        currency: currency.into(),
        total: Some(available),
        granted: number_field(balance_data, "voucher_balance"),
        topped_up: number_field(balance_data, "cash_balance"),
    }];
    let presentation = balance_presentation(profile, &balances, available > 0.0);
    let mut snapshot = snapshot_success(profile, Vec::new(), balances, None, presentation);
    if available <= 0.0 {
        snapshot.status = ProviderStatus::InsufficientBalance;
        snapshot.error = Some(AdapterError::InsufficientBalance.to_info());
    }
    Ok(snapshot)
}

fn parse_xai_balance(
    profile: &ProviderProfile,
    data: &Value,
) -> Result<ProviderSnapshot, AdapterError> {
    let cents = data
        .get("total")
        .and_then(Value::as_object)
        .and_then(|total| total.get("val"))
        .and_then(value_as_f64)
        .ok_or_else(|| AdapterError::Schema("missing total.val".into()))?;
    let amount = cents / 100.0;
    let balances = vec![BalanceEntry {
        amount,
        currency: "USD".into(),
        total: Some(amount),
        granted: None,
        topped_up: None,
    }];
    let presentation = balance_presentation(profile, &balances, amount > 0.0);
    let mut snapshot = snapshot_success(profile, Vec::new(), balances, None, presentation);
    if amount <= 0.0 {
        snapshot.status = ProviderStatus::InsufficientBalance;
        snapshot.error = Some(AdapterError::InsufficientBalance.to_info());
    }
    Ok(snapshot)
}

fn parse_custom(profile: &ProviderProfile, data: &Value) -> Result<ProviderSnapshot, AdapterError> {
    let config = profile
        .custom
        .as_ref()
        .ok_or_else(|| AdapterError::Schema("Custom API configuration is missing".into()))?;
    let remaining = config
        .remaining_path
        .as_deref()
        .and_then(|path| value_at_path(data, path))
        .and_then(value_as_f64);
    let limit = config
        .limit_path
        .as_deref()
        .and_then(|path| value_at_path(data, path))
        .and_then(value_as_f64);
    let used = config
        .used_path
        .as_deref()
        .and_then(|path| value_at_path(data, path))
        .and_then(value_as_f64);
    let currency = config
        .currency_path
        .as_deref()
        .and_then(|path| value_at_path(data, path))
        .and_then(Value::as_str)
        .unwrap_or("USD")
        .to_uppercase();
    let resets_at = config
        .resets_at_path
        .as_deref()
        .and_then(|path| value_at_path(data, path))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    if let Some(limit) = limit.filter(|limit| *limit > 0.0) {
        let remaining = remaining
            .or_else(|| used.map(|used| (limit - used).max(0.0)))
            .ok_or_else(|| {
                AdapterError::Schema("custom provider needs remaining or used value".into())
            })?;
        let remaining_percent = clamp_percent(remaining / limit * 100.0);
        let window = UsageWindow {
            id: "custom_limit".into(),
            label: "Usage limit".into(),
            used_percent: Some(clamp_percent(100.0 - remaining_percent)),
            remaining_percent: Some(remaining_percent),
            resets_at: resets_at.clone(),
            detail: Some(format!("remaining {remaining:.2} / limit {limit:.2}")),
        };
        let presentation = Presentation {
            mode: PresentationMode::Quota,
            level_percent: remaining_percent,
            metric_label: "Usage limit".into(),
            metric_value: format!("{remaining_percent:.0}% left"),
            health: health_from_remaining(remaining_percent),
            primary_window_id: Some(window.id.clone()),
        };
        let mut snapshot = snapshot_success(profile, vec![window], Vec::new(), None, presentation);
        snapshot.capabilities.quota = true;
        snapshot.capabilities.balance = false;
        snapshot.capabilities.reset_time = resets_at.is_some();
        return Ok(snapshot);
    }

    if let Some(amount) = remaining {
        let balances = vec![BalanceEntry {
            amount,
            currency,
            total: None,
            granted: None,
            topped_up: None,
        }];
        let presentation = balance_presentation(profile, &balances, amount > 0.0);
        let mut snapshot = snapshot_success(profile, Vec::new(), balances, None, presentation);
        snapshot.capabilities.quota = false;
        snapshot.capabilities.reset_time = false;
        return Ok(snapshot);
    }

    if let Some(used) = used {
        let usage = KeyUsage {
            daily: Some(used),
            ..KeyUsage::default()
        };
        let presentation = Presentation {
            mode: PresentationMode::UsageOnly,
            level_percent: 100.0,
            metric_label: "Usage".into(),
            metric_value: format!("{used:.2}"),
            health: Health::Safe,
            primary_window_id: None,
        };
        let mut snapshot =
            snapshot_success(profile, Vec::new(), Vec::new(), Some(usage), presentation);
        snapshot.capabilities.balance = false;
        snapshot.capabilities.quota = false;
        snapshot.capabilities.reset_time = false;
        return Ok(snapshot);
    }

    Err(AdapterError::Schema(
        "custom JSON paths did not match balance, limit, or usage".into(),
    ))
}

fn snapshot_success(
    profile: &ProviderProfile,
    windows: Vec<UsageWindow>,
    balances: Vec<BalanceEntry>,
    usage: Option<KeyUsage>,
    presentation: Presentation,
) -> ProviderSnapshot {
    let now = Utc::now().to_rfc3339();
    ProviderSnapshot {
        provider_id: profile.effective_provider_id(),
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        tracking_mode: profile.tracking_mode,
        label: profile.label.clone(),
        status: ProviderStatus::Ok,
        fetched_at: now.clone(),
        last_success_at: Some(now),
        stale: false,
        consecutive_failures: 0,
        retry_at: None,
        next_refresh_at: None,
        capabilities: capabilities_for(profile.provider_type),
        windows,
        balances,
        usage,
        presentation,
        error: None,
    }
}

fn ollama_presentation(
    profile: &ProviderProfile,
    windows: &[UsageWindow],
    balances: &[BalanceEntry],
) -> Presentation {
    if profile.tracking_mode != TrackingMode::Balance
        && let Some((window, remaining)) = windows
            .iter()
            .filter_map(|window| {
                window
                    .remaining_percent
                    .map(|remaining| (window, remaining))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
    {
        return Presentation {
            mode: PresentationMode::Quota,
            level_percent: remaining,
            metric_label: format!("訂閱 {}", window.label),
            metric_value: format!("{remaining:.0}% left"),
            health: health_from_subscription(profile, remaining),
            primary_window_id: Some(window.id.clone()),
        };
    }

    if !balances.is_empty() {
        let mut presentation = balance_presentation(profile, balances, true);
        presentation.metric_label = "加購餘額".into();
        return presentation;
    }

    let amount = 0.0;
    Presentation {
        mode: PresentationMode::Balance,
        level_percent: 0.0,
        metric_label: "加購餘額".into(),
        metric_value: format!("${amount:.2}"),
        health: Health::Critical,
        primary_window_id: None,
    }
}

fn balance_presentation(
    profile: &ProviderProfile,
    balances: &[BalanceEntry],
    available: bool,
) -> Presentation {
    let primary = select_primary_balance(profile, balances);
    let Some(balance) = primary else {
        return Presentation {
            mode: PresentationMode::Error,
            level_percent: 0.0,
            metric_label: "可用餘額".into(),
            metric_value: "無法讀取".into(),
            health: Health::Error,
            primary_window_id: None,
        };
    };

    let full_bottle = profile.full_bottle_amount.unwrap_or(50.0).max(0.01);
    let level = clamp_percent(balance.amount / full_bottle * 100.0);
    let health = if !available || balance.amount <= profile.critical_threshold.unwrap_or(5.0) {
        Health::Critical
    } else if balance.amount <= profile.warning_threshold.unwrap_or(20.0) {
        Health::Warning
    } else {
        Health::Safe
    };

    Presentation {
        mode: PresentationMode::Balance,
        level_percent: level,
        metric_label: "可用餘額".into(),
        metric_value: format_money(balance.amount, &balance.currency),
        health,
        primary_window_id: None,
    }
}

fn openrouter_presentation(
    profile: &ProviderProfile,
    windows: &[UsageWindow],
    balances: &[BalanceEntry],
    usage: &KeyUsage,
) -> Presentation {
    if let Some(window) = windows.first() {
        let remaining = window.remaining_percent.unwrap_or(0.0);
        return Presentation {
            mode: PresentationMode::KeyLimit,
            level_percent: remaining,
            metric_label: window.label.clone(),
            metric_value: format!("{remaining:.0}% left"),
            health: health_from_remaining(remaining),
            primary_window_id: Some(window.id.clone()),
        };
    }

    if let Some(balance) = balances.first() {
        let level = balance
            .total
            .filter(|total| *total > 0.0)
            .map(|total| clamp_percent(balance.amount / total * 100.0))
            .unwrap_or(100.0);
        return Presentation {
            mode: PresentationMode::Balance,
            level_percent: level,
            metric_label: "Credits".into(),
            metric_value: format!("${:.2}", balance.amount),
            health: health_from_remaining(level),
            primary_window_id: None,
        };
    }

    let warning = profile.warning_threshold.unwrap_or(5.0);
    let spent = usage.daily.unwrap_or(0.0);
    Presentation {
        mode: PresentationMode::UsageOnly,
        level_percent: 100.0,
        metric_label: "Daily usage".into(),
        metric_value: format!("${spent:.2}"),
        health: if spent >= warning {
            Health::Warning
        } else {
            Health::Safe
        },
        primary_window_id: None,
    }
}

fn select_primary_balance<'a>(
    profile: &ProviderProfile,
    balances: &'a [BalanceEntry],
) -> Option<&'a BalanceEntry> {
    if let Some(currency) = &profile.primary_currency
        && let Some(balance) = balances
            .iter()
            .find(|balance| balance.currency.eq_ignore_ascii_case(currency))
    {
        return Some(balance);
    }
    balances
        .iter()
        .find(|balance| balance.amount > 0.0)
        .or_else(|| balances.first())
}

fn health_from_remaining(remaining: f64) -> Health {
    if remaining < 15.0 {
        Health::Critical
    } else if remaining < 40.0 {
        Health::Warning
    } else {
        Health::Safe
    }
}

fn health_from_subscription(profile: &ProviderProfile, remaining: f64) -> Health {
    let critical = profile.subscription_critical_percent.unwrap_or(5.0);
    let warning = profile.subscription_warning_percent.unwrap_or(20.0);
    if remaining <= critical {
        Health::Critical
    } else if remaining <= warning {
        Health::Warning
    } else {
        Health::Safe
    }
}

fn format_money(amount: f64, currency: &str) -> String {
    let symbol = match currency.to_ascii_uppercase().as_str() {
        "CNY" => "¥",
        "USD" => "$",
        _ => "",
    };
    format!("{symbol}{amount:.2} {currency}")
}

fn clamp_percent(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

fn number_field(object: &Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key).and_then(value_as_f64)
}

fn value_as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|number| number.parse::<f64>().ok()))
}

fn string_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.').filter(|segment| !segment.is_empty()) {
        current = if let Ok(index) = segment.parse::<usize>() {
            current.get(index)?
        } else {
            current.get(segment)?
        };
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn serve_once(status: &str, headers: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_string();
        let headers = headers.to_string();
        let body = body.to_string();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 4096];
            let _ = socket.read(&mut request).await;
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        format!("http://{address}/")
    }

    fn profile(provider_type: ProviderType) -> ProviderProfile {
        ProviderProfile {
            id: "test-profile".into(),
            provider_id: provider_type.as_str().into(),
            provider_type,
            label: "Test".into(),
            enabled: true,
            refresh_seconds: 300,
            tracking_mode: TrackingMode::Auto,
            subscription_warning_percent: Some(20.0),
            subscription_critical_percent: Some(5.0),
            full_bottle_amount: Some(50.0),
            warning_threshold: Some(20.0),
            critical_threshold: Some(5.0),
            primary_currency: Some("CNY".into()),
            team_id: None,
            custom: None,
        }
    }

    #[test]
    fn parses_ollama_legacy_quotas() {
        let data = serde_json::json!({
            "included": {
                "session": {
                    "remaining_percent": 75,
                    "resets_at": "2026-11-07T16:04:00Z"
                },
                "weekly": {
                    "remaining_percent": 40,
                    "resets_at": "2026-11-08T16:04:00Z"
                }
            },
            "purchased": {
                "balance_usd": 25.0
            }
        });

        let snapshot = parse_ollama(&profile(ProviderType::Ollama), &data).unwrap();
        assert_eq!(snapshot.windows.len(), 2);
        assert_eq!(snapshot.windows[0].used_percent, Some(25.0));
        assert_eq!(
            snapshot.presentation.primary_window_id.as_deref(),
            Some("weekly")
        );
        assert_eq!(snapshot.presentation.level_percent, 40.0);
        assert_eq!(snapshot.balances[0].amount, 25.0);
    }

    #[test]
    fn parses_ollama_monthly_credits() {
        let data = serde_json::json!({
            "included": {
                "balance_usd": 39.0,
                "allowance_usd": 60.0,
                "period": {
                    "from": "2026-11-01T00:00:00Z",
                    "until": "2026-12-01T00:00:00Z"
                }
            }
        });

        let snapshot = parse_ollama(&profile(ProviderType::Ollama), &data).unwrap();
        assert_eq!(snapshot.windows.len(), 1);
        assert_eq!(snapshot.windows[0].id, "monthly");
        assert_eq!(snapshot.windows[0].remaining_percent, Some(65.0));
        assert_eq!(
            snapshot.windows[0].resets_at.as_deref(),
            Some("2026-12-01T00:00:00Z")
        );
    }

    #[test]
    fn ollama_tracking_mode_keeps_subscription_and_balance_distinct() {
        let data = serde_json::json!({
            "included": {
                "session": { "remaining_percent": 25, "resets_at": "2026-11-08T03:00:00Z" },
                "weekly": { "remaining_percent": 60, "resets_at": "2026-11-10T03:00:00Z" }
            },
            "purchased": { "balance_usd": 3.36 }
        });

        let mut subscription = profile(ProviderType::Ollama);
        subscription.tracking_mode = TrackingMode::Subscription;
        subscription.subscription_warning_percent = Some(30.0);
        subscription.subscription_critical_percent = Some(10.0);
        let subscription_snapshot = parse_ollama(&subscription, &data).unwrap();
        assert_eq!(
            subscription_snapshot.presentation.mode,
            PresentationMode::Quota
        );
        assert_eq!(subscription_snapshot.presentation.level_percent, 25.0);
        assert_eq!(subscription_snapshot.presentation.health, Health::Warning);

        let mut balance = profile(ProviderType::Ollama);
        balance.tracking_mode = TrackingMode::Balance;
        let balance_snapshot = parse_ollama(&balance, &data).unwrap();
        assert_eq!(
            balance_snapshot.presentation.mode,
            PresentationMode::Balance
        );
        assert_eq!(balance_snapshot.presentation.metric_label, "加購餘額");
        assert_eq!(balance_snapshot.presentation.metric_value, "$3.36 USD");
    }

    #[test]
    fn parses_deepseek_multi_currency() {
        let data = serde_json::json!({
            "is_available": true,
            "balance_infos": [
                {
                    "currency": "USD",
                    "total_balance": "0.00",
                    "granted_balance": "0.00",
                    "topped_up_balance": "0.00"
                },
                {
                    "currency": "CNY",
                    "total_balance": "37.44",
                    "granted_balance": "2.44",
                    "topped_up_balance": "35.00"
                }
            ]
        });

        let mut deepseek = profile(ProviderType::DeepSeek);
        deepseek.primary_currency = Some("CNY".into());
        deepseek.full_bottle_amount = Some(50.0);
        deepseek.warning_threshold = Some(20.0);
        deepseek.critical_threshold = Some(5.0);
        let snapshot = parse_deepseek(&deepseek, &data).unwrap();
        assert_eq!(snapshot.balances.len(), 2);
        assert_eq!(snapshot.presentation.metric_value, "¥37.44 CNY");
        assert_eq!(snapshot.presentation.level_percent, 74.88);
        assert_eq!(snapshot.presentation.health, Health::Safe);
        assert_eq!(snapshot.presentation.mode, PresentationMode::Balance);
    }

    #[test]
    fn deepseek_primary_currency_reflects_top_up() {
        let data = serde_json::json!({
            "is_available": true,
            "balance_infos": [
                {
                    "currency": "USD",
                    "total_balance": "3.36",
                    "granted_balance": "0.00",
                    "topped_up_balance": "3.36"
                },
                {
                    "currency": "CNY",
                    "total_balance": "103.36",
                    "granted_balance": "3.36",
                    "topped_up_balance": "100.00"
                }
            ]
        });

        let mut profile = profile(ProviderType::DeepSeek);
        profile.primary_currency = Some("CNY".into());
        profile.full_bottle_amount = Some(150.0);
        let snapshot = parse_deepseek(&profile, &data).unwrap();
        assert_eq!(snapshot.presentation.metric_value, "¥103.36 CNY");
        assert_eq!(snapshot.presentation.health, Health::Safe);

        profile.primary_currency = Some("USD".into());
        let usd = parse_deepseek(&profile, &data).unwrap();
        assert_eq!(usd.presentation.metric_value, "$3.36 USD");
        assert_eq!(usd.presentation.health, Health::Critical);
    }

    #[test]
    fn parses_kimi_balance_for_global_and_china_currency() {
        let data = serde_json::json!({
            "code": 0,
            "status": true,
            "data": {
                "available_balance": 49.58894,
                "voucher_balance": 46.58893,
                "cash_balance": 3.00001
            }
        });
        let mut kimi = profile(ProviderType::Moonshot);
        kimi.provider_id = "kimi".into();
        let global = parse_moonshot(&kimi, &data, "USD").unwrap();
        assert_eq!(global.presentation.mode, PresentationMode::Balance);
        assert_eq!(global.balances[0].currency, "USD");
        assert_eq!(global.balances[0].amount, 49.58894);

        kimi.provider_id = "kimi-cn".into();
        let china = parse_moonshot(&kimi, &data, "CNY").unwrap();
        assert_eq!(china.balances[0].currency, "CNY");
    }

    #[test]
    fn parses_xai_prepaid_balance_cents() {
        let data = serde_json::json!({
            "total": { "val": "1234" },
            "changes": []
        });
        let mut xai = profile(ProviderType::XaiManagement);
        xai.provider_id = "xai".into();
        xai.team_id = Some("team-123".into());
        let snapshot = parse_xai_balance(&xai, &data).unwrap();
        assert_eq!(snapshot.balances[0].amount, 12.34);
        assert_eq!(snapshot.presentation.mode, PresentationMode::Balance);
    }

    #[test]
    fn custom_adapter_infers_limit_and_balance_modes() {
        let mut custom = profile(ProviderType::Custom);
        custom.provider_id = "qwen".into();
        custom.custom = Some(CustomProviderConfig {
            url: Some("https://example.com/usage".into()),
            auth_header: Some("Authorization".into()),
            auth_scheme: Some("bearer".into()),
            remaining_path: Some("data.remaining".into()),
            limit_path: Some("data.limit".into()),
            used_path: Some("data.used".into()),
            currency_path: Some("data.currency".into()),
            resets_at_path: Some("data.reset_at".into()),
        });

        let quota = parse_custom(
            &custom,
            &serde_json::json!({
                "data": {
                    "remaining": 25,
                    "limit": 100,
                    "currency": "USD",
                    "reset_at": "2026-12-01T00:00:00Z"
                }
            }),
        )
        .unwrap();
        assert_eq!(quota.presentation.mode, PresentationMode::Quota);
        assert_eq!(quota.presentation.level_percent, 25.0);

        custom.custom.as_mut().unwrap().limit_path = None;
        let balance = parse_custom(
            &custom,
            &serde_json::json!({
                "data": {
                    "remaining": 37.5,
                    "currency": "CNY"
                }
            }),
        )
        .unwrap();
        assert_eq!(balance.presentation.mode, PresentationMode::Balance);
        assert_eq!(balance.balances[0].currency, "CNY");
    }

    #[test]
    fn catalog_contains_major_providers_and_icons() {
        let catalog = provider_definitions();
        for id in [
            "ollama",
            "deepseek",
            "openrouter",
            "kimi",
            "kimi-cn",
            "xai",
            "qwen",
            "groq",
            "mistral",
            "together",
            "custom",
        ] {
            let definition = catalog
                .iter()
                .find(|definition| definition.id == id)
                .unwrap_or_else(|| panic!("missing provider {id}"));
            assert!(definition.icon.starts_with("/providers/"));
        }
    }

    #[test]
    fn parses_openrouter_key_limit_and_credits() {
        let key_data = serde_json::json!({
            "data": {
                "limit": 20.0,
                "limit_remaining": 7.25,
                "limit_reset": "monthly",
                "usage": 12.75,
                "usage_daily": 0.42,
                "usage_weekly": 2.0,
                "usage_monthly": 4.75
            }
        });
        let credits_data = serde_json::json!({
            "data": {
                "total_credits": 100.5,
                "total_usage": 25.75
            }
        });

        let snapshot = parse_openrouter(
            &profile(ProviderType::OpenRouter),
            &key_data,
            Some(&credits_data),
        )
        .unwrap();
        assert_eq!(snapshot.windows[0].remaining_percent, Some(36.25));
        assert_eq!(snapshot.balances[0].amount, 74.75);
        assert_eq!(snapshot.usage.as_ref().unwrap().daily, Some(0.42));
        assert_eq!(snapshot.presentation.health, Health::Warning);
    }

    #[test]
    fn classifies_http_errors() {
        assert_eq!(status_for_http(401), ProviderStatus::AuthError);
        assert_eq!(status_for_http(402), ProviderStatus::InsufficientBalance);
        assert_eq!(status_for_http(403), ProviderStatus::PermissionError);
        assert_eq!(status_for_http(429), ProviderStatus::RateLimited);
        assert_eq!(status_for_http(503), ProviderStatus::ProviderError);
    }

    #[test]
    fn malformed_payload_becomes_schema_error() {
        let snapshot = parse_deepseek(
            &profile(ProviderType::DeepSeek),
            &serde_json::json!({"unexpected": true}),
        )
        .unwrap_err();
        assert_eq!(snapshot.status(), ProviderStatus::SchemaError);
    }

    #[tokio::test]
    async fn http_auth_error_is_classified_without_leaking_key() {
        let url = serve_once("401 Unauthorized", "", r#"{"error":"invalid"}"#).await;
        let error = get_json(&Client::new(), &url, "secret-api-key")
            .await
            .unwrap_err();
        let info = error.to_info();
        assert_eq!(error.status(), ProviderStatus::AuthError);
        assert_eq!(info.code, "authentication_error");
        assert!(!info.message.contains("secret-api-key"));
    }

    #[tokio::test]
    async fn http_rate_limit_preserves_retry_after() {
        let url = serve_once("429 Too Many Requests", "Retry-After: 12\r\n", "{}").await;
        let error = get_json(&Client::new(), &url, "key").await.unwrap_err();
        let info = error.to_info();
        assert_eq!(error.status(), ProviderStatus::RateLimited);
        assert_eq!(info.retry_after_seconds, Some(12));
    }

    #[tokio::test]
    async fn invalid_json_is_reported_as_schema_error() {
        let url = serve_once("200 OK", "", "{not-json").await;
        let error = get_json(&Client::new(), &url, "key").await.unwrap_err();
        assert_eq!(error.status(), ProviderStatus::SchemaError);
    }

    #[test]
    fn error_snapshot_never_panics_or_contains_secret() {
        let snapshot =
            snapshot_from_error(&profile(ProviderType::Ollama), AdapterError::Authentication);
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert_eq!(snapshot.status, ProviderStatus::AuthError);
        assert!(serialized.contains("authentication_error"));
        assert!(!serialized.contains("Bearer"));
    }
}
