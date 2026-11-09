import { useCallback, useEffect, useState } from "react";
import { api } from "../../../lib/api";
import type { ProcessSession } from "../types";
import type { NewSessionInput } from "../types";

export function useSessions() {
  const [sessions, setSessions] = useState<ProcessSession[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const fetchSessions = useCallback(async (isSilent = false) => {
    if (!isSilent) {
      setLoading(true);
      setError(null);
    }
    try {
      const result = await api.listSessions();
      setSessions((prev) => {
        // Fast comparison to prevent unnecessary React renders
        if (
          prev.length === result.length &&
          prev.every(
            (p, i) =>
              p.id === result[i]?.id &&
              p.status === result[i]?.status &&
              p.cpu_percent === result[i]?.cpu_percent &&
              p.memory_bytes === result[i]?.memory_bytes &&
              p.exit_code === result[i]?.exit_code,
          )
        ) {
          return prev;
        }
        return result;
      });
      setError(null);
      setLastUpdated(new Date());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      if (!isSilent) {
        setLoading(false);
      }
    }
  }, []);

  const refresh = useCallback(async () => {
    await fetchSessions(false);
  }, [fetchSessions]);

  const startSession = useCallback(
    async (input: NewSessionInput) => {
      try {
        await api.startSession(input);
      } catch (cause) {
        const message = cause instanceof Error ? cause.message : String(cause);
        throw new Error(message);
      }
      await refresh();
    },
    [refresh],
  );

  const stopSession = useCallback(
    async (id: string) => {
      await api.stopSession(id);
      await refresh();
    },
    [refresh],
  );

  const restartSession = useCallback(
    async (id: string) => {
      await api.restartSession(id);
      await refresh();
    },
    [refresh],
  );

  const rebuildSession = useCallback(
    async (name: string) => {
      await api.rebuildSession(name);
      await refresh();
    },
    [refresh],
  );

  const restartProject = useCallback(
    async (projectDir: string) => {
      await api.restartProject(projectDir);
      await refresh();
    },
    [refresh],
  );

  const syncProject = useCallback(
    async (projectDir: string, installDeps = true) => {
      const res = await api.syncProject(projectDir, installDeps);
      await refresh();
      return res;
    },
    [refresh],
  );

  const stopProject = useCallback(
    async (projectDir: string) => {
      await api.stopProject(projectDir);
      await refresh();
    },
    [refresh],
  );

  const stopAllSessions = useCallback(async () => {
    await api.stopAllSessions();
    await refresh();
  }, [refresh]);

  useEffect(() => {
    void fetchSessions(false);

    // Refresh immediately when tab becomes visible again
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void fetchSessions(true);
      }
    };
    document.addEventListener("visibilitychange", onVisibilityChange);

    // Poll every 2 seconds when active; pause when window is hidden
    const interval = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        void fetchSessions(true);
      }
    }, 2000);

    return () => {
      document.removeEventListener("visibilitychange", onVisibilityChange);
      window.clearInterval(interval);
    };
  }, [fetchSessions]);

  return {
    sessions,
    error,
    loading,
    lastUpdated,
    refresh,
    startSession,
    stopSession,
    restartSession,
    rebuildSession,
    restartProject,
    syncProject,
    stopProject,
    stopAllSessions,
  };
}
