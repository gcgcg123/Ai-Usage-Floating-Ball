import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { nearestEdgePosition } from "./windowSnap";
import { useUsageState } from "./useUsageState";
import type {
  BalanceEntry,
  Health,
  ProviderDefinition,
  ProviderSnapshot,
} from "./providerTypes";
import "./App.css";

type Placement =
  | "above-left"
  | "above-right"
  | "below-left"
  | "below-right";

type MetricRow = {
  label: string;
  value: string;
  detail?: string;
  progress?: number;
};

function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

const fallbackIcons: Record<string, string> = {
  ollama: "/providers/ollama.png",
  deepseek: "/providers/deepseek.png",
  openrouter: "/providers/openrouter.png",
  kimi: "/providers/kimi.png",
  "kimi-cn": "/providers/kimi.png",
  xai: "/providers/xai.png",
  qwen: "/providers/qwen.png",
  groq: "/providers/groq.png",
  mistral: "/providers/mistral.png",
  together: "/providers/together.png",
  custom: "/providers/generic.svg",
};

function clampOpacity(value: number) {
  return Math.max(10, Math.min(100, Math.round(value / 10) * 10));
}

function formatMoney(balance: BalanceEntry) {
  return formatCurrencyAmount(balance.amount, balance.currency);
}

function formatCurrencyAmount(amount: number, currency: string) {
  const symbol =
    currency.toUpperCase() === "CNY"
      ? "¥"
      : currency.toUpperCase() === "USD"
        ? "$"
        : "";
  return `${symbol}${amount.toFixed(2)} ${currency}`;
}

