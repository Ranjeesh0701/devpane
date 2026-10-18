import { useState } from "react";
import { Icon } from "../../../components/ui/Icon";
import { formatStarted } from "../utils/formatStarted";
import { extractLocalhostUrls, openInBrowser } from "../../../lib/browser";
import type { ProcessSession } from "../types";
import { ProcessStatusBadge } from "../../../components/ui/ProcessStatusBadge";

type Props = {
  sessions: ProcessSession[];
  onStop: (id: string) => Promise<void>;
  onViewLogs: (session: ProcessSession) => void;
};

export function SessionTable({ sessions, onStop, onViewLogs }: Props) {
  const [stoppingId, setStoppingId] = useState<string | null>(null);
  const [stopError, setStopError] = useState<string | null>(null);

  async function handleStop(id: string) {
    setStoppingId(id);
    setStopError(null);
    try {
      await onStop(id);
    } catch (cause) {
      setStopError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setStoppingId(null);
    }
  }

  if (sessions.length === 0) return null;

  return (
    <div className="session-table-wrap">
      <table className="session-table">
        <thead>
          <tr>
            <th>PROCESS</th>
            <th>COMMAND</th>
            <th>PID</th>
            <th>STARTED</th>
            <th>STATUS</th>
            <th>
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {sessions.map((session) => (
            <tr key={session.id}>
              <td>
                <div className="process-cell">
                  <div className="process-icon">
                    <Icon name="terminal" size={17} />
                  </div>
                  <div className="process-copy">
                    <strong>{session.name || "Unnamed process"}</strong>
                    <span title={session.id}>{session.id.slice(0, 8)}…</span>
                  </div>
                </div>
              </td>
              <td>
                <code className="command-cell" title={session.command}>
                  {session.command}
                </code>
              </td>
              <td>
                <span className="pid-cell">{session.pid}</span>
              </td>
              <td>
                <span className="started-cell">
                  <Icon name="clock" size={14} />
                  {formatStarted(session)}
                </span>
              </td>
              <td>
                <ProcessStatusBadge session={session} />
              </td>
              <td>
                <div className="row-actions">
                  {extractLocalhostUrls(session.command).length > 0 && (
                    <button
                      className="row-action"
                      aria-label={`Open ${extractLocalhostUrls(session.command)[0]} in browser`}
                      onClick={() =>
                        void openInBrowser(
                          extractLocalhostUrls(session.command)[0]
                        )
                      }
                      title={`Open ${extractLocalhostUrls(session.command)[0]} in browser`}
                      style={{ color: "#38ef7d" }}
                    >
                      <Icon name="globe" size={16} />
                    </button>
                  )}
                  <button
                    className="row-action"
                    aria-label={`View logs for ${session.name}`}
                    onClick={() => onViewLogs(session)}
                    title="View logs"
                  >
                    <Icon name="terminal" size={16} />
                  </button>
                  <button
                    className="row-action"
                    aria-label={`Copy session ID for ${session.name}`}
                    onClick={() =>
                      void navigator.clipboard?.writeText(session.id)
                    }
                    title="Copy session ID"
                  >
                    <Icon name="copy" size={16} />
                  </button>
                  {session.status === "running" && (
                    <button
                      className="row-action"
                      onClick={() => void handleStop(session.id)}
                      title="Stop session"
                      aria-label={`Stop ${session.name}`}
                      disabled={stoppingId === session.id}
                    >
                      <Icon name="square" size={16} />
                    </button>
                  )}
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {stopError && (
        <p className="error-message" role="alert">
          {stopError}
        </p>
      )}
      <div className="table-footer">
        <span>
          Showing <strong>{sessions.length}</strong> session
          {sessions.length === 1 ? "" : "s"}
        </span>
        <span className="live-indicator">
          <span /> Synced on refresh
        </span>
      </div>
    </div>
  );
}
