import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type {
  ProviderDefinition,
  ProviderSnapshot,
  UsageState,
} from "./providerTypes";

const startDragging = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
const invokeMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() =>
  vi.fn().mockResolvedValue(() => undefined),
);
const currentMonitorMock = vi.hoisted(() =>
  vi.fn().mockResolvedValue({
    position: { x: 0, y: 0 },
    size: { width: 1920, height: 1080 },
    scaleFactor: 1,
  }),
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/window", () => ({
  currentMonitor: currentMonitorMock,
  getCurrentWindow: () => ({
    onMoved: vi.fn().mockResolvedValue(() => undefined),
    outerPosition: vi.fn().mockResolvedValue({ x: 1500, y: 700 }),
    outerSize: vi.fn().mockResolvedValue({ width: 76, height: 76 }),
    setPosition: vi.fn().mockResolvedValue(undefined),
    startDragging,
  }),
}));

const capabilities = {
  balance: false,
  quota: true,
  resetTime: true,
  perKeyLimit: false,
  monthlyCredits: true,
  multiCurrency: false,
  usageOnly: false,
};

const catalog: ProviderDefinition[] = [
  {
    id: "ollama",
    label: "OLLAMA",
    description: "Ollama Cloud quota",
    keyUrl: "https://ollama.com/settings/keys",
    icon: "/providers/ollama.png",
    brandColor: "#ffffff",
    adapter: "ollama",
    authLabel: "Ollama API Key",
    support: "native",
    defaultRefreshSeconds: 300,
    requiresTeamId: false,
    capabilities,
  },
  {
    id: "kimi",
    label: "KIMI",
    description: "Kimi balance",
    keyUrl: "https://platform.moonshot.ai/console/api-keys",
    icon: "/providers/kimi.png",
    brandColor: "#111111",
    adapter: "moonshot",
    authLabel: "Kimi API Key",
    support: "native",
    defaultRefreshSeconds: 300,
    requiresTeamId: false,
    capabilities: { ...capabilities, balance: true, quota: false },
  },
];

function snapshot(
  profileId: string,
  providerId: string,
  label: string,
  overrides: Partial<ProviderSnapshot> = {},
): ProviderSnapshot {
  return {
    providerId,
    profileId,
    type: providerId === "kimi" ? "moonshot" : "ollama",
    trackingMode: providerId === "kimi" ? "balance" : "auto",
    label,
    status: "ok",
    fetchedAt: "2026-11-07T12:00:00Z",
    lastSuccessAt: "2026-11-07T12:00:00Z",
    stale: false,
    consecutiveFailures: 0,
    retryAt: null,
    nextRefreshAt: "2026-11-07T12:05:00Z",
    capabilities,
    windows: [
      {
        id: "monthly",
        label: "每月額度",
        usedPercent: 18,
        remainingPercent: 82,
        resetsAt: "2026-12-01T00:00:00Z",
        detail: null,
      },
    ],
    balances: [],
    usage: null,
    presentation: {
      mode: "quota",
      levelPercent: 82,
      metricLabel: "每月額度",
      metricValue: "82.0%",
      health: "safe",
      primaryWindowId: "monthly",
    },
    error: null,
    ...overrides,
  };
}

function usageState(): UsageState {
  return {
    profiles: [
      {
        id: "ollama-profile",
        providerId: "ollama",
        type: "ollama",
        label: "OLLAMA",
        enabled: true,
        refreshSeconds: 300,
        trackingMode: "auto",
        subscriptionWarningPercent: 20,
        subscriptionCriticalPercent: 5,
        fullBottleAmount: null,
        warningThreshold: 20,
        criticalThreshold: 5,
        primaryCurrency: null,
        teamId: null,
        custom: null,
      },
      {
        id: "kimi-profile",
        providerId: "kimi",
        type: "moonshot",
        label: "KIMI",
        enabled: true,
        refreshSeconds: 300,
        trackingMode: "balance",
        subscriptionWarningPercent: 20,
        subscriptionCriticalPercent: 5,
        fullBottleAmount: 50,
        warningThreshold: 20,
        criticalThreshold: 5,
        primaryCurrency: "USD",
        teamId: null,
        custom: null,
      },
    ],
    snapshots: [
      snapshot("ollama-profile", "ollama", "OLLAMA"),
      snapshot("kimi-profile", "kimi", "KIMI"),
    ],
    selectedProfileId: "ollama-profile",
    paused: false,
    updatedAt: "2026-11-07T12:00:00Z",
  };
}

let currentUsageState: UsageState;

