export type ProviderType =
  | "ollama"
  | "deepseek"
  | "openrouter"
  | "moonshot"
  | "xai_management"
  | "custom";

export type TrackingMode = "auto" | "subscription" | "balance";

export type ProviderSupport =
  | "native"
  | "management_key"
  | "inference_usage_only"
  | "custom_only";

export type PresentationMode =
  | "quota"
  | "balance"
  | "key_limit"
  | "usage_only"
  | "hybrid"
  | "error"
  | "unknown";

export type ProviderStatus =
  | "ok"
  | "auth_error"
  | "permission_error"
  | "rate_limited"
  | "insufficient_balance"
  | "provider_error"
  | "schema_error"
  | "offline"
  | "timeout"
  | "disabled"
  | "unknown";

export type Health = "safe" | "warning" | "critical" | "error" | "unknown";

export interface ProviderCapabilities {
  balance: boolean;
  quota: boolean;
  resetTime: boolean;
  perKeyLimit: boolean;
  monthlyCredits: boolean;
  multiCurrency: boolean;
  usageOnly: boolean;
}

export interface UsageWindow {
  id: string;
  label: string;
  usedPercent: number | null;
  remainingPercent: number | null;
  resetsAt: string | null;
  detail: string | null;
}

export interface BalanceEntry {
  amount: number;
  currency: string;
  total: number | null;
  granted: number | null;
  toppedUp: number | null;
}

export interface KeyUsage {
  daily: number | null;
  weekly: number | null;
  monthly: number | null;
  allTime: number | null;
}

export interface Presentation {
  mode: PresentationMode;
  levelPercent: number;
  metricLabel: string;
  metricValue: string;
  health: Health;
  primaryWindowId: string | null;
}

export interface ProviderErrorInfo {
  code: string;
  message: string;
  action: string;
  retryAfterSeconds: number | null;
}

export interface ProviderSnapshot {
  providerId: string;
  profileId: string;
  type: ProviderType;
  trackingMode: TrackingMode;
  label: string;
  status: ProviderStatus;
  fetchedAt: string;
  lastSuccessAt: string | null;
  stale: boolean;
  consecutiveFailures: number;
  retryAt: string | null;
  nextRefreshAt: string | null;
  capabilities: ProviderCapabilities;
  windows: UsageWindow[];
  balances: BalanceEntry[];
  usage: KeyUsage | null;
  presentation: Presentation;
  error: ProviderErrorInfo | null;
}

export interface CustomProviderConfig {
  url: string | null;
  authHeader: string | null;
  authScheme: "bearer" | "raw" | null;
  remainingPath: string | null;
  limitPath: string | null;
  usedPath: string | null;
  currencyPath: string | null;
  resetsAtPath: string | null;
}

export interface ProviderProfile {
  id: string;
  providerId: string;
  type: ProviderType;
  label: string;
  enabled: boolean;
  refreshSeconds: number;
  trackingMode: TrackingMode;
  subscriptionWarningPercent: number | null;
  subscriptionCriticalPercent: number | null;
  fullBottleAmount: number | null;
  warningThreshold: number | null;
  criticalThreshold: number | null;
  primaryCurrency: string | null;
  teamId: string | null;
  custom: CustomProviderConfig | null;
}

export interface ProviderDefinition {
  id: string;
  label: string;
  description: string;
  keyUrl: string;
  icon: string;
  brandColor: string;
  adapter: ProviderType;
  authLabel: string;
  support: ProviderSupport;
  defaultRefreshSeconds: number;
  requiresTeamId: boolean;
  capabilities: ProviderCapabilities;
}

export interface UsageState {
  profiles: ProviderProfile[];
  snapshots: ProviderSnapshot[];
  selectedProfileId: string | null;
  paused: boolean;
  updatedAt: string;
}

export interface TestProviderResult {
  snapshot: ProviderSnapshot;
}
