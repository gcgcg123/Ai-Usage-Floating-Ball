import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { demoCatalog, demoUsageState } from "./demoState";
import type {
  ProviderDefinition,
  ProviderProfile,
  ProviderSnapshot,
  ProviderSupport,
  TrackingMode,
  UsageState,
} from "./providerTypes";
import "./SettingsApp.css";

const demoMode =
  !("__TAURI_INTERNALS__" in window) &&
  new URLSearchParams(window.location.search).get("demo") === "1";

type SettingsDraft = {
  id: string;
  providerId: string;
  label: string;
  apiKey: string;
  enabled: boolean;
  refreshSeconds: number;
  trackingMode: TrackingMode;
  subscriptionWarningPercent: string;
  subscriptionCriticalPercent: string;
  primaryCurrency: string;
  fullBottleAmount: string;
  warningThreshold: string;
  criticalThreshold: string;
  teamId: string;
  customUrl: string;
  customAuthHeader: string;
  customAuthScheme: "bearer" | "raw";
  remainingPath: string;
  limitPath: string;
  usedPath: string;
  currencyPath: string;
  resetsAtPath: string;
};

const supportLabels: Record<ProviderSupport, string> = {
  native: "官方 API",
  management_key: "需要 Management Key",
  inference_usage_only: "Inference usage only",
  custom_only: "Custom endpoint",
};

const statusLabels: Record<string, string> = {
  ok: "正常",
  auth_error: "API Key 無效",
  permission_error: "權限不足",
  rate_limited: "已限流",
  insufficient_balance: "餘額不足",
  provider_error: "供應商錯誤",
  schema_error: "格式錯誤",
  offline: "離線",
  timeout: "逾時",
  disabled: "已停用",
  unknown: "未更新",
};

