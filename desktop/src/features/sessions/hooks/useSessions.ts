import { useCallback, useEffect, useState } from "react";
import { api } from "../../../lib/api";
import type { ProcessSession } from "../types";
import type { NewSessionInput } from "../types";

export function useSessions() {
  const [sessions, setSessions] = useState<ProcessSession[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await api.listSessions();
      setSessions(result);
      setLastUpdated(new Date());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

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
    void refresh();
  }, [refresh]);

  return {
    sessions,
    error,
    loading,
    lastUpdated,
    refresh,
    startSession,
    stopSession,
    stopProject,
    stopAllSessions,
  };
}