describe("App", () => {
  beforeEach(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    currentUsageState = usageState();
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === "provider_catalog") return Promise.resolve(catalog);
      if (command === "get_usage_state") {
        return Promise.resolve(currentUsageState);
      }
      if (command === "select_provider_profile") {
        currentUsageState = {
          ...currentUsageState,
          selectedProfileId: String(args?.profileId ?? ""),
        };
        return Promise.resolve(undefined);
      }
      if (command === "set_hud_expanded") {
        return Promise.resolve("above-left");
      }
      return Promise.resolve(undefined);
    });
    startDragging.mockClear();
  });

  afterEach(() => {
    document.body.innerHTML = "";
    localStorage.clear();
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  });

  it("renders the real provider state with a static capacity orb", async () => {
    render(<App />);

    expect(await screen.findByText("OLLAMA")).toBeInTheDocument();
    expect(document.querySelector(".capacity-orb")).not.toBeNull();
    expect(document.querySelector(".liquid")).toBeNull();
    expect(document.querySelector(".wave")).toBeNull();
    expect(document.querySelector(".bubble")).toBeNull();
  });

  it("starts a native window drag from the capacity orb", async () => {
    render(<App />);

    fireEvent.pointerDown(
      await screen.findByRole("button", { name: /OLLAMA 用量/i }),
      { button: 0 },
    );

    await waitFor(() => {
      expect(startDragging).toHaveBeenCalledOnce();
    });
  });

  it("opens the dedicated Settings window", async () => {
    render(<App />);
    await screen.findByText("OLLAMA");

    fireEvent.mouseEnter(document.querySelector(".hud")!);
    fireEvent.click(screen.getByRole("button", { name: "設定" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("open_settings_window");
    });
  });

  it("switches between real provider profiles and their official icons", async () => {
    render(<App />);
    await screen.findByText("OLLAMA");

    fireEvent.mouseEnter(document.querySelector(".hud")!);
    fireEvent.click(screen.getByRole("button", { name: "下一個供應商" }));

    expect(await screen.findByText("KIMI")).toBeInTheDocument();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("select_provider_profile", {
        profileId: "kimi-profile",
      });
    });
    const headingIcon = document.querySelector(
      ".heading-icon img",
    ) as HTMLImageElement | null;
    expect(headingIcon?.src).toContain("/providers/kimi.png");
  });

  it("shows an actionable error while retaining last-known rows", async () => {
    currentUsageState = usageState();
    currentUsageState.snapshots[0] = snapshot(
      "ollama-profile",
      "ollama",
      "OLLAMA",
      {
        status: "auth_error",
        stale: true,
        presentation: {
          mode: "error",
          levelPercent: 82,
          metricLabel: "每月額度",
          metricValue: "82.0%",
          health: "error",
          primaryWindowId: "monthly",
        },
        error: {
          code: "auth_error",
          message: "API Key 無效",
          action: "請到 Settings 更新 API Key",
          retryAfterSeconds: 1800,
        },
      },
    );

    render(<App />);
    await screen.findByText("OLLAMA");
    fireEvent.mouseEnter(document.querySelector(".hud")!);

    expect(screen.getByText("API Key 無效")).toBeInTheDocument();
    expect(screen.getByText("請到 Settings 更新 API Key")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry" })).toBeInTheDocument();
    expect(screen.getByText(/剩餘/)).toBeInTheDocument();
  });

  it("ignores a snapshot that belongs to another provider", async () => {
    currentUsageState = usageState();
    currentUsageState.snapshots[0] = snapshot(
      "ollama-profile",
      "deepseek",
      "OLLAMA",
    );

    render(<App />);
    await screen.findByText("OLLAMA");
    fireEvent.mouseEnter(document.querySelector(".hud")!);

    expect(screen.getByText("資料對應中")).toBeInTheDocument();
    expect(screen.queryByText("82.0% 剩餘")).toBeNull();
  });

  it("auto-refreshes stale data when the information card opens", async () => {
    render(<App />);
    await screen.findByText("OLLAMA");

    fireEvent.mouseEnter(document.querySelector(".hud")!);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("refresh_provider_profile", {
        profileId: "ollama-profile",
      });
    });
  });

  it("shows only the metric family selected for Ollama", async () => {
    currentUsageState = usageState();
    currentUsageState.profiles[0].trackingMode = "balance";
    currentUsageState.snapshots[0].trackingMode = "balance";
    currentUsageState.snapshots[0].balances = [
      {
        amount: 3.36,
        currency: "USD",
        total: null,
        granted: 0,
        toppedUp: 3.36,
      },
    ];

    render(<App />);
    await screen.findByText("OLLAMA");
    fireEvent.mouseEnter(document.querySelector(".hud")!);

    expect(screen.getByText("加購餘額 · USD")).toBeInTheDocument();
    expect(screen.queryByText(/訂閱額度/)).toBeNull();
  });
});