function numberOrNull(value: string) {
  if (value.trim() === "") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function createDraft(definition: ProviderDefinition): SettingsDraft {
  return {
    id: "",
    providerId: definition.id,
    label: definition.label,
    apiKey: "",
    enabled: true,
    refreshSeconds: definition.defaultRefreshSeconds,
    trackingMode:
      definition.adapter === "ollama" || definition.adapter === "custom"
        ? "auto"
        : "balance",
    subscriptionWarningPercent: "20",
    subscriptionCriticalPercent: "5",
    primaryCurrency: "",
    fullBottleAmount:
      definition.capabilities.balance && !definition.capabilities.quota
        ? "50"
        : "",
    warningThreshold: "20",
    criticalThreshold: "5",
    teamId: "",
    customUrl: "",
    customAuthHeader: "Authorization",
    customAuthScheme: "bearer",
    remainingPath: "data.remaining",
    limitPath: "data.limit",
    usedPath: "data.used",
    currencyPath: "data.currency",
    resetsAtPath: "data.reset_at",
  };
}

function editDraft(profile: ProviderProfile): SettingsDraft {
  return {
    id: profile.id,
    providerId: profile.providerId,
    label: profile.label,
    apiKey: "",
    enabled: profile.enabled,
    refreshSeconds: profile.refreshSeconds,
    trackingMode: profile.trackingMode ?? "auto",
    subscriptionWarningPercent:
      profile.subscriptionWarningPercent?.toString() ?? "20",
    subscriptionCriticalPercent:
      profile.subscriptionCriticalPercent?.toString() ?? "5",
    primaryCurrency: profile.primaryCurrency ?? "",
    fullBottleAmount: profile.fullBottleAmount?.toString() ?? "",
    warningThreshold: profile.warningThreshold?.toString() ?? "",
    criticalThreshold: profile.criticalThreshold?.toString() ?? "",
    teamId: profile.teamId ?? "",
    customUrl: profile.custom?.url ?? "",
    customAuthHeader: profile.custom?.authHeader ?? "Authorization",
    customAuthScheme: profile.custom?.authScheme ?? "bearer",
    remainingPath: profile.custom?.remainingPath ?? "",
    limitPath: profile.custom?.limitPath ?? "",
    usedPath: profile.custom?.usedPath ?? "",
    currencyPath: profile.custom?.currencyPath ?? "",
    resetsAtPath: profile.custom?.resetsAtPath ?? "",
  };
}

function profileFromDraft(
  draft: SettingsDraft,
  definition: ProviderDefinition | undefined,
  existing: ProviderProfile | undefined,
): ProviderProfile {
  const isCustom = definition?.adapter === "custom" || existing?.type === "custom";
  return {
    id: draft.id,
    providerId: draft.providerId,
    type: definition?.adapter ?? existing?.type ?? "custom",
    label: draft.label.trim() || definition?.label || "Provider",
    enabled: draft.enabled,
    refreshSeconds: draft.refreshSeconds,
    trackingMode: draft.trackingMode,
    subscriptionWarningPercent: numberOrNull(draft.subscriptionWarningPercent),
    subscriptionCriticalPercent: numberOrNull(draft.subscriptionCriticalPercent),
    fullBottleAmount: numberOrNull(draft.fullBottleAmount),
    warningThreshold: numberOrNull(draft.warningThreshold),
    criticalThreshold: numberOrNull(draft.criticalThreshold),
    primaryCurrency: draft.primaryCurrency.trim().toUpperCase() || null,
    teamId: draft.teamId.trim() || null,
    custom: isCustom
      ? {
          url: draft.customUrl.trim() || null,
          authHeader: draft.customAuthHeader.trim() || "Authorization",
          authScheme: draft.customAuthScheme,
          remainingPath: draft.remainingPath.trim() || null,
          limitPath: draft.limitPath.trim() || null,
          usedPath: draft.usedPath.trim() || null,
          currencyPath: draft.currencyPath.trim() || null,
          resetsAtPath: draft.resetsAtPath.trim() || null,
        }
      : null,
  };
}

export default function SettingsApp() {
  const [view, setView] = useState<"providers" | "appearance" | "notifications">(
    "providers",
  );
  const [catalog, setCatalog] = useState<ProviderDefinition[]>([]);
  const [usageState, setUsageState] = useState<UsageState | null>(null);
  const [credentialMap, setCredentialMap] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [draft, setDraft] = useState<SettingsDraft | null>(null);
  const [testResult, setTestResult] = useState<ProviderSnapshot | null>(null);
  const [testError, setTestError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [windowOpacity, setWindowOpacity] = useState(() => {
    const stored = Number(localStorage.getItem("window-opacity"));
    return Number.isFinite(stored) && stored > 0 ? stored : 100;
  });
  const [dock, setDock] = useState(() => localStorage.getItem("hud-dock") ?? "none");
  const [autostart, setAutostart] = useState(false);
  const [notificationsEnabled, setNotificationsEnabled] = useState(
    () => localStorage.getItem("notifications-enabled") !== "0",
  );

  const load = useCallback(async () => {
    if (demoMode) {
      setCatalog(demoCatalog);
      setUsageState(demoUsageState());
      setCredentialMap({
        "demo-ollama": true,
        "demo-deepseek": true,
      });
      setLoadError(null);
      setLoading(false);
      return;
    }
    try {
      const [nextCatalog, nextState] = await Promise.all([
        invoke<ProviderDefinition[]>("provider_catalog"),
        invoke<UsageState>("get_usage_state"),
      ]);
      const credentialEntries = await Promise.all(
        nextState.profiles.map(async (profile) => {
          const hasCredential = await invoke<boolean>(
            "provider_has_credential",
            { profileId: profile.id },
          );
          return [profile.id, hasCredential] as const;
        }),
      );
      setCatalog(nextCatalog);
      setUsageState(nextState);
      setCredentialMap(Object.fromEntries(credentialEntries));
      setLoadError(null);
    } catch (error) {
      setLoadError(String(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void load();
    if ("__TAURI_INTERNALS__" in window && !demoMode) {
      invoke<boolean>("get_autostart")
        .then(setAutostart)
        .catch((error) => console.error(error));
    }
    if (!demoMode) {
      void listen<UsageState>("provider://state-updated", (event) => {
        if (!disposed) setUsageState(event.payload);
      })
        .then((cleanup) => {
          unlisten = cleanup;
        })
        .catch((error) => {
          if (!disposed) setLoadError(String(error));
        });
    }
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [load]);

  const profiles = usageState?.profiles ?? [];
  const snapshots = usageState?.snapshots ?? [];
  const selectedDefinition = draft
    ? catalog.find((definition) => definition.id === draft.providerId)
    : undefined;
  const selectedProfile = draft
    ? profiles.find((profile) => profile.id === draft.id)
    : undefined;
  const filteredCatalog = useMemo(() => {
    const normalized = search.trim().toLowerCase();
    if (!normalized) return catalog;
    return catalog.filter(
      (definition) =>
        definition.label.toLowerCase().includes(normalized) ||
        definition.description.toLowerCase().includes(normalized) ||
        definition.id.toLowerCase().includes(normalized),
    );
  }, [catalog, search]);

  function snapshotFor(profileId: string) {
    return snapshots.find((snapshot) => snapshot.profileId === profileId);
  }

  async function testDraft() {
    if (!draft) return;
    setBusy(true);
    setTestError(null);
    setTestResult(null);
    try {
      const profile = profileFromDraft(draft, selectedDefinition, selectedProfile);
      const result = await invoke<ProviderSnapshot>("test_provider_profile", {
        profile,
        apiKey: draft.apiKey.trim() || null,
      });
      setTestResult(result);
    } catch (error) {
      setTestError(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function saveDraft() {
    if (!draft) return;
    setBusy(true);
    setTestError(null);
    try {
      const profile = profileFromDraft(draft, selectedDefinition, selectedProfile);
      const saved = await invoke<ProviderProfile>("save_provider_profile", {
        profile,
        apiKey: draft.apiKey.trim() || null,
      });
      setDraft(editDraft(saved));
      setTestResult(null);
      await load();
    } catch (error) {
      setTestError(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function toggleProfile(profile: ProviderProfile) {
    setBusy(true);
    try {
      await invoke("save_provider_profile", {
        profile: { ...profile, enabled: !profile.enabled },
        apiKey: null,
      });
      await load();
    } catch (error) {
      setLoadError(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function moveProfile(profile: ProviderProfile, direction: -1 | 1) {
    setBusy(true);
    try {
      await invoke("move_provider_profile", {
        profileId: profile.id,
        direction,
      });
      await load();
    } catch (error) {
      setLoadError(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function deleteProfile(profile: ProviderProfile) {
    const confirmed = window.confirm(
      `刪除「${profile.label}」？API Key 與本機 Snapshot 會一起移除。`,
    );
    if (!confirmed) return;
    setBusy(true);
    try {
      await invoke("delete_provider_profile", { profileId: profile.id });
      if (draft?.id === profile.id) setDraft(null);
      await load();
    } catch (error) {
      setLoadError(String(error));
    } finally {
      setBusy(false);
    }
  }

  function updateDraft<K extends keyof SettingsDraft>(
    key: K,
    value: SettingsDraft[K],
  ) {
    setDraft((current) => (current ? { ...current, [key]: value } : current));
  }

  function updateDock(value: string) {
    setDock(value);
    localStorage.setItem("hud-dock", value);
  }

  async function toggleAutostart(next: boolean) {
    try {
      const enabled = await invoke<boolean>("set_autostart", { enabled: next });
      setAutostart(enabled);
    } catch (error) {
      setLoadError(String(error));
    }
  }

  async function sendTestNotification() {
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        granted = (await requestPermission()) === "granted";
      }
      if (!granted) {
        setLoadError("通知權限被拒絕，請在 Windows 設定開啟。");
        return;
      }
      sendNotification({
        title: "AI Usage Floating Ball",
        body: "通知功能正常。",
      });
    } catch (error) {
      setLoadError(String(error));
    }
  }

  return (
    <div className="settings-app">
      <aside className="settings-sidebar">
        <div className="settings-brand">
          <img src="/providers/generic.svg" alt="" />
          <div>
            <strong>AI Usage</strong>
            <span>Floating Ball</span>
          </div>
        </div>
        <nav>
          <button
            className={`nav-item ${view === "providers" ? "is-active" : ""}`}
            onClick={() => setView("providers")}
          >
            Providers
          </button>
          <button
            className={`nav-item ${view === "appearance" ? "is-active" : ""}`}
            onClick={() => setView("appearance")}
          >
            Appearance
          </button>
          <button
            className={`nav-item ${view === "notifications" ? "is-active" : ""}`}
            onClick={() => setView("notifications")}
          >
            Notifications
          </button>
        </nav>
        <div className="sidebar-note">
          API Keys 儲存在 Windows Credential Manager。
        </div>
      </aside>

      <main className="settings-main">
        <header className="settings-main-header">
          <div>
            <h1>
              {view === "providers"
                ? "Providers"
                : view === "appearance"
                  ? "Appearance"
                  : "Notifications"}
            </h1>
            <p>
              {view === "providers"
                ? "管理 API Key、顯示順序與刷新設定。"
                : view === "appearance"
                  ? "懸浮球停靠位置、透明度與開機啟動。"
                  : "用量警示與系統通知。"}
            </p>
          </div>
          {view === "providers" && (
            <button
              className="accent-button"
              disabled={busy || Boolean(draft)}
              onClick={() => {
                setSearch("");
                setDraft(null);
                setTestResult(null);
                setTestError(null);
              }}
            >
              {draft ? "正在編輯" : "Provider Catalog"}
            </button>
          )}
        </header>

        {loadError && <div className="settings-error">{loadError}</div>}

        {view === "providers" &&
          (draft ? (
          <section className="provider-editor">
            <div className="editor-heading">
              <button className="ghost-button" onClick={() => setDraft(null)}>
                ‹ Catalogs
              </button>
              <div>
                <h2>{selectedDefinition?.label ?? "Provider"}</h2>
                <p>{selectedDefinition?.description}</p>
              </div>
              <img src={selectedDefinition?.icon ?? "/providers/generic.svg"} alt="" />
            </div>

            <div className="form-grid">
              <label>
                Display name
                <input
                  value={draft.label}
                  onChange={(event) => updateDraft("label", event.target.value)}
                />
              </label>
              <label>
                API Key
                <input
                  type="password"
                  autoComplete="new-password"
                  value={draft.apiKey}
                  placeholder={
                    draft.id && credentialMap[draft.id]
                      ? "已儲存；留空可保留"
                      : "輸入 API Key"
                  }
                  onChange={(event) => updateDraft("apiKey", event.target.value)}
                />
              </label>

              {selectedDefinition?.requiresTeamId && (
                <label>
                  Team ID
                  <input
                    value={draft.teamId}
                    placeholder="xAI Team ID"
                    onChange={(event) =>
                      updateDraft("teamId", event.target.value)
                    }
                  />
                </label>
              )}

              <label>
                自動刷新
                <select
                  value={draft.refreshSeconds}
                  onChange={(event) =>
                    updateDraft("refreshSeconds", Number(event.target.value))
                  }
                >
                  <option value={300}>5 分鐘</option>
                  <option value={600}>10 分鐘</option>
                  <option value={900}>15 分鐘</option>
                </select>
              </label>

              {selectedDefinition?.adapter === "ollama" && (
                <label>
                  追蹤方式
                  <select
                    value={draft.trackingMode}
                    onChange={(event) =>
                      updateDraft(
                        "trackingMode",
                        event.target.value as TrackingMode,
                      )
                    }
                  >
                    <option value="auto">自動（訂閱優先）</option>
                    <option value="subscription">只追蹤訂閱額度</option>
                    <option value="balance">只追蹤加購餘額</option>
                  </select>
                </label>
              )}

              {selectedDefinition?.capabilities.multiCurrency && (
                <label>
                  主要顯示幣種
                  <select
                    value={draft.primaryCurrency}
                    onChange={(event) =>
                      updateDraft("primaryCurrency", event.target.value)
                    }
                  >
                    <option value="">自動選擇</option>
                    <option value="CNY">CNY（人民幣）</option>
                    <option value="USD">USD（美元）</option>
                  </select>
                </label>
              )}

              {selectedDefinition?.adapter === "ollama" &&
                selectedDefinition.capabilities.quota &&
                draft.trackingMode !== "balance" && (
                  <>
                    <label>
                      訂閱警告百分比
                      <input
                        type="number"
                        min="0"
                        max="100"
                        value={draft.subscriptionWarningPercent}
                        onChange={(event) =>
                          updateDraft(
                            "subscriptionWarningPercent",
                            event.target.value,
                          )
                        }
                      />
                    </label>
                    <label>
                      訂閱危險百分比
                      <input
                        type="number"
                        min="0"
                        max="100"
                        value={draft.subscriptionCriticalPercent}
                        onChange={(event) =>
                          updateDraft(
                            "subscriptionCriticalPercent",
                            event.target.value,
                          )
                        }
                      />
                    </label>
                  </>
                )}

              {selectedDefinition?.capabilities.balance &&
                (selectedDefinition.adapter !== "ollama" ||
                  draft.trackingMode !== "subscription") && (
                <>
                  <label>
                    滿瓶金額
                    <input
                      type="number"
                      value={draft.fullBottleAmount}
                      onChange={(event) =>
                        updateDraft("fullBottleAmount", event.target.value)
                      }
                    />
                  </label>
                  <label>
                    警告餘額
                    <input
                      type="number"
                      value={draft.warningThreshold}
                      onChange={(event) =>
                        updateDraft("warningThreshold", event.target.value)
                      }
                    />
                  </label>
                  <label>
                    危險餘額
                    <input
                      type="number"
                      value={draft.criticalThreshold}
                      onChange={(event) =>
                        updateDraft("criticalThreshold", event.target.value)
                      }
                    />
                  </label>
                </>
                )}
            </div>

            {selectedDefinition?.adapter === "custom" && (
              <details className="custom-details" open>
                <summary>Custom API Mapping</summary>
                <div className="form-grid">
                  <label className="span-two">
                    GET endpoint
                    <input
                      value={draft.customUrl}
                      placeholder="https://api.example.com/usage"
                      onChange={(event) =>
                        updateDraft("customUrl", event.target.value)
                      }
                    />
                  </label>
                  <label>
                    Auth header
                    <input
                      value={draft.customAuthHeader}
                      onChange={(event) =>
                        updateDraft("customAuthHeader", event.target.value)
                      }
                    />
                  </label>
                  <label>
                    Auth format
                    <select
                      value={draft.customAuthScheme}
                      onChange={(event) =>
                        updateDraft(
                          "customAuthScheme",
                          event.target.value as "bearer" | "raw",
                        )
                      }
                    >
                      <option value="bearer">Bearer token</option>
                      <option value="raw">Raw value</option>
                    </select>
                  </label>
                  <label>
                    Remaining path
                    <input
                      value={draft.remainingPath}
                      onChange={(event) =>
                        updateDraft("remainingPath", event.target.value)
                      }
                    />
                  </label>
                  <label>
                    Limit path
                    <input
                      value={draft.limitPath}
                      onChange={(event) =>
                        updateDraft("limitPath", event.target.value)
                      }
                    />
                  </label>
                  <label>
                    Used path
                    <input
                      value={draft.usedPath}
                      onChange={(event) =>
                        updateDraft("usedPath", event.target.value)
                      }
                    />
                  </label>
                  <label>
                    Currency path
                    <input
                      value={draft.currencyPath}
                      onChange={(event) =>
                        updateDraft("currencyPath", event.target.value)
                      }
                    />
                  </label>
                  <label>
                    Reset path
                    <input
                      value={draft.resetsAtPath}
                      onChange={(event) =>
                        updateDraft("resetsAtPath", event.target.value)
                      }
                    />
                  </label>
                </div>
              </details>
            )}

            <div className="editor-actions">
              {selectedDefinition?.keyUrl && (
                <button
                  className="ghost-button"
                  onClick={() => void openUrl(selectedDefinition.keyUrl)}
                >
                  取得 {selectedDefinition.authLabel}
                </button>
              )}
              <span className="action-spacer" />
              <button
                className="ghost-button"
                disabled={busy}
                onClick={() => void testDraft()}
              >
                {busy ? "處理中…" : "Test Connection"}
              </button>
              <button
                className="accent-button"
                disabled={busy}
                onClick={() => void saveDraft()}
              >
                Save
              </button>
            </div>

            {testError && <div className="settings-error">{testError}</div>}
            {testResult && (
              <div
                className={`test-result status-${testResult.presentation.health}`}
              >
                <strong>
                  {testResult.status === "ok"
                    ? "連線成功"
                    : statusLabels[testResult.status] ?? testResult.status}
                </strong>
                <span>
                  Presentation mode：{testResult.presentation.mode}
                  {" · "}
                  {testResult.presentation.metricLabel}：{testResult.presentation.metricValue}
                </span>
                {testResult.error && (
                  <small>
                    {testResult.error.message} {testResult.error.action}
                  </small>
                )}
              </div>
            )}
          </section>
        ) : (
          <>
            <div className="catalog-search">
              <input
                value={search}
                placeholder="搜尋 Provider，例如 Kimi、Qwen、Grok…"
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>

            <section className="provider-catalog-grid">
              {filteredCatalog.map((definition) => (
                <button
                  className="catalog-card"
                  key={definition.id}
                  onClick={() => {
                    setDraft(createDraft(definition));
                    setTestResult(null);
                    setTestError(null);
                  }}
                >
                  <img src={definition.icon} alt="" />
                  <span>
                    <strong>{definition.label}</strong>
                    <small>{supportLabels[definition.support]}</small>
                  </span>
                </button>
              ))}
            </section>

            <section className="profiles-section">
              <div className="profiles-heading">
                <h2>已加入的 Providers</h2>
                <span>{profiles.length}</span>
              </div>
              {loading ? (
                <div className="settings-empty">正在讀取設定…</div>
              ) : profiles.length === 0 ? (
                <div className="settings-empty">
                  尚未加入 Provider。從上方 Catalog 選擇後輸入 API Key。
                </div>
              ) : (
                <div className="profile-list">
                  {profiles.map((profile, index) => {
                    const definition = catalog.find(
                      (item) => item.id === profile.providerId,
                    );
                    const snapshot = snapshotFor(profile.id);
                    return (
                      <article
                        className={`profile-card ${
                          usageState?.selectedProfileId === profile.id
                            ? "is-selected"
                            : ""
                        }`}
                        key={profile.id}
                      >
                        <div className="profile-icon">
                          <img
                            src={
                              definition?.icon ??
                              fallbackIcon(profile.providerId)
                            }
                            alt=""
                          />
                        </div>
                        <div className="profile-main">
                          <div className="profile-title">
                            <strong>{profile.label}</strong>
                            <span className={snapshot?.error ? "bad" : "good"}>
                              {snapshot
                                ? statusLabels[snapshot.status] ?? snapshot.status
                                : "未更新"}
                            </span>
                          </div>
                          <small>
                            {definition?.label ?? profile.providerId}
                            {" · "}
                            {profile.enabled ? "Enabled" : "Disabled"}
                            {" · "}
                            {credentialMap[profile.id] ? "API Key 已儲存" : "尚未設定 Key"}
                          </small>
                        </div>
                        <div className="profile-actions">
                          <button
                            disabled={index === 0 || busy}
                            onClick={() => void moveProfile(profile, -1)}
                            aria-label="上移"
                          >
                            ↑
                          </button>
                          <button
                            disabled={index === profiles.length - 1 || busy}
                            onClick={() => void moveProfile(profile, 1)}
                            aria-label="下移"
                          >
                            ↓
                          </button>
                          <button
                            disabled={busy}
                            onClick={() => {
                              setDraft(editDraft(profile));
                              setTestResult(null);
                              setTestError(null);
                            }}
                          >
                            Edit
                          </button>
                          <button
                            disabled={busy}
                            onClick={() => void toggleProfile(profile)}
                          >
                            {profile.enabled ? "Disable" : "Enable"}
                          </button>
                          <button
                            disabled={busy}
                            onClick={() => void deleteProfile(profile)}
                          >
                            Delete
                          </button>
                        </div>
                      </article>
                    );
                  })}
                </div>
              )}
            </section>

          </>
          ))}

        {view === "appearance" && (
          <div className="settings-view">
            <section className="appearance-section">
              <div>
                <h2>懸浮球停靠</h2>
                <p>摺疊時只留下 12px 提示邊，滑鼠移入即展開。</p>
              </div>
              <div className="dock-options">
                {[
                  ["none", "不隱藏"],
                  ["left", "左側"],
                  ["right", "右側"],
                  ["top", "上方"],
                ].map(([value, label]) => (
                  <label key={value}>
                    <input
                      type="radio"
                      name="hud-dock"
                      checked={dock === value}
                      onChange={() => updateDock(value)}
                    />
                    {label}
                  </label>
                ))}
              </div>
            </section>

            <section className="appearance-section">
              <div>
                <h2>Window opacity</h2>
                <p>套用到懸浮球與用量卡片。</p>
              </div>
              <input
                aria-label="透明度"
                type="range"
                min="10"
                max="100"
                step="10"
                value={windowOpacity}
                onChange={(event) => {
                  const value = Number(event.target.value);
                  setWindowOpacity(value);
                  localStorage.setItem("window-opacity", String(value));
                }}
              />
            </section>

            <section className="appearance-section">
              <div>
                <h2>開機自動啟動</h2>
                <p>登入 Windows 時自動開啟 AI Usage Floating Ball。</p>
              </div>
              <label className="switch-label">
                <input
                  type="checkbox"
                  checked={autostart}
                  onChange={(event) => void toggleAutostart(event.target.checked)}
                />
                隨開機啟動
              </label>
            </section>
          </div>
        )}

        {view === "notifications" && (
          <div className="settings-view">
            <section className="appearance-section">
              <div>
                <h2>用量警示通知</h2>
                <p>當供應商達到警告或危險水平時顯示系統通知。</p>
              </div>
              <label className="switch-label">
                <input
                  type="checkbox"
                  checked={notificationsEnabled}
                  onChange={(event) => {
                    setNotificationsEnabled(event.target.checked);
                    localStorage.setItem(
                      "notifications-enabled",
                      event.target.checked ? "1" : "0",
                    );
                  }}
                />
                啟用通知
              </label>
            </section>
            <section className="appearance-section">
              <div>
                <h2>測試通知</h2>
                <p>確認 Windows 通知權限與顯示正常。</p>
              </div>
              <button
                className="ghost-button"
                disabled={busy}
                onClick={() => void sendTestNotification()}
              >
                送出測試通知
              </button>
            </section>
          </div>
        )}
      </main>
    </div>
  );
}

function fallbackIcon(providerId: string) {
  const icons: Record<string, string> = {
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
  };
  return icons[providerId] ?? "/providers/generic.svg";
}
