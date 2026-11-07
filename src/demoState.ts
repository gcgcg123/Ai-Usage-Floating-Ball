import type {
  ProviderCapabilities,
  ProviderDefinition,
  ProviderProfile,
  ProviderSnapshot,
  UsageState,
} from "./providerTypes";

const nativeCapabilities: ProviderCapabilities = {
  balance: true,
  quota: false,
  resetTime: false,
  perKeyLimit: false,
  monthlyCredits: false,
  multiCurrency: false,
  usageOnly: false,
};

export const demoCatalog: ProviderDefinition[] = [
  {
    id: "ollama",
    label: "Ollama",
    description: "Session、Weekly、Monthly credits 與 credit balance",
    keyUrl: "https://ollama.com/settings/keys",
    icon: "/providers/ollama.png",
    brandColor: "#55D7BC",
    adapter: "ollama",
    authLabel: "Ollama API Key",
    support: "native",
    defaultRefreshSeconds: 300,
    requiresTeamId: false,
    capabilities: {
      ...nativeCapabilities,
      quota: true,
      resetTime: true,
      monthlyCredits: true,
    },
  },
  {
    id: "deepseek",
    label: "DeepSeek",
    description: "CNY／USD 可用餘額",
    keyUrl: "https://platform.deepseek.com/api_keys",
    icon: "/providers/deepseek.png",
    brandColor: "#4D6BFE",
    adapter: "deepseek",
    authLabel: "DeepSeek API Key",
    support: "native",
    defaultRefreshSeconds: 300,
    requiresTeamId: false,
    capabilities: {
      ...nativeCapabilities,
      multiCurrency: true,
    },
  },
];

function profile(
  id: string,
  providerId: string,
  label: string,
  trackingMode: "auto" | "subscription" | "balance",
  fullBottleAmount: number | null,
  primaryCurrency: string | null,
): ProviderProfile {
  return {
    id,
    providerId,
    type: providerId === "deepseek" ? "deepseek" : "ollama",
    label,
    enabled: true,
    refreshSeconds: 300,
    trackingMode,
    subscriptionWarningPercent: trackingMode === "balance" ? null : 20,
    subscriptionCriticalPercent: trackingMode === "balance" ? null : 5,
    fullBottleAmount,
    warningThreshold: 20,
    criticalThreshold: 5,
    primaryCurrency,
    teamId: null,
    custom: null,
  };
}

function ollamaSnapshot(now: string): ProviderSnapshot {
  return {
    providerId: "ollama",
    profileId: "demo-ollama",
    type: "ollama",
    trackingMode: "subscription",
    label: "Ollama",
    status: "ok",
    fetchedAt: now,
    lastSuccessAt: now,
    stale: false,
    consecutiveFailures: 0,
    retryAt: null,
    nextRefreshAt: new Date(Date.now() + 5 * 60_000).toISOString(),
    capabilities: demoCatalog[0].capabilities,
    windows: [
      {
        id: "session",
        label: "Session",
        usedPercent: 18,
        remainingPercent: 82,
        resetsAt: new Date(Date.now() + 3 * 60 * 60_000).toISOString(),
        detail: null,
      },
      {
        id: "weekly",
        label: "Weekly",
        usedPercent: 12,
        remainingPercent: 88,
        resetsAt: new Date(Date.now() + 4 * 24 * 60 * 60_000).toISOString(),
        detail: null,
      },
      {
        id: "monthly",
        label: "Monthly credits",
        usedPercent: 9,
        remainingPercent: 91,
        resetsAt: new Date(Date.now() + 22 * 24 * 60 * 60_000).toISOString(),
        detail: "$45.50 remaining of $50.00",
      },
    ],
    balances: [
      {
        amount: 3.36,
        currency: "USD",
        total: null,
        granted: null,
        toppedUp: 3.36,
      },
    ],
    usage: null,
    presentation: {
      mode: "quota",
      levelPercent: 82,
      metricLabel: "訂閱 Session",
      metricValue: "82% left",
      health: "safe",
      primaryWindowId: "session",
    },
    error: null,
  };
}

function deepseekSnapshot(now: string): ProviderSnapshot {
  return {
    providerId: "deepseek",
    profileId: "demo-deepseek",
    type: "deepseek",
    trackingMode: "balance",
    label: "DeepSeek",
    status: "ok",
    fetchedAt: now,
    lastSuccessAt: now,
    stale: false,
    consecutiveFailures: 0,
    retryAt: null,
    nextRefreshAt: new Date(Date.now() + 5 * 60_000).toISOString(),
    capabilities: demoCatalog[1].capabilities,
    windows: [],
    balances: [
      {
        amount: 103.36,
        currency: "CNY",
        total: 103.36,
        granted: 3.36,
        toppedUp: 100,
      },
      {
        amount: 3.36,
        currency: "USD",
        total: 3.36,
        granted: 0,
        toppedUp: 3.36,
      },
    ],
    usage: null,
    presentation: {
      mode: "balance",
      levelPercent: 68.91,
      metricLabel: "可用餘額",
      metricValue: "¥103.36 CNY",
      health: "safe",
      primaryWindowId: null,
    },
    error: null,
  };
}

export function demoUsageState(selectedProviderId?: string | null): UsageState {
  const now = new Date(Date.now() - 35_000).toISOString();
  const profiles = [
    profile("demo-ollama", "ollama", "Ollama", "subscription", null, null),
    profile("demo-deepseek", "deepseek", "DeepSeek", "balance", 150, "CNY"),
  ];
  return {
    profiles,
    snapshots: [ollamaSnapshot(now), deepseekSnapshot(now)],
    selectedProfileId:
      selectedProviderId === "deepseek"
        ? "demo-deepseek"
        : "demo-ollama",
    paused: false,
    updatedAt: now,
  };
}
