import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  ProviderDefinition,
  ProviderProfile,
  ProviderSnapshot,
  UsageState,
} from "./providerTypes";
import SettingsApp from "./SettingsApp";

const invokeMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());
const openUrlMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: openUrlMock,
}));

const capabilities = {
  balance: true,
  quota: false,
  resetTime: false,
  perKeyLimit: false,
  monthlyCredits: false,
  multiCurrency: true,
  usageOnly: false,
};

const catalog: ProviderDefinition[] = [
  {
    id: "ollama",
    label: "Ollama",
    description: "Session、Weekly、Monthly credits 與 credit balance",
    keyUrl: "https://ollama.com/settings/keys",
    icon: "/providers/ollama.png",
    brandColor: "#55d7bc",
    adapter: "ollama",
    authLabel: "Ollama API Key",
    support: "native",
    defaultRefreshSeconds: 300,
    requiresTeamId: false,
    capabilities: {
      ...capabilities,
      quota: true,
      balance: true,
      monthlyCredits: true,
    },
  },
  {
    id: "deepseek",
    label: "DeepSeek",
    description: "Official DeepSeek balance API",
    keyUrl: "https://platform.deepseek.com/api_keys",
    icon: "/providers/deepseek.png",
    brandColor: "#4d6bfe",
    adapter: "deepseek",
    authLabel: "DeepSeek API Key",
    support: "native",
    defaultRefreshSeconds: 600,
    requiresTeamId: false,
    capabilities,
  },
  {
    id: "kimi",
    label: "Kimi",
    description: "Moonshot global balance API",
    keyUrl: "https://platform.moonshot.ai/console/api-keys",
    icon: "/providers/kimi.png",
    brandColor: "#111111",
    adapter: "moonshot",
    authLabel: "Kimi API Key",
    support: "native",
    defaultRefreshSeconds: 600,
    requiresTeamId: false,
    capabilities,
  },
];

const deepseekProfile: ProviderProfile = {
  id: "deepseek-profile",
  providerId: "deepseek",
  type: "deepseek",
  label: "DeepSeek personal",
  enabled: true,
  refreshSeconds: 600,
  trackingMode: "balance",
  subscriptionWarningPercent: 20,
  subscriptionCriticalPercent: 5,
  fullBottleAmount: 50,
  warningThreshold: 20,
  criticalThreshold: 5,
  primaryCurrency: "CNY",
  teamId: null,
  custom: null,
};

const kimiProfile: ProviderProfile = {
  ...deepseekProfile,
  id: "kimi-profile",
  providerId: "kimi",
  type: "moonshot",
  label: "Kimi personal",
  primaryCurrency: "USD",
};

function snapshotFor(profile: ProviderProfile): ProviderSnapshot {
  return {
    providerId: profile.providerId,
    profileId: profile.id,
    type: profile.type,
    trackingMode: profile.trackingMode,
    label: profile.label,
    status: "ok",
    fetchedAt: "2026-11-07T15:00:00.000Z",
    lastSuccessAt: "2026-11-07T15:00:00.000Z",
    stale: false,
    consecutiveFailures: 0,
    retryAt: null,
    nextRefreshAt: "2026-11-07T15:10:00.000Z",
    capabilities,
    windows: [],
    balances: [{ amount: 42.5, currency: "CNY", total: 50, granted: 0, toppedUp: 50 }],
    usage: null,
    presentation: {
      mode: "balance",
      levelPercent: 85,
      metricLabel: "Balance",
      metricValue: "¥42.50",
      health: "safe",
      primaryWindowId: null,
    },
    error: null,
  };
}

function usageState(profiles: ProviderProfile[]): UsageState {
  return {
    profiles,
    snapshots: profiles.map(snapshotFor),
    selectedProfileId: profiles[0]?.id ?? null,
    paused: false,
    updatedAt: "2026-11-07T15:00:00.000Z",
  };
}

function mockBackend(profiles: ProviderProfile[] = [deepseekProfile, kimiProfile]) {
  invokeMock.mockImplementation(async (command: string) => {
    switch (command) {
      case "provider_catalog":
        return catalog;
      case "get_usage_state":
        return usageState(profiles);
      case "get_autostart":
        return false;
      case "provider_has_credential":
        return true;
      case "save_provider_profile":
        return deepseekProfile;
      case "test_provider_profile":
        return snapshotFor(deepseekProfile);
      case "move_provider_profile":
        return profiles;
      case "delete_provider_profile":
        return undefined;
      default:
        throw new Error(`Unexpected command: ${command}`);
    }
  });
}

describe("SettingsApp", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    openUrlMock.mockReset();
    listenMock.mockResolvedValue(() => undefined);
    localStorage.clear();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("loads the provider catalog and opens a new provider draft", async () => {
    mockBackend();
    render(<SettingsApp />);

    expect(await screen.findByText("DeepSeek personal")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /DeepSeek/ }));

    expect(screen.getByRole("heading", { name: "DeepSeek" })).toBeInTheDocument();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(screen.getByText("取得 DeepSeek API Key")).toBeInTheDocument();
    expect(screen.getByLabelText("主要顯示幣種")).toBeInTheDocument();
  });

  it("separates Ollama subscription and balance settings", async () => {
    mockBackend([]);
    render(<SettingsApp />);

    fireEvent.click(await screen.findByRole("button", { name: /Ollama/ }));
    expect(screen.getByLabelText("追蹤方式")).toHaveValue("auto");
    expect(screen.getByLabelText("訂閱警告百分比")).toBeInTheDocument();
    expect(screen.getByLabelText("滿瓶金額")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("追蹤方式"), {
      target: { value: "balance" },
    });
    expect(screen.queryByLabelText("訂閱警告百分比")).toBeNull();
    expect(screen.getByLabelText("滿瓶金額")).toBeInTheDocument();
  });

  it("saves a new provider with the entered API Key", async () => {
    mockBackend([]);
    render(<SettingsApp />);

    fireEvent.click(await screen.findByRole("button", { name: /DeepSeek/ }));
    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "DeepSeek work" },
    });
    fireEvent.change(screen.getByLabelText("API Key"), {
      target: { value: "sk-test-secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("save_provider_profile", {
        profile: expect.objectContaining({
          id: "",
          providerId: "deepseek",
          label: "DeepSeek work",
        }),
        apiKey: "sk-test-secret",
      });
    });
  });

  it("tests a draft without persisting it and shows the result", async () => {
    mockBackend();
    render(<SettingsApp />);

    fireEvent.click(await screen.findByRole("button", { name: /DeepSeek/ }));
    fireEvent.change(screen.getByLabelText("API Key"), {
      target: { value: "sk-test-secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Test Connection" }));

    expect(await screen.findByText("連線成功")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith(
      "test_provider_profile",
      expect.objectContaining({ apiKey: "sk-test-secret" }),
    );
    expect(
      invokeMock.mock.calls.some(([command]) => command === "save_provider_profile"),
    ).toBe(false);
  });

  it("moves a profile in the provider switch order", async () => {
    mockBackend();
    render(<SettingsApp />);

    await screen.findByText("DeepSeek personal");
    fireEvent.click(screen.getAllByRole("button", { name: "下移" })[0]);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("move_provider_profile", {
        profileId: "deepseek-profile",
        direction: 1,
      });
    });
  });

  it("deletes a profile only after confirmation", async () => {
    mockBackend();
    vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<SettingsApp />);

    await screen.findByText("DeepSeek personal");
    fireEvent.click(screen.getAllByRole("button", { name: "Delete" })[0]);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("delete_provider_profile", {
        profileId: "deepseek-profile",
      });
    });
  });
});