function formatRelativeTime(value: string | null, now: number) {
  if (!value) return "尚未成功更新";
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) return "更新時間未知";
  const seconds = Math.max(0, Math.round((now - timestamp) / 1000));
  if (seconds < 60) return `${seconds} 秒前`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} 分鐘前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小時前`;
  return `${Math.floor(hours / 24)} 天前`;
}

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-HK", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function snapshotRows(snapshot: ProviderSnapshot | undefined): MetricRow[] {
  if (!snapshot) return [];
  if (snapshot.windows.length > 0) {
    const subscriptionRows =
      snapshot.trackingMode === "balance"
        ? []
        : snapshot.windows.slice(0, 3).map((window) => ({
            label: `訂閱額度 · ${window.label}`,
            value:
              window.remainingPercent != null
                ? `${window.remainingPercent.toFixed(1)}% 剩餘`
                : window.usedPercent != null
                  ? `${window.usedPercent.toFixed(1)}% 已用`
                  : window.detail ?? "—",
            detail: window.resetsAt
              ? `${formatDate(window.resetsAt)} 重置`
              : window.detail ?? undefined,
            progress: window.remainingPercent ?? undefined,
          }));
    const balanceRows =
      snapshot.trackingMode === "subscription"
        ? []
        : snapshot.balances.slice(0, 3).map((balance) => ({
            label: `加購餘額 · ${balance.currency}`,
            value: formatMoney(balance),
            detail:
              balance.granted != null || balance.toppedUp != null
                ? `贈金 ${formatCurrencyAmount(balance.granted ?? 0, balance.currency)} · 充值 ${formatCurrencyAmount(balance.toppedUp ?? 0, balance.currency)}`
                : undefined,
          }));
    return [...subscriptionRows, ...balanceRows];
  }
  if (snapshot.balances.length > 0) {
    return snapshot.balances.slice(0, 3).map((balance) => ({
      label: `餘額 · ${balance.currency}`,
      value: formatMoney(balance),
      detail:
        balance.granted != null || balance.toppedUp != null
          ? `贈金 ${formatCurrencyAmount(balance.granted ?? 0, balance.currency)} · 充值 ${formatCurrencyAmount(balance.toppedUp ?? 0, balance.currency)}`
          : undefined,
    }));
  }
  if (snapshot.usage) {
    return [
      {
        label: "今日用量",
        value:
          snapshot.usage.daily != null
            ? snapshot.usage.daily.toFixed(2)
            : "—",
      },
      {
        label: "本月用量",
        value:
          snapshot.usage.monthly != null
            ? snapshot.usage.monthly.toFixed(2)
            : "—",
      },
    ];
  }
  return [];
}

function App() {
  const { state, loading, error: stateError, selectProfile, refreshProfile } =
    useUsageState();
  const [catalog, setCatalog] = useState<ProviderDefinition[]>([]);
  const [expanded, setExpanded] = useState(() => {
    const demo =
      !isTauriRuntime() &&
      new URLSearchParams(window.location.search).get("demo") === "1";
    return (
      demo &&
      new URLSearchParams(window.location.search).get("expanded") === "1"
    );
  });
  const [placement, setPlacement] = useState<Placement>("above-left");
  const [windowOpacity, setWindowOpacity] = useState(() => {
    const stored = Number(localStorage.getItem("window-opacity"));
    return Number.isFinite(stored) && stored > 0 ? clampOpacity(stored) : 100;
  });
  const [refreshing, setRefreshing] = useState(false);
  const [now, setNow] = useState(Date.now());
  const hideTimer = useRef<number | null>(null);
  const snapTimer = useRef<number | null>(null);
  const adjustingWindow = useRef(false);
  const snappingWindow = useRef(false);
  const draggingWindow = useRef(false);
  const healthRef = useRef<Record<string, Health>>({});
  const lastHoverRefresh = useRef<Record<string, number>>({});

  useEffect(() => {
    if (isTauriRuntime()) {
      invoke<ProviderDefinition[]>("provider_catalog")
        .then(setCatalog)
        .catch(console.error);
      const dock = localStorage.getItem("hud-dock") ?? "none";
      invoke("set_hud_expanded", { expanded: false })
        .then(() => applyDock(dock))
        .catch(console.error);
    }
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 30_000);
    const onStorage = (event: StorageEvent) => {
      if (event.key === "window-opacity" && event.newValue) {
        const next = Number(event.newValue);
        if (Number.isFinite(next)) setWindowOpacity(clampOpacity(next));
      }
      if (event.key === "hud-dock" && event.newValue && isTauriRuntime()) {
        applyDock(event.newValue);
      }
    };
    window.addEventListener("storage", onStorage);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("storage", onStorage);
    };
  }, []);

  function applyDock(dock: string) {
    if (!isTauriRuntime()) return;
    adjustingWindow.current = true;
    invoke("set_hud_dock", { dock })
      .catch(console.error)
      .finally(() => {
        window.setTimeout(() => {
          adjustingWindow.current = false;
        }, 200);
      });
  }

  useEffect(() => {
    if (!state || localStorage.getItem("notifications-enabled") === "0") return;
    const previous = healthRef.current;
    for (const snapshot of state.snapshots) {
      const health = snapshot.presentation.health;
      if (
        (health === "warning" || health === "critical") &&
        previous[snapshot.profileId] !== health
      ) {
        void sendHealthAlert(snapshot);
      }
      previous[snapshot.profileId] = health;
    }
  }, [state]);

  async function sendHealthAlert(snapshot: ProviderSnapshot) {
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        granted = (await requestPermission()) === "granted";
      }
      if (!granted) return;
      const level =
        snapshot.presentation.health === "critical" ? "危險水平" : "警示水平";
      sendNotification({
        title: "AI Usage Floating Ball",
        body: `${snapshot.label} ${level}：${snapshot.presentation.metricLabel} ${snapshot.presentation.metricValue}`,
      });
    } catch (notificationError) {
      console.error(notificationError);
    }
  }

  useEffect(() => {
    if (!isTauriRuntime()) return;
    const hudWindow = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void hudWindow
      .onMoved(() => {
        if (
          disposed ||
          adjustingWindow.current ||
          snappingWindow.current
        ) {
          return;
        }
        if (snapTimer.current !== null) window.clearTimeout(snapTimer.current);
        snapTimer.current = window.setTimeout(async () => {
          try {
            const [position, size, monitor] = await Promise.all([
              hudWindow.outerPosition(),
              hudWindow.outerSize(),
              currentMonitor(),
            ]);
            if (!monitor || disposed) return;
            const target = nearestEdgePosition({
              position,
              size,
              monitor: {
                x: monitor.position.x,
                y: monitor.position.y,
                width: monitor.size.width,
                height: monitor.size.height,
              },
              padding: Math.round(6 * monitor.scaleFactor),
              bottomPadding: Math.round(52 * monitor.scaleFactor),
            });
            if (target.x !== position.x || target.y !== position.y) {
              snappingWindow.current = true;
              await hudWindow.setPosition(
                new PhysicalPosition(target.x, target.y),
              );
              window.setTimeout(() => {
                snappingWindow.current = false;
              }, 150);
            }
          } catch (snapError) {
            snappingWindow.current = false;
            console.error(snapError);
          }
        }, 350);
      })
      .then((disposeListener) => {
        unlisten = disposeListener;
      })
      .catch(console.error);

    return () => {
      disposed = true;
      if (snapTimer.current !== null) window.clearTimeout(snapTimer.current);
      unlisten?.();
    };
  }, []);

  const definitions = useMemo(
    () => new Map(catalog.map((definition) => [definition.id, definition])),
    [catalog],
  );
  const profiles = state?.profiles ?? [];
  const selectedProfile =
    profiles.find((profile) => profile.id === state?.selectedProfileId) ??
    profiles[0];
  const definition = selectedProfile
    ? definitions.get(selectedProfile.providerId)
    : undefined;
  const snapshotCandidate = selectedProfile
    ? state?.snapshots.find(
        (candidate) => candidate.profileId === selectedProfile.id,
      )
    : undefined;
  const snapshot =
    snapshotCandidate &&
    selectedProfile &&
    snapshotCandidate.profileId === selectedProfile.id &&
    snapshotCandidate.providerId ===
      (selectedProfile.providerId || selectedProfile.type) &&
    snapshotCandidate.trackingMode === selectedProfile.trackingMode
      ? snapshotCandidate
      : undefined;
  const providerIcon =
    definition?.icon ??
    (selectedProfile ? fallbackIcons[selectedProfile.providerId] : undefined) ??
    "/providers/generic.svg";
  const providerLabel =
    selectedProfile?.label ?? definition?.label ?? "尚未設定 Provider";
  const health: Health = snapshot?.presentation.health ?? "unknown";
  const level = snapshot?.presentation.levelPercent ?? 0;
  const metric = snapshot?.presentation.metricValue ?? (loading ? "SYNC" : "--");
  const rows = snapshotRows(snapshot);
  const hasError = Boolean(snapshot?.error);
  const snapshotMismatch = Boolean(snapshotCandidate && !snapshot);
  const canCycle = profiles.filter((profile) => profile.enabled).length > 1;
  const ringRadius = 31;
  const ringCircumference = 2 * Math.PI * ringRadius;

  async function setWindowExpanded(next: boolean, waitForPaint = true) {
    setExpanded(next);
    if (!isTauriRuntime()) return;
    adjustingWindow.current = true;
    if (!next && waitForPaint) {
      await new Promise<void>((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
      });
    }
    const nextPlacement = await invoke<string>("set_hud_expanded", {
      expanded: next,
    }).catch((expandError) => {
      console.error(expandError);
      return null;
    });
    if (
      nextPlacement === "above-left" ||
      nextPlacement === "above-right" ||
      nextPlacement === "below-left" ||
      nextPlacement === "below-right"
    ) {
      setPlacement(nextPlacement);
    }
    window.setTimeout(() => {
      adjustingWindow.current = false;
    }, 180);
  }

  function clearHideTimer() {
    if (hideTimer.current !== null) {
      window.clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
  }

  function handleEnter() {
    if (draggingWindow.current) return;
    clearHideTimer();
    if (!expanded) void setWindowExpanded(true);
    if (selectedProfile) {
      const lastSuccess = snapshot?.lastSuccessAt
        ? Date.parse(snapshot.lastSuccessAt)
        : 0;
      const now = Date.now();
      const lastAttempt = lastHoverRefresh.current[selectedProfile.id] ?? 0;
      if (
        now - (Number.isFinite(lastSuccess) ? lastSuccess : 0) > 30_000 &&
        now - lastAttempt > 15_000
      ) {
        lastHoverRefresh.current[selectedProfile.id] = now;
        void refreshCurrent();
      }
    }
  }

  function handleLeave() {
    if (draggingWindow.current) return;
    clearHideTimer();
    hideTimer.current = window.setTimeout(() => {
      void setWindowExpanded(false);
    }, 450);
  }

  async function selectAdjacent(direction: -1 | 1) {
    const enabled = profiles.filter((profile) => profile.enabled);
    if (enabled.length < 2) return;
    const currentIndex = Math.max(
      0,
      enabled.findIndex((profile) => profile.id === selectedProfile?.id),
    );
    const next = enabled[(currentIndex + direction + enabled.length) % enabled.length];
    await selectProfile(next.id);
  }

  async function refreshCurrent() {
    if (!selectedProfile || refreshing) return;
    setRefreshing(true);
    try {
      await refreshProfile(selectedProfile.id);
    } catch (refreshError) {
      console.error(refreshError);
    } finally {
      setRefreshing(false);
      setNow(Date.now());
    }
  }

  async function beginWindowDrag(
    event: React.PointerEvent<HTMLButtonElement>,
  ) {
    if (event.button !== 0) return;
    event.preventDefault();
    draggingWindow.current = true;
    clearHideTimer();
    try {
      await setWindowExpanded(false, false);
      adjustingWindow.current = false;
      await getCurrentWindow().startDragging();
    } catch (dragError) {
      console.error(dragError);
    } finally {
      window.setTimeout(() => {
        draggingWindow.current = false;
      }, 1800);
    }
  }

  return (
    <main
      className={`hud placement-${placement} ${expanded ? "is-expanded" : "is-collapsed"} status-${health}`}
      style={{ opacity: windowOpacity / 100 }}
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
    >
      <section
        className="info-card"
        aria-hidden={!expanded}
        key={selectedProfile?.id ?? "empty"}
      >
        <header className="card-header">
          <button
            className="arrow-button"
            aria-label="上一個供應商"
            disabled={!canCycle}
            onClick={() => void selectAdjacent(-1)}
          >
            ‹
          </button>
          <div className="provider-heading">
            <span className="heading-icon">
              <img src={providerIcon} alt="" draggable={false} />
            </span>
            <span>{providerLabel}</span>
          </div>
          <button
            className="arrow-button"
            aria-label="下一個供應商"
            disabled={!canCycle}
            onClick={() => void selectAdjacent(1)}
          >
            ›
          </button>
          <span
            className={`status-badge ${snapshot?.stale ? "is-stale" : ""} ${
              hasError ? "is-error" : ""
            }`}
          >
            {!selectedProfile
              ? "SETUP"
              : hasError
                ? "ERROR"
                : snapshot?.stale
                  ? "STALE"
                  : "LIVE"}
          </span>
          <button
            className="utility-button"
            aria-label="設定"
            onClick={() => {
              if (isTauriRuntime()) {
                invoke("open_settings_window").catch(console.error);
              }
            }}
          >
            ⚙
          </button>
        </header>

        {!selectedProfile ? (
          <div className="empty-provider">
            <strong>尚未加入供應商</strong>
            <p>在 Settings 選擇 Provider、輸入 API Key 並測試連線。</p>
            <button
              className="primary-button"
              onClick={() => {
                if (isTauriRuntime()) {
                  invoke("open_settings_window").catch(console.error);
                }
              }}
            >
              開啟 Settings
            </button>
          </div>
        ) : (
          <div className="provider-body">
            {hasError && (
              <div className="error-banner">
                <div>
                  <strong>{snapshot?.error?.message ?? "無法讀取資料"}</strong>
                  <span>{snapshot?.error?.action}</span>
                </div>
                <button onClick={() => void refreshCurrent()}>Retry</button>
              </div>
            )}
            {rows.length > 0 ? (
              rows.map((row, index) => (
                <div
                  className="metric-row"
                  key={`${selectedProfile?.id ?? "empty"}-${index}-${row.label}`}
                >
                  <div className="metric-line">
                    <span>{row.label}</span>
                    <strong>{row.value}</strong>
                  </div>
                  {row.progress !== undefined && (
                    <div className="progress-track">
                      <span
                        className="progress-fill"
                        style={{
                          width: `${Math.max(0, Math.min(100, row.progress))}%`,
                        }}
                      />
                    </div>
                  )}
                  {row.detail && <p className="metric-detail">{row.detail}</p>}
                </div>
              ))
            ) : (
              !hasError && (
                <div className="empty-provider compact">
                  <strong>
                    {snapshotMismatch
                      ? "資料對應中"
                      : loading
                        ? "正在同步"
                        : "等待第一次更新"}
                  </strong>
                  <p>
                    {snapshotMismatch
                      ? "已忽略不屬於目前供應商的舊快取，正在重新整理。"
                      : (stateError ?? "背景監測會在幾秒內完成。")}
                  </p>
                </div>
              )
            )}
          </div>
        )}

        <footer className="card-footer">
          <span title={snapshot?.lastSuccessAt ?? undefined}>
            更新：{formatRelativeTime(snapshot?.lastSuccessAt ?? null, now)}
          </span>
          <button
            className="refresh-button"
            aria-label="重新整理"
            disabled={!selectedProfile || refreshing}
            onClick={() => void refreshCurrent()}
          >
            {refreshing ? "…" : "↻"}
          </button>
        </footer>
      </section>

      <button
        className="capacity-orb"
        aria-label={`${providerLabel} 用量 ${metric}`}
        data-level={level}
        onPointerDown={(event) => void beginWindowDrag(event)}
      >
        <svg className="capacity-ring" viewBox="0 0 72 72" aria-hidden="true">
          <circle className="capacity-track" cx="36" cy="36" r={ringRadius} />
          <circle
            className="capacity-progress"
            cx="36"
            cy="36"
            r={ringRadius}
            strokeDasharray={`${ringCircumference} ${ringCircumference}`}
            strokeDashoffset={
              ringCircumference - (level / 100) * ringCircumference
            }
          />
        </svg>
        <span className="orb-center">
          <img className="provider-icon" src={providerIcon} alt="" draggable={false} />
        </span>
        <span className="orb-value">{metric}</span>
      </button>
    </main>
  );
}

export default App;
