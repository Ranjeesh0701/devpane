import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../../../lib/api";
import { Icon } from "../../../components/ui/Icon";
import type { ProcessSession } from "../types";
import "./LogViewer.css";

type Props = {
  session: ProcessSession;
  onClose: () => void;
};

export function LogViewer({ session, onClose }: Props) {
  const [logs, setLogs] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const outputRef = useRef<HTMLPreElement>(null);

  const loadLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await api.getSessionLogs(session.id);
      setLogs(result);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, [session.id]);

  useEffect(() => {
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    void loadLogs();

    // Auto-refresh logs every 1.5 seconds while open if process is running
    const interval = window.setInterval(() => {
      void loadLogs();
    }, 1500);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.clearInterval(interval);
    };
  }, [loadLogs]);

  useEffect(() => {
    const output = outputRef.current;
    if (output) output.scrollTop = output.scrollHeight;
  }, [logs]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  async function copyLogs() {
    try {
      await navigator.clipboard.writeText(logs);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setError("Could not copy logs to the clipboard.");
    }
  }

  return (
    <div
      className="log-viewer-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section
        className="log-viewer-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="log-viewer-title"
      >
        <header className="log-viewer-header">
          <div className="log-viewer-heading">
            <div className="log-viewer-icon">
              <Icon name="terminal" size={18} />
            </div>
            <div>
              <div className="log-viewer-eyebrow">SESSION OUTPUT</div>
              <h2 id="log-viewer-title">
                {session.name || "Unnamed process"}
                <span>.logs</span>
              </h2>
              <p>
                <span className="log-viewer-status-dot" /> PID {session.pid}{" "}
                <span className="log-viewer-separator">/</span>{" "}
                {session.command}
              </p>
            </div>
          </div>
          <button
            className="log-viewer-close"
            onClick={onClose}
            aria-label="Close log viewer"
            title="Close"
          >
            ×
          </button>
        </header>

        <div className="log-viewer-toolbar">
          <div className="log-viewer-stream-label">
            <span className="log-viewer-live-dot" /> Captured output{" "}
            <span className="log-viewer-count">
              {logs ? `${logs.split("\n").length} lines` : "0 lines"}
            </span>
          </div>
          <div className="log-viewer-actions">
            <button
              className="log-viewer-tool-button"
              onClick={() => void loadLogs()}
              disabled={loading}
            >
              <span className={loading ? "log-viewer-spin" : ""}>
                <Icon name="refresh" size={14} />
              </span>{" "}
              Refresh
            </button>
            <button
              className="log-viewer-tool-button"
              onClick={() => void copyLogs()}
              disabled={!logs || loading}
            >
              <Icon name={copied ? "check" : "copy"} size={14} />{" "}
              {copied ? "Copied" : "Copy logs"}
            </button>
          </div>
        </div>

        <div className="log-viewer-output-wrap">
          {loading && !logs ? (
            <div className="log-viewer-message">
              <span className="log-viewer-spinner" /> Loading session output…
            </div>
          ) : error && !logs ? (
            <div className="log-viewer-message log-viewer-error">
              <strong>Unable to load logs</strong>
              <span>{error}</span>
              <button onClick={() => void loadLogs()}>Try again</button>
            </div>
          ) : logs ? (
            <pre
              className="log-viewer-output"
              ref={outputRef}
              tabIndex={0}
              aria-label="Session log output"
            >
              {logs}
            </pre>
          ) : (
            <div className="log-viewer-message">
              <div className="log-viewer-empty-icon">
                <Icon name="terminal" size={22} />
              </div>
              <strong>No output yet</strong>
              <span>
                This process hasn't written anything to its log files.
              </span>
            </div>
          )}
        </div>

        {error && logs && (
          <div className="log-viewer-inline-error" role="alert">
            {error}
          </div>
        )}
        <footer className="log-viewer-footer">
          <span>
            <kbd>Esc</kbd> to close{" "}
            <span className="log-viewer-separator">·</span> Refresh to fetch the
            latest output
          </span>
          <button className="log-viewer-done" onClick={onClose}>
            Done
          </button>
        </footer>
      </section>
    </div>
  );
}
