import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { demoUsageState } from "./demoState";
import type { UsageState } from "./providerTypes";

function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

const demoProvider = new URLSearchParams(window.location.search).get("provider");
const demoMode =
  !isTauriRuntime() &&
  new URLSearchParams(window.location.search).get("demo") === "1";

export function useUsageState() {
  const [state, setState] = useState<UsageState | null>(() =>
    demoMode ? demoUsageState(demoProvider) : null,
  );
  const [loading, setLoading] = useState(!demoMode);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (demoMode) {
      setLoading(false);
      return;
    }
    if (!isTauriRuntime()) {
      setLoading(false);
      return;
    }
    try {
      const next = await invoke<UsageState>("get_usage_state");
      setState(next);
      setError(null);
    } catch (requestError) {
      setError(String(requestError));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void reload();
    if (isTauriRuntime() && !demoMode) {
      void listen<UsageState>("provider://state-updated", (event) => {
        if (!disposed) {
          setState(event.payload);
          setError(null);
          setLoading(false);
        }
      })
        .then((cleanup) => {
          unlisten = cleanup;
        })
        .catch((listenError) => {
          if (!disposed) setError(String(listenError));
        });
    }

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [reload]);

  const selectProfile = useCallback(
    async (profileId: string) => {
      if (demoMode) {
        setState((current) =>
          current ? { ...current, selectedProfileId: profileId } : current,
        );
        return;
      }
      if (!isTauriRuntime()) return;
      await invoke("select_provider_profile", { profileId });
      setState((current) =>
        current ? { ...current, selectedProfileId: profileId } : current,
      );
    },
    [],
  );

  const refreshProfile = useCallback(
    async (profileId: string) => {
      if (demoMode) return;
      if (!isTauriRuntime()) return;
      await invoke("refresh_provider_profile", { profileId });
      await reload();
    },
    [reload],
  );

  return {
    state,
    loading,
    error,
    reload,
    selectProfile,
    refreshProfile,
  };
}
