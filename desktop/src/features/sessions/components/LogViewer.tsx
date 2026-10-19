import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../../../lib/api";
import { Icon } from "../../../components/ui/Icon";
import { extractLocalhostUrls, openInBrowser } from "../../../lib/browser";
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
  const [isAtBottom, setIsAtBottom] = useState(true);
  const [copied, setCopied] = useState(false);
  const [search, setSearch] = useState("");
  const [filterOnly, setFilterOnly] = useState(false);
  const outputRef = useRef<HTMLPreElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const { displayLogs, matchCount } = useMemo(() => {
    if (!logs) return { displayLogs: "", matchCount: 0 };
    const query = search.trim().toLowerCase();
    if (!query) return { displayLogs: logs, matchCount: 0 };

    let count = 0;
    let pos = 0;
    const lower = logs.toLowerCase();
    while ((pos = lower.indexOf(query, pos)) !== -1) {
      count++;
      pos += query.length;
    }

    if (filterOnly) {
      const filtered = logs
        .split("\n")
        .filter((line) => line.toLowerCase().includes(query))
        .join("\n");
      return { displayLogs: filtered, matchCount: count };
    }

    return { displayLogs: logs, matchCount: count };
  }, [logs, search, filterOnly]);

  const renderedContent = useMemo(() => {
    const query = search.trim();
    if (!query || !displayLogs) return displayLogs;

    const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const regex = new RegExp(`(${escaped})`, "gi");
    const parts = displayLogs.split(regex);

    return parts.map((part, index) =>
      regex.test(part) ? (
        <mark key={index} className="log-viewer-highlight">
          {part}
        </mark>
      ) : (
        part
      ),
    );
  }, [displayLogs, search]);

  const detectedUrls = useMemo(() => {
    return extractLocalhostUrls(`${logs} ${session.command}`);
  }, [logs, session.command]);

  const loadLogs = useCallback(
    async (isSilent = false) => {
      if (!isSilent) {
        setLoading(true);
        setError(null);
      }
      try {
        const result = await api.getSessionLogs(session.id);
        setLogs(result);
        setError(null);
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        if (!isSilent) {
          setLoading(false);
        }
      }
    },
    [session.id],
  );

  useEffect(() => {
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    void loadLogs(false);

    // Auto-refresh logs silently every 1.5 seconds while open
    const interval = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        void loadLogs(true);
      }
    }, 1500);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.clearInterval(interval);
    };
  }, [loadLogs]);

  const handleScroll = useCallback(() => {
    const output = outputRef.current;
    if (!output) return;
    const distanceFromBottom =
      output.scrollHeight - output.scrollTop - output.clientHeight;
    setIsAtBottom(distanceFromBottom <= 40);
  }, []);

  useEffect(() => {
    const output = outputRef.current;
    if (output && isAtBottom) {
      output.scrollTop = output.scrollHeight;
    }
  }, [logs, isAtBottom]);

  const scrollToBottom = useCallback(() => {
    const output = outputRef.current;
    if (output) {
      output.scrollTop = output.scrollHeight;
      setIsAtBottom(true);
    }
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        if (search) {
          setSearch("");
        } else {
          onClose();
        }
      } else if (
        (event.metaKey || event.ctrlKey) &&
        event.key.toLowerCase() === "f"
      ) {
        event.preventDefault();
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, search]);

  async function copyLogs() {
    try {
      const textToCopy = filterOnly && search.trim() ? displayLogs : logs;
      await navigator.clipboard.writeText(textToCopy);
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
          <div className="log-viewer-search-wrap">
            <label className="log-viewer-search-field">
              <Icon name="search" size={14} />
              <input
                ref={searchInputRef}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Find in logs (Cmd+F)..."
                aria-label="Find in logs"
              />
              {search && (
                <button
                  type="button"
                  className="log-viewer-search-clear"
                  onClick={() => setSearch("")}
                  title="Clear search"
                  aria-label="Clear search"
                >
                  ×
                </button>
              )}
            </label>
            {search.trim() ? (
              <span className="log-viewer-match-badge">
                {matchCount} {matchCount === 1 ? "match" : "matches"}
              </span>
            ) : null}
            {search.trim() ? (
              <button
                type="button"
                className={`log-viewer-filter-toggle ${filterOnly ? "active" : ""}`}
                onClick={() => setFilterOnly((prev) => !prev)}
                title="Only show lines matching search query"
              >
                {filterOnly ? "Filtered" : "Filter lines"}
              </button>
            ) : null}
          </div>

          <div className="log-viewer-actions">
            <div className="log-viewer-stream-label">
              <span className="log-viewer-live-dot" />
              <span className="log-viewer-count">
                {logs
                  ? filterOnly && search.trim()
                    ? `${displayLogs.split("\n").length} / ${logs.split("\n").length} lines`
                    : `${logs.split("\n").length} lines`
                  : "0 lines"}
              </span>
            </div>
            {detectedUrls.map((url) => (
              <button
                key={url}
                className="log-viewer-tool-button log-viewer-url-btn"
                onClick={() => void openInBrowser(url)}
                title={`Open ${url} in default browser`}
              >
                <Icon name="globe" size={14} />
                <span>{url}</span>
                <Icon name="external" size={12} />
              </button>
            ))}
            <button
              className="log-viewer-tool-button"
              onClick={() => void loadLogs(false)}
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
              {copied
                ? "Copied"
                : filterOnly && search.trim()
                  ? "Copy filtered"
                  : "Copy logs"}
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
              <button onClick={() => void loadLogs(false)}>Try again</button>
            </div>
          ) : logs ? (
            <>
              <pre
                className="log-viewer-output"
                ref={outputRef}
                onScroll={handleScroll}
                tabIndex={0}
                aria-label="Session log output"
              >
                {renderedContent}
              </pre>
              {!isAtBottom && (
                <button
                  className="log-viewer-scroll-btn"
                  onClick={scrollToBottom}
                  aria-label="Jump to latest logs"
                >
                  <Icon name="arrow-down" size={13} />
                  <span>Latest logs</span>
                </button>
              )}
            </>
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
            <kbd>Cmd+F</kbd> find <span className="log-viewer-separator">·</span>{" "}
            <kbd>Esc</kbd> close
          </span>
          <button className="log-viewer-done" onClick={onClose}>
            Done
          </button>
        </footer>
      </section>
    </div>
  );
}
