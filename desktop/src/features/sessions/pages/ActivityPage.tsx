import { useMemo, useState } from "react";
import { Icon } from "../../../components/ui/Icon";
import { api } from "../../../lib/api";
import { useSessions } from "../hooks/useSessions";
import type { ProcessSession } from "../types";

type Filter = "all" | "running" | "completed" | "failed";

function getOutcome(session: ProcessSession) {
  if (session.status === "running") return "running";
  if (session.exit_code === 0) return "completed";
  if (session.exit_code != null) return "failed";
  return "exited";
}

function formatStartTime(session: ProcessSession) {
  return new Date(session.start_time_seconds * 1000).toLocaleString();
}

export function ActivityPage() {
  const { sessions, loading, error, refresh } = useSessions();
  const [filter, setFilter] = useState<Filter>("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [logs, setLogs] = useState("");
  const [logsLoading, setLogsLoading] = useState(false);
  const [logsError, setLogsError] = useState<string | null>(null);

  const filteredSessions = useMemo(() => {
    return [...sessions]
      .filter((session) => {
        const outcome = getOutcome(session);
        if (filter === "all") return true;
        if (filter === "running") return outcome === "running";
        if (filter === "completed") return outcome === "completed";
        return outcome === "failed";
      })
      .sort(
        (a, b) =>
          b.start_time_seconds - a.start_time_seconds ||
          b.start_time_useconds - a.start_time_useconds,
      );
  }, [sessions, filter]);

  async function viewLogs(session: ProcessSession) {
    setSelectedId(session.id);
    setLogs("");
    setLogsError(null);
    setLogsLoading(true);

    try {
      const result = await api.getSessionLogs(session.id);
      setLogs(result);
    } catch (cause) {
      setLogsError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLogsLoading(false);
    }
  }

  return (
    <section className="page-content activity-page">
      <div className="eyebrow">
        <span className="eyebrow-line" /> DEVPANE WORKSPACE
      </div>

      <div className="activity-heading">
        <div>
          <h1>
            Activity<span className="heading-period">.</span>
          </h1>
          <p className="activity-subtitle">
            Review process outcomes and inspect session logs.
          </p>
        </div>

        <button className="secondary-button" onClick={() => void refresh()}>
          <Icon name="refresh" size={15} />
          Refresh
        </button>
      </div>

      <div className="activity-toolbar">
        <div className="activity-filters">
          {(["all", "running", "completed", "failed"] as Filter[]).map(
            (value) => (
              <button
                key={value}
                className={`activity-filter ${filter === value ? "is-active" : ""}`}
                onClick={() => setFilter(value)}
              >
                {value[0].toUpperCase() + value.slice(1)}
              </button>
            ),
          )}
        </div>

        <span className="activity-count">
          {filteredSessions.length} session
          {filteredSessions.length === 1 ? "" : "s"}
        </span>
      </div>

      {error && <p className="activity-error">{error}</p>}

      {loading ? (
        <div className="activity-empty">Loading sessions…</div>
      ) : filteredSessions.length === 0 ? (
        <div className="activity-empty">
          <Icon name="activity" size={22} />
          <strong>No matching activity</strong>
          <span>Try another filter or start a new process.</span>
        </div>
      ) : (
        <div className="activity-list">
          {filteredSessions.map((session) => {
            const outcome = getOutcome(session);

            return (
              <article className="activity-item" key={session.id}>
                <div
                  className={`activity-marker activity-marker--${outcome}`}
                />

                <div className="activity-item-main">
                  <div className="activity-item-title">
                    <strong>{session.name || "Unnamed process"}</strong>
                    <span
                      className={`activity-outcome activity-outcome--${outcome}`}
                    >
                      {outcome === "completed"
                        ? "Completed"
                        : outcome === "failed"
                          ? `Failed · ${session.exit_code}`
                          : outcome === "running"
                            ? "Running"
                            : "Exited · code unavailable"}
                    </span>
                  </div>

                  <code className="activity-command" title={session.command}>
                    {session.command}
                  </code>

                  <div className="activity-meta">
                    <span>PID {session.pid}</span>
                    <span>{formatStartTime(session)}</span>
                  </div>

                  {selectedId === session.id && (
                    <div className="activity-log-panel">
                      <div className="activity-log-heading">
                        <div className="activity-log-label">
                          <Icon name="terminal" size={14} />
                          <span>Output</span>
                        </div>

                        <button
                          className="row-action"
                          onClick={() => setSelectedId(null)}
                          aria-label="Close logs"
                          title="Close logs"
                        >
                          <Icon name="close" size={13} />
                        </button>
                      </div>

                      <div className="activity-log-content">
                        {logsLoading ? (
                          <p className="activity-log-state">Loading output…</p>
                        ) : logsError ? (
                          <p className="activity-error">{logsError}</p>
                        ) : logs ? (
                          <pre>{logs}</pre>
                        ) : (
                          <p className="activity-log-state">
                            No output captured for this session.
                          </p>
                        )}
                      </div>
                    </div>
                  )}
                </div>

                <button
                  className="secondary-button activity-log-button"
                  onClick={() => void viewLogs(session)}
                >
                  View Logs
                </button>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}
