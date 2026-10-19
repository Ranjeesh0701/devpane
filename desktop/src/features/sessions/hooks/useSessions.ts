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
      setSessions(result);
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

    const interval = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        void fetchSessions(true);
      }
    }, 1500);

    return () => {
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
    stopProject,
    stopAllSessions,
  };
}
