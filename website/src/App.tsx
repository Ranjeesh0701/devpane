import { useState } from "react";
import "./App.css";
import {
  trackDownload,
  trackCopyInstall,
  trackGithubClick,
} from "./lib/analytics";

type TuiLogLine = {
  num: number;
  level: "INFO" | "WARN" | "ERROR";
  text: string;
  url?: string;
  prefix?: string;
};

type TuiService = {
  id: string;
  key: string;
  name: string;
  icon: string;
  statusText: string;
  subline: string;
  port?: number;
  url?: string;
  memory: string;
  memoryBar: string;
  cpu: string;
  cpuBar: string;
  uptime: string;
  logs: TuiLogLine[];
};

const TUI_SERVICES: TuiService[] = [
  {
    id: "all",
    key: "0",
    name: "All services",
    icon: "=",
    statusText: "3/3 Running",
    subline: "Combined log stream",
    memory: "86.2 MB",
    memoryBar: "████████░░░░",
    cpu: "0.8%",
    cpuBar: "────────────",
    uptime: "8m 51s",
    logs: [
      {
        num: 1,
        level: "INFO",
        prefix: "[api]",
        text: "Listening on http://localhost:8080",
        url: "http://localhost:8080",
      },
      {
        num: 2,
        level: "INFO",
        prefix: "[web]",
        text: "Frontend UI server listening on http://localhost:5173",
        url: "http://localhost:5173",
      },
      {
        num: 3,
        level: "INFO",
        prefix: "[web]",
        text: "Linked to API backend at http://localhost:8080",
        url: "http://localhost:8080",
      },
      {
        num: 4,
        level: "INFO",
        prefix: "[worker]",
        text: "Connected to redis://127.0.0.1:6379/0",
      },
      {
        num: 5,
        level: "INFO",
        prefix: "[web]",
        text: "Ready to accept user browser connections",
      },
      {
        num: 6,
        level: "INFO",
        prefix: "[worker]",
        text: "Subscribed to stream 'events' (0 pending)",
      },
      {
        num: 7,
        level: "INFO",
        prefix: "[api]",
        text: "GET /api/v1/sessions 200 OK (0.6ms)",
      },
    ],
  },
  {
    id: "api",
    key: "1",
    name: "api",
    icon: "⬡",
    statusText: "Running",
    subline: ":8080 · PID 34470",
    port: 8080,
    url: "localhost:8080",
    memory: "32.4 MB",
    memoryBar: "████░░░░░░░░",
    cpu: "0.8%",
    cpuBar: "────────────",
    uptime: "8m 51s",
    logs: [
      {
        num: 1,
        level: "INFO",
        text: "Compiling api v0.4.0 (/Workspace/realworld-stack/api)",
      },
      {
        num: 2,
        level: "INFO",
        text: "Finished dev [unoptimized + debuginfo] profile in 0.74s",
      },
      {
        num: 3,
        level: "INFO",
        text: "Listening on http://localhost:8080",
        url: "http://localhost:8080",
      },
      {
        num: 4,
        level: "INFO",
        text: "Readiness check OK: HTTP 200 at /health (8ms)",
      },
      {
        num: 5,
        level: "INFO",
        text: "Database connection pool verified: 10 connections idle",
      },
      {
        num: 6,
        level: "INFO",
        text: "GET /api/v1/sessions 200 OK (0.6ms)",
      },
      {
        num: 7,
        level: "INFO",
        text: "POST /api/v1/services/sync 200 OK (1.1ms)",
      },
    ],
  },
  {
    id: "web",
    key: "2",
    name: "web",
    icon: "🌐",
    statusText: "Running",
    subline: ":5173 · shared with api",
    port: 5173,
    url: "localhost:8080",
    memory: "45.6 MB",
    memoryBar: "█████░░░░░░░",
    cpu: "0.0%",
    cpuBar: "────────────",
    uptime: "8m 51s",
    logs: [
      {
        num: 1,
        level: "INFO",
        text: "Frontend UI server listening on http://localhost:5173",
        url: "http://localhost:5173",
      },
      {
        num: 2,
        level: "INFO",
        text: "Linked to API backend at http://localhost:8080",
        url: "http://localhost:8080",
      },
      {
        num: 3,
        level: "INFO",
        text: "Ready to accept user browser connections",
      },
      {
        num: 4,
        level: "INFO",
        text: "Vite v6.0.4 hot module replacement (HMR) active",
      },
      {
        num: 5,
        level: "INFO",
        text: "Watcher ready: 24 modules synchronized in 18ms",
      },
      {
        num: 6,
        level: "INFO",
        text: "GET /src/App.tsx 200 OK",
      },
    ],
  },
  {
    id: "worker",
    key: "3",
    name: "worker",
    icon: "⚙",
    statusText: "Running",
    subline: "Background process",
    memory: "8.2 MB",
    memoryBar: "█░░░░░░░░░░░",
    cpu: "0.0%",
    cpuBar: "────────────",
    uptime: "8m 51s",
    logs: [
      {
        num: 1,
        level: "INFO",
        text: "Supervised 'worker' (PID 34482) • Zero-zombie active",
      },
      {
        num: 2,
        level: "INFO",
        text: "Connected to redis://127.0.0.1:6379/0",
      },
      {
        num: 3,
        level: "INFO",
        text: "Subscribed to stream 'events' (0 pending)",
      },
      {
        num: 4,
        level: "INFO",
        text: "✓ Heartbeat ACK • worker pool healthy",
      },
      {
        num: 5,
        level: "INFO",
        text: "Job #4089 processed in 14ms (success)",
      },
      {
        num: 6,
        level: "INFO",
        text: "Waiting for incoming event payloads...",
      },
    ],
  },
];

function HeroTerminal() {
  const [activeId, setActiveId] = useState<string>("web");
  const activeService =
    TUI_SERVICES.find((s) => s.id === activeId) || TUI_SERVICES[2];

  return (
    <div className="terminal-window">
      {/* 1. Window Title Bar */}
      <div className="tui-window-titlebar">
        <div className="window-dots" aria-hidden="true">
          <span className="dot dot-close" />
          <span className="dot dot-minimize" />
          <span className="dot dot-expand" />
        </div>
        <div className="tui-window-title">
          realworld-stack — devpane up — 136×30
        </div>
        <div className="tui-title-spacer" aria-hidden="true" />
      </div>

      {/* 2. Top TUI Bar */}
      <div className="tui-topbar">
        <div className="tui-brand">
          <span className="tui-diamond">◆</span>
          <strong className="tui-brand-name">Devpane</strong>
          <span className="tui-brand-ver">v0.4.0</span>
          <span className="tui-slash">/</span>
          <span className="tui-project-name">realworld-stack</span>
        </div>
        <div className="tui-filter">
          <span className="tui-filter-icon">🔍</span>
          <span>Filter logs</span>
          <kbd className="tui-key">/</kbd>
        </div>
      </div>

      {/* 3. Subheader */}
      <div className="tui-subheader">
        <div className="tui-sub-left">
          <span className="tui-sub-label">SERVICES</span>
          <span className="tui-sub-count">3</span>
          <span className="tui-sub-buttons">[-] [⤢]</span>
        </div>
        <div className="tui-sub-center">
          <strong className="tui-active-name">{activeService.name}</strong>
          <span className="tui-active-status">
            <span className="tui-check">✓</span> {activeService.statusText}
          </span>
        </div>
        <div className="tui-sub-right">
          <span className="tui-action">
            <span className="tui-action-icon">↺</span> Restart{" "}
            <kbd className="tui-key">R</kbd>
          </span>
          <span className="tui-action">
            <span className="tui-action-icon">■</span> Stop{" "}
            <kbd className="tui-key">S</kbd>
          </span>
          {activeService.url && (
            <a
              href={`http://${activeService.url}`}
              target="_blank"
              rel="noreferrer"
              className="tui-port-link"
            >
              {activeService.url} <span className="tui-arrow">↗</span>
            </a>
          )}
        </div>
      </div>

      {/* 4. Main Split View: Sidebar + Workspace */}
      <div className="tui-body">
        {/* Sidebar */}
        <aside className="tui-sidebar">
          <div className="tui-sidebar-list" role="tablist" aria-label="Services">
            {TUI_SERVICES.map((svc) => {
              const isActive = svc.id === activeId;
              return (
                <button
                  key={svc.id}
                  className={`tui-service-item ${isActive ? "active" : ""}`}
                  onClick={() => setActiveId(svc.id)}
                  role="tab"
                  aria-selected={isActive}
                  type="button"
                >
                  <div className="tui-service-line1">
                    <div className="tui-service-left">
                      <span className="tui-service-icon">{svc.icon}</span>
                      <strong className="tui-service-name">{svc.name}</strong>
                      {svc.id !== "all" && <span className="tui-service-dot" />}
                    </div>
                    <span className="tui-service-key">{svc.key}</span>
                  </div>
                  <div className="tui-service-subline">{svc.subline}</div>
                </button>
              );
            })}
          </div>

          <div className="tui-sidebar-footer">
            <div className="tui-meta-row">
              <span className="tui-meta-muted">Environment</span>
              <span className="tui-meta-val">development</span>
            </div>
            <div className="tui-meta-row">
              <span className="tui-meta-muted">Services</span>
              <span className="tui-meta-green">
                3 running <span className="tui-meta-muted">· 0 exited</span>
              </span>
            </div>
          </div>
        </aside>

        {/* Workspace */}
        <main className="tui-main">
          {/* Telemetry Cards */}
          <div className="tui-metrics-grid">
            <div className="tui-metric-card">
              <div className="tui-metric-header">
                <span className="tui-metric-label">Memory</span>
              </div>
              <div className="tui-metric-value-row">
                <strong className="tui-metric-val">{activeService.memory}</strong>
                <span
                  className="tui-meter-bar tui-meter-green"
                  aria-hidden="true"
                >
                  [{activeService.memoryBar}]
                </span>
              </div>
            </div>

            <div className="tui-metric-card">
              <div className="tui-metric-header">
                <span className="tui-metric-label">CPU</span>
              </div>
              <div className="tui-metric-value-row">
                <strong className="tui-metric-val">{activeService.cpu}</strong>
                <span
                  className="tui-meter-bar tui-meter-blue"
                  aria-hidden="true"
                >
                  [{activeService.cpuBar}]
                </span>
              </div>
            </div>

            <div className="tui-metric-card">
              <div className="tui-metric-header">
                <span className="tui-metric-label">Uptime</span>
              </div>
              <div className="tui-metric-value-row">
                <strong className="tui-metric-val">{activeService.uptime}</strong>
                <span className="tui-active-badge">
                  <span className="tui-beacon" /> Active
                </span>
              </div>
            </div>
          </div>

          {/* Logs Box */}
          <div className="tui-logs-box">
            <div className="tui-logs-header">
              <div className="tui-logs-tabs">
                <span className="tui-log-tab active">Logs</span>
                <span className="tui-log-tab tab-info">Info</span>
                <span className="tui-log-tab tab-warn">Warn</span>
                <span className="tui-log-tab tab-error">Error</span>
              </div>
              <div className="tui-wrap-badge">[Wrap: ON • W]</div>
            </div>

            <div className="tui-logs-stream">
              {activeService.logs.map((log) => (
                <div key={log.num} className="tui-log-line">
                  <span className="tui-log-num">{log.num}</span>
                  <span
                    className={`tui-log-level level-${log.level.toLowerCase()}`}
                  >
                    {log.level}
                  </span>
                  {log.prefix && (
                    <span className="tui-log-prefix">{log.prefix}</span>
                  )}
                  <span className="tui-log-text">
                    {log.url ? (
                      log.text.split(log.url).map((part, i, arr) => (
                        <span key={i}>
                          {part}
                          {i < arr.length - 1 && (
                            <span className="tui-log-url">{log.url}</span>
                          )}
                        </span>
                      ))
                    ) : (
                      log.text
                    )}
                  </span>
                </div>
              ))}
            </div>

            <div className="tui-logs-footer">
              <span className="tui-following-pill">↓ Following latest</span>
            </div>
          </div>
        </main>
      </div>

      {/* 5. Bottom Status Bar */}
      <footer className="tui-statusbar">
        <div className="tui-status-left">
          <span className="tui-status-green">✓ 3/3 running</span>
        </div>
        <div className="tui-status-center">
          <span>
            <kbd>Tab</kbd> Switch
          </span>
          <span>
            <kbd>R</kbd> Restart
          </span>
          <span>
            <kbd>S</kbd> Stop
          </span>
          <span>
            <kbd>W</kbd> Unwrap
          </span>
          <span>
            <kbd>/</kbd> Filter
          </span>
          <span>
            <kbd>M</kbd> Min/Max
          </span>
          <span>
            <kbd>^T</kbd> Shell
          </span>
          <span>
            <kbd>Q</kbd>
          </span>
        </div>
        <div className="tui-status-right">
          <span className="tui-status-green">✓ Daemon connected</span>
        </div>
      </footer>
    </div>
  );
}

function HeroDesktopPreview() {
  const [filter, setFilter] = useState<string>("");

  return (
    <div className="desktop-window-preview">
      {/* Titlebar */}
      <div className="gui-titlebar">
        <div className="window-dots" aria-hidden="true">
          <span className="dot dot-close" />
          <span className="dot dot-minimize" />
          <span className="dot dot-expand" />
        </div>
        <div className="gui-titlebar-crumbs">
          <span>Workspace</span>
          <span className="gui-crumb-sep">›</span>
          <span className="gui-crumb-active">Sessions</span>
        </div>
        <div className="gui-titlebar-shortcut">
          <kbd>⌘K</kbd>
        </div>
      </div>

      {/* Main app layout */}
      <div className="gui-layout">
        {/* Sidebar */}
        <aside className="gui-sidebar">
          <div className="gui-brand">
            <div className="gui-brand-icon">
              <span>&gt;_</span>
            </div>
            <div className="gui-brand-copy">
              <strong>devpane</strong>
              <span>PROCESS WORKSPACE</span>
            </div>
          </div>

          <div className="gui-host-card">
            <div className="gui-host-avatar">D</div>
            <div className="gui-host-info">
              <span className="gui-host-title">Local machine</span>
              <span className="gui-host-sub">macOS · Desktop</span>
            </div>
            <span className="gui-host-dot" />
          </div>

          <div className="gui-nav-section">
            <span className="gui-nav-label">WORKSPACE</span>
            <div className="gui-nav-item active">
              <span className="gui-nav-icon">⊞</span>
              <span>Sessions</span>
              <span className="gui-nav-pill">3</span>
            </div>
            <div className="gui-nav-item">
              <span className="gui-nav-icon">⚡</span>
              <span>Activity</span>
            </div>
            <div className="gui-nav-item">
              <span className="gui-nav-icon">⚙</span>
              <span>Settings</span>
            </div>
          </div>

          <div className="gui-sidebar-bottom">
            <span>Devpane Desktop</span>
            <span className="gui-version-badge">v0.4.0</span>
          </div>
        </aside>

        {/* Content */}
        <div className="gui-content">
          <div className="gui-header">
            <div>
              <h3>
                All sessions <span className="gui-count-pill">3</span>
              </h3>
              <p>Processes managed by your Devpane daemon</p>
            </div>
            <button className="gui-refresh-btn" type="button">
              <span>↺</span> Refresh
            </button>
          </div>

          <div className="gui-card">
            <div className="gui-search-toolbar">
              <div className="gui-search-field">
                <span>🔍</span>
                <input
                  placeholder="Search sessions..."
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                  aria-label="Search preview sessions"
                />
                <kbd>⌘F</kbd>
              </div>
              <div className="gui-filter-status">
                <span className="gui-live-dot" /> All processes
              </div>
            </div>

            {/* Project Header */}
            <div className="gui-project-header">
              <div className="gui-project-info">
                <div className="gui-project-icon">⊞</div>
                <div className="gui-project-text">
                  <div className="gui-project-title-row">
                    <strong>realworld-stack</strong>
                    <span className="gui-meta-badge">3 running · 0 exited</span>
                  </div>
                  <span className="gui-project-path">
                    /Users/ranjeesh/Workspace/devbox/examples/realworld-stack
                  </span>
                </div>
              </div>
              <div className="gui-project-actions">
                <button className="gui-btn-sync" type="button">
                  <span>↺</span> Sync
                </button>
                <button className="gui-btn-restart" type="button">
                  <span>↺</span> Restart
                </button>
                <button className="gui-btn-stop" type="button">
                  <span>■</span> Stop
                </button>
              </div>
            </div>

            {/* Table */}
            <div className="gui-table-wrap">
              <table className="gui-table">
                <thead>
                  <tr>
                    <th>PROCESS</th>
                    <th>COMMAND</th>
                    <th>CPU</th>
                    <th>MEMORY</th>
                    <th>STATUS</th>
                    <th style={{ textAlign: "right" }}>ACTIONS</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>
                      <div className="gui-proc-cell">
                        <span className="gui-proc-icon hex">⬡</span>
                        <div>
                          <div className="gui-proc-title">
                            <strong>api</strong>
                            <span className="gui-port-badge">:8080</span>
                          </div>
                          <span className="gui-proc-sub">
                            PID 34470 · 60707a8c…
                          </span>
                        </div>
                      </div>
                    </td>
                    <td>
                      <code className="gui-code">cargo run</code>
                    </td>
                    <td>
                      <span className="gui-stat">0.8%</span>
                    </td>
                    <td>
                      <span className="gui-stat green">32.4 MB</span>
                    </td>
                    <td>
                      <span className="gui-status-pill running">
                        <span className="gui-dot" /> Running
                      </span>
                    </td>
                    <td>
                      <div className="gui-row-actions">
                        <button className="gui-act-btn" title="Open in browser">
                          ↗
                        </button>
                        <button className="gui-act-btn" title="View live logs">
                          ⌨
                        </button>
                        <button className="gui-act-btn" title="Restart">
                          ↺
                        </button>
                        <button className="gui-act-btn" title="More">
                          ···
                        </button>
                      </div>
                    </td>
                  </tr>

                  <tr>
                    <td>
                      <div className="gui-proc-cell">
                        <span className="gui-proc-icon globe">🌐</span>
                        <div>
                          <div className="gui-proc-title">
                            <strong>web</strong>
                            <span className="gui-port-badge">:5173</span>
                          </div>
                          <span className="gui-proc-sub">
                            PID 34475 · eb9b77a7…
                          </span>
                        </div>
                      </div>
                    </td>
                    <td>
                      <code className="gui-code">bun run dev</code>
                    </td>
                    <td>
                      <span className="gui-stat">0.0%</span>
                    </td>
                    <td>
                      <span className="gui-stat green">45.6 MB</span>
                    </td>
                    <td>
                      <span className="gui-status-pill running">
                        <span className="gui-dot" /> Running
                      </span>
                    </td>
                    <td>
                      <div className="gui-row-actions">
                        <button className="gui-act-btn" title="Open in browser">
                          ↗
                        </button>
                        <button className="gui-act-btn" title="View live logs">
                          ⌨
                        </button>
                        <button className="gui-act-btn" title="Restart">
                          ↺
                        </button>
                        <button className="gui-act-btn" title="More">
                          ···
                        </button>
                      </div>
                    </td>
                  </tr>

                  <tr>
                    <td>
                      <div className="gui-proc-cell">
                        <span className="gui-proc-icon gear">⚙</span>
                        <div>
                          <div className="gui-proc-title">
                            <strong>worker</strong>
                          </div>
                          <span className="gui-proc-sub">
                            PID 34482 · 8c48de44…
                          </span>
                        </div>
                      </div>
                    </td>
                    <td>
                      <code className="gui-code">python3 worker.py</code>
                    </td>
                    <td>
                      <span className="gui-stat">0.0%</span>
                    </td>
                    <td>
                      <span className="gui-stat green">8.2 MB</span>
                    </td>
                    <td>
                      <span className="gui-status-pill running">
                        <span className="gui-dot" /> Running
                      </span>
                    </td>
                    <td>
                      <div className="gui-row-actions">
                        <button className="gui-act-btn" title="View live logs">
                          ⌨
                        </button>
                        <button className="gui-act-btn" title="Restart">
                          ↺
                        </button>
                        <button className="gui-act-btn" title="More">
                          ···
                        </button>
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div className="gui-table-footer">
              <span>
                Showing <strong>3</strong> sessions (
                <strong style={{ color: "#3fb950" }}>3 running</strong> · 0
                exited)
              </span>
              <span className="gui-footer-live">
                <span className="gui-dot" /> Supervised
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function HeroPreview() {
  const [mode, setMode] = useState<"tui" | "gui">("tui");

  return (
    <div className="hero-preview-container">
      <div className="hero-preview-mode-bar">
        <div className="mode-toggle-group" role="tablist">
          <button
            className={`mode-btn ${mode === "tui" ? "active" : ""}`}
            onClick={() => setMode("tui")}
            role="tab"
            aria-selected={mode === "tui"}
            type="button"
          >
            <span className="mode-icon">⊞</span>
            <span>Terminal TUI</span>
            <span className="mode-tag">devpane up</span>
          </button>
          <button
            className={`mode-btn ${mode === "gui" ? "active" : ""}`}
            onClick={() => setMode("gui")}
            role="tab"
            aria-selected={mode === "gui"}
            type="button"
          >
            <span className="mode-icon">💻</span>
            <span>Desktop GUI</span>
            <span className="mode-tag">Tauri App</span>
          </button>
        </div>
        <div className="mode-hint">
          {mode === "tui"
            ? "Interactive terminal: click services (0-3) to inspect telemetry"
            : "Native desktop app: clean, responsive multi-process control"}
        </div>
      </div>

      {mode === "tui" ? <HeroTerminal /> : <HeroDesktopPreview />}
    </div>
  );
}

function MCPConfigSection() {
  const [activeTab, setActiveTab] = useState<"simulate" | "config">("simulate");
  const [activeClient, setActiveClient] = useState<"cursor" | "claude">("cursor");
  const [copied, setCopied] = useState(false);
  const [simStep, setSimStep] = useState(3);
  const [simulating, setSimulating] = useState(false);

  const cursorSnippet = `{
  "mcpServers": {
    "devpane": {
      "command": "devpane",
      "args": ["mcp"]
    }
  }
}`;

  const copySnippet = () => {
    void navigator.clipboard.writeText(cursorSnippet);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleSimulate = () => {
    setSimStep(0);
    setSimulating(true);
    setTimeout(() => setSimStep(1), 600);
    setTimeout(() => setSimStep(2), 1400);
    setTimeout(() => {
      setSimStep(3);
      setSimulating(false);
    }, 2200);
  };

  return (
    <div className="mcp-section" id="mcp">
      <div className="section-head">
        <div className="tech-badge">MODEL CONTEXT PROTOCOL</div>
        <h2>Native AI Agent Integration.</h2>
        <p>
          Coding agents in Cursor, Claude Desktop, and Windsurf can inspect server errors,
          track compile status, and restart services without hijacking your interactive terminal.
        </p>
      </div>

      <div className="mcp-mode-switcher">
        <button
          className={`mcp-mode-btn ${activeTab === "simulate" ? "active" : ""}`}
          onClick={() => setActiveTab("simulate")}
          type="button"
        >
          <span>⚡ Live AI Agent Simulator</span>
          <span className="mcp-mode-tag">Interactive</span>
        </button>
        <button
          className={`mcp-mode-btn ${activeTab === "config" ? "active" : ""}`}
          onClick={() => setActiveTab("config")}
          type="button"
        >
          <span>⚙ Client Config &amp; Tools</span>
          <span className="mcp-mode-tag">JSON-RPC</span>
        </button>
      </div>

      {activeTab === "simulate" ? (
        <div className="mcp-simulator-card">
          <div className="mcp-simulator-header">
            <div className="mcp-sim-header-left">
              <span className="sim-dot" />
              <strong>AI Coding Agent Diagnostic Flow</strong>
              <span className="sim-sub">Cursor Agent ➜ Devpane MCP stdio</span>
            </div>
            <button
              className="sim-run-btn"
              onClick={handleSimulate}
              disabled={simulating}
              type="button"
            >
              <span>{simulating ? "Diagnosing…" : "Re-run Agent Diagnosis ↻"}</span>
            </button>
          </div>

          <div className="mcp-chat-stream">
            {simStep >= 0 && (
              <div className="sim-bubble sim-user">
                <span className="sim-avatar">👤</span>
                <div className="sim-bubble-body">
                  <div className="sim-author">Developer</div>
                  <p>Why did the payment worker crash during testing?</p>
                </div>
              </div>
            )}

            {simStep >= 1 && (
              <div className="sim-bubble sim-agent">
                <span className="sim-avatar">🤖</span>
                <div className="sim-bubble-body">
                  <div className="sim-author">Cursor Agent</div>
                  <div className="sim-call">
                    <span className="sim-call-tag">TOOL CALL</span>
                    <code>devpane_get_logs(service: "worker", filter: "error", limit: 3)</code>
                  </div>
                  <div className="sim-response">
                    <code>
                      [worker] ConnectionRefusedError: redis://127.0.0.1:6379 failed<br />
                      [worker] Process exited with status code 1 (EADDRNOTAVAIL)
                    </code>
                  </div>
                </div>
              </div>
            )}

            {simStep >= 2 && (
              <div className="sim-bubble sim-agent">
                <span className="sim-avatar">⚡</span>
                <div className="sim-bubble-body">
                  <div className="sim-author">Devpane MCP Daemon</div>
                  <div className="sim-call">
                    <span className="sim-call-tag">AUTOMATED RECOVERY</span>
                    <code>devpane_restart_service(service: "worker")</code>
                  </div>
                  <div className="sim-response success">
                    <code>✓ Service &apos;worker&apos; restarted cleanly • PID 34502 • Health: Active</code>
                  </div>
                </div>
              </div>
            )}

            {simStep >= 3 && (
              <div className="sim-bubble sim-agent final">
                <span className="sim-avatar">🤖</span>
                <div className="sim-bubble-body">
                  <div className="sim-author">Cursor Agent</div>
                  <p>
                    I diagnosed the worker crash. The Redis dependency socket wasn&apos;t ready.
                    I restarted the worker service via Devpane MCP. All <strong>3 services</strong> are now healthy.
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>
      ) : (
        <div className="mcp-grid">
          <div className="mcp-card mcp-config-box">
            <div className="mcp-card-header">
              <div className="mcp-tabs">
                <button
                  className={`mcp-tab ${activeClient === "cursor" ? "active" : ""}`}
                  onClick={() => setActiveClient("cursor")}
                  type="button"
                >
                  Cursor (.cursor/mcp.json)
                </button>
                <button
                  className={`mcp-tab ${activeClient === "claude" ? "active" : ""}`}
                  onClick={() => setActiveClient("claude")}
                  type="button"
                >
                  Claude Desktop (config.json)
                </button>
              </div>
              <button className="copy-code-btn" onClick={copySnippet} type="button">
                {copied ? "Copied ✓" : "Copy JSON"}
              </button>
            </div>
            <pre className="code-block">
              <code>{cursorSnippet}</code>
            </pre>
            <div className="mcp-card-footer">
              <span>Runs locally over stdio • Zero telemetry • Sub-millisecond latency</span>
            </div>
          </div>

          <div className="mcp-card mcp-features-box">
            <h3>Tools Exposed to Your AI Agent</h3>
            <ul className="mcp-tool-list">
              <li>
                <div className="tool-name">
                  <code>devpane_list_services</code>
                </div>
                <p>Reports live process statuses, PIDs, ports, CPU%, and memory to the LLM.</p>
              </li>
              <li>
                <div className="tool-name">
                  <code>devpane_get_logs</code>
                </div>
                <p>Fetches recent logs with error filtering so the agent diagnoses crash stacktraces.</p>
              </li>
              <li>
                <div className="tool-name">
                  <code>devpane_restart_service</code>
                </div>
                <p>Triggers clean service reloads automatically after code edits are applied.</p>
              </li>
              <li>
                <div className="tool-name">
                  <code>devpane_rebuild_service</code>
                </div>
                <p>Triggers clean rebuild-and-restart cycles for compiled backends and services.</p>
              </li>
              <li>
                <div className="tool-name">
                  <code>devpane_sync_environment</code>
                </div>
                <p>Ensures <code>.env</code> keys and dependencies match before tests run.</p>
              </li>
            </ul>
          </div>
        </div>
      )}
    </div>
  );
}

function BenchmarkTable() {
  return (
    <div className="benchmark-section" id="benchmarks">
      <div className="section-head">
        <div className="tech-badge">PERFORMANCE BENCHMARKS</div>
        <h2>Lightweight native Rust. Real numbers.</h2>
        <p>
          Local development shouldn&apos;t require heavyweight VM layers or heavy Node.js runtimes.
        </p>
      </div>

      {/* Visual Comparison Cards */}
      <div className="benchmark-bars-grid">
        <div className="benchmark-bar-card">
          <div className="bench-card-head">
            <span className="bench-label">Startup Latency (Lower is Better)</span>
            <span className="bench-multiplier">800× Faster</span>
          </div>
          <div className="bench-rows">
            <div className="bench-row">
              <span className="bench-name devpane">Devpane</span>
              <div className="bench-track">
                <div className="bench-fill devpane-fill" style={{ width: "2%" }} />
              </div>
              <span className="bench-val good">~3 ms</span>
            </div>
            <div className="bench-row">
              <span className="bench-name">Overmind</span>
              <div className="bench-track">
                <div className="bench-fill" style={{ width: "12%" }} />
              </div>
              <span className="bench-val">~35 ms</span>
            </div>
            <div className="bench-row">
              <span className="bench-name">PM2</span>
              <div className="bench-track">
                <div className="bench-fill" style={{ width: "24%" }} />
              </div>
              <span className="bench-val">~420 ms</span>
            </div>
            <div className="bench-row">
              <span className="bench-name">Docker Compose</span>
              <div className="bench-track">
                <div className="bench-fill docker-fill" style={{ width: "100%" }} />
              </div>
              <span className="bench-val slow">~2,400 ms</span>
            </div>
          </div>
        </div>

        <div className="benchmark-bar-card">
          <div className="bench-card-head">
            <span className="bench-label">Idle RAM Consumption (Lower is Better)</span>
            <span className="bench-multiplier">99.4% Lighter</span>
          </div>
          <div className="bench-rows">
            <div className="bench-row">
              <span className="bench-name devpane">Devpane</span>
              <div className="bench-track">
                <div className="bench-fill devpane-fill" style={{ width: "3%" }} />
              </div>
              <span className="bench-val good">~11 MB</span>
            </div>
            <div className="bench-row">
              <span className="bench-name">Overmind</span>
              <div className="bench-track">
                <div className="bench-fill" style={{ width: "7%" }} />
              </div>
              <span className="bench-val">~28 MB</span>
            </div>
            <div className="bench-row">
              <span className="bench-name">PM2</span>
              <div className="bench-track">
                <div className="bench-fill" style={{ width: "16%" }} />
              </div>
              <span className="bench-val">~68 MB</span>
            </div>
            <div className="bench-row">
              <span className="bench-name">Docker Compose</span>
              <div className="bench-track">
                <div className="bench-fill docker-fill" style={{ width: "100%" }} />
              </div>
              <span className="bench-val slow">~1,800 MB</span>
            </div>
          </div>
        </div>
      </div>

      <div className="benchmark-table-wrapper">
        <table className="benchmark-table">
          <thead>
            <tr>
              <th>Architecture &amp; Feature</th>
              <th className="highlight-col">Devpane</th>
              <th>Docker Compose</th>
              <th>PM2</th>
              <th>Overmind</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><strong>Startup Latency</strong></td>
              <td className="highlight-col"><span className="metric-good">~3 ms</span></td>
              <td>~2,400 ms</td>
              <td>~420 ms</td>
              <td>~35 ms</td>
            </tr>
            <tr>
              <td><strong>Idle Memory Footprint</strong></td>
              <td className="highlight-col"><span className="metric-good">~11 MB</span></td>
              <td>~1,800 MB (VM)</td>
              <td>~68 MB</td>
              <td>~28 MB</td>
            </tr>
            <tr>
              <td><strong>Runtime Dependencies</strong></td>
              <td className="highlight-col"><span className="metric-good">100% Native Rust</span></td>
              <td>Docker Engine</td>
              <td>Node.js runtime</td>
              <td>Ruby / Tmux</td>
            </tr>
            <tr>
              <td><strong>Auto Port-Conflict Killer</strong></td>
              <td className="highlight-col"><span className="metric-good">Automatic</span></td>
              <td>Fails (EADDRINUSE)</td>
              <td>Manual</td>
              <td>Manual</td>
            </tr>
            <tr>
              <td><strong>Zero-Zombie Reaping &amp; Tree Teardown</strong></td>
              <td className="highlight-col"><span className="metric-good">Automatic (waitpid)</span></td>
              <td>Docker Daemon</td>
              <td>Leaves defunct zombies</td>
              <td>Manual signal trapping</td>
            </tr>
            <tr>
              <td><strong>Built-in AI MCP Server</strong></td>
              <td className="highlight-col"><span className="metric-good">Built-in (devpane mcp)</span></td>
              <td>No</td>
              <td>No</td>
              <td>No</td>
            </tr>
            <tr>
              <td><strong>Team .env &amp; Lockfile Sync</strong></td>
              <td className="highlight-col"><span className="metric-good">Built-in (devpane sync)</span></td>
              <td>No</td>
              <td>No</td>
              <td>No</td>
            </tr>
            <tr>
              <td><strong>Dual TUI + Standalone Desktop</strong></td>
              <td className="highlight-col"><span className="metric-good">Both Included</span></td>
              <td>CLI / Docker Desktop</td>
              <td>CLI only</td>
              <td>TUI only</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}

type StackTemplate = {
  id: string;
  name: string;
  badge: string;
  desc: string;
  ports: string;
  yaml: string;
};

const STACK_TEMPLATES: StackTemplate[] = [
  {
    id: "fullstack",
    name: "Fullstack Web & API",
    badge: "Next.js / Rust / Postgres",
    desc: "React/Vite frontend, hyper-fast Rust API backend, and managed PostgreSQL with automatic readiness dependency wait.",
    ports: ":5173 ➜ :8080 ➜ :5432",
    yaml: `# devpane.yaml — Fullstack Stack
name: fullstack-app
version: "1"

services:
  api:
    command: cargo run
    working_dir: ./backend
    ports: [8080]
    env_file: .env

  web:
    command: bun run dev
    working_dir: ./frontend
    ports: [5173]
    depends_on: [api]
    env_file: .env

  db:
    command: devpane-service postgres:16
    ports: [5432]
`,
  },
  {
    id: "ai",
    name: "AI Agent & Local LLM",
    badge: "FastAPI / Ollama / Qdrant",
    desc: "Autonomous coding agent workflow with local embedding vector database, local LLM inference, and FastAPI server.",
    ports: ":8000 ➜ :6333 ➜ :11434",
    yaml: `# devpane.yaml — AI Agent Stack
name: ai-agent-stack
version: "1"

services:
  agent-api:
    command: python -m uvicorn server:app --port 8000
    working_dir: ./agent
    ports: [8000]
    env_file: .env

  vector-db:
    command: qdrant --config-path ./qdrant.yaml
    ports: [6333]

  local-llm:
    command: ollama serve
    ports: [11434]
    depends_on: [vector-db]
`,
  },
  {
    id: "microservices",
    name: "Microservices & Celery",
    badge: "Go / Node / Python / Redis",
    desc: "Multi-language microservices architecture with background Celery queue workers and shared Redis broker.",
    ports: ":3000 ➜ :4001 ➜ :4002 ➜ :6379",
    yaml: `# devpane.yaml — Microservices Stack
name: microservices-stack
version: "1"

services:
  gateway:
    command: go run ./cmd/gateway
    ports: [3000]
    depends_on: [auth, billing]

  auth:
    command: bun run start:auth
    ports: [4001]

  billing:
    command: python -m billing_service
    ports: [4002]

  worker:
    command: python -m celery -A tasks worker -l INFO
    depends_on: [billing]
`,
  },
];

function StackConfigSection() {
  const [selectedId, setSelectedId] = useState<string>("fullstack");
  const [copied, setCopied] = useState<boolean>(false);
  const activeTemplate =
    STACK_TEMPLATES.find((t) => t.id === selectedId) || STACK_TEMPLATES[0];

  const handleCopy = () => {
    void navigator.clipboard.writeText(activeTemplate.yaml);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="stack-config-section" id="config">
      <div className="section-head">
        <div className="tech-badge">DECLARATIVE CONFIGURATION</div>
        <h2>One clean file replaces 200 lines of Docker.</h2>
        <p>
          Define your local dependencies in a human-readable <code>devpane.yaml</code>.
          Automatic port clearing, environment sync, and ordered dependency boots.
        </p>
      </div>

      <div className="stack-config-card">
        <div className="stack-tabs-bar">
          <div className="stack-tabs">
            {STACK_TEMPLATES.map((tmpl) => (
              <button
                key={tmpl.id}
                className={`stack-tab ${tmpl.id === selectedId ? "active" : ""}`}
                onClick={() => setSelectedId(tmpl.id)}
                type="button"
              >
                <span className="stack-tab-title">{tmpl.name}</span>
                <span className="stack-tab-badge">{tmpl.badge}</span>
              </button>
            ))}
          </div>

          <button className="stack-copy-btn" onClick={handleCopy} type="button">
            {copied ? "Copied ✓" : "Copy devpane.yaml"}
          </button>
        </div>

        <div className="stack-editor-body">
          <div className="stack-editor-meta">
            <span className="stack-file-tag">📄 devpane.yaml</span>
            <span className="stack-flow-ports">{activeTemplate.ports}</span>
          </div>
          <pre className="stack-code-block">
            <code>{activeTemplate.yaml}</code>
          </pre>
          <div className="stack-editor-footer">
            <span>✓ {activeTemplate.desc}</span>
            <span className="stack-run-hint">
              Run with: <code>devpane up</code>
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

function FeaturesGrid() {
  const pillars = [
    {
      badge: "TERMINAL WORKSPACE",
      kbd: "Ctrl+T / m",
      title: "Interactive VT100 Panes & Shells",
      desc: "Tmux-inspired ergonomics with zero configuration. Instant tab switching, collapsible sidebar with minimize/maximize (m/M), and interactive PTY shell sessions.",
    },
    {
      badge: "ZERO-ZOMBIE SUPERVISOR",
      kbd: "waitpid",
      title: "Zero-Orphan Process Supervisor",
      desc: "No more <defunct> zombie processes in ps. Devpane intercepts child exits, reaps zombies with non-blocking waitpid, and isolates process groups cleanly.",
    },
    {
      badge: "PORT CONFLICT RESOLVER",
      kbd: "kill-port",
      title: "Smart Port Conflict Killer",
      desc: "Say goodbye to EADDRINUSE errors forever. Devpane identifies lingering processes on required ports and safely reclaims them on startup.",
    },
    {
      badge: "AI MODEL CONTEXT PROTOCOL",
      kbd: "devpane mcp",
      title: "Model Context Protocol Server",
      desc: "Built-in JSON-RPC MCP server. Cursor, Claude Desktop, and Windsurf can read logs, check health, and restart services without human intervention.",
    },
    {
      badge: "TEAM ENVIRONMENT SYNC",
      kbd: "devpane sync",
      title: "Automatic .env & Dependency Sync",
      desc: "Compares .env with .env.example, flags missing keys, auto-installs missing dependencies, and hashes configurations into devpane.lock.",
    },
    {
      badge: "VIRTUALIZED LOG ENGINE",
      kbd: "sub-ms",
      title: "High-Performance Log Viewer",
      desc: "Stream tens of thousands of lines without lag. Viewport-windowed slice rendering and fast-path file checks maintain a buttery 60 FPS.",
    },
    {
      badge: "ORCHESTRATION & HEALTH",
      kbd: "depends_on",
      title: "Dependency Chains & Health Checks",
      desc: "Define dependency graphs in devpane.yaml. Dependent services wait for TCP port or HTTP endpoint readiness before launching.",
    },
    {
      badge: "STANDALONE GUI",
      kbd: "Tauri v2",
      title: "Zero-Click Standalone Desktop",
      desc: "Prefer a graphical UI? Run the native Tauri desktop app with an embedded supervisor. Real-time CPU%, RAM metrics, service badges, and live log tailing.",
    },
  ];

  return (
    <div className="features-section" id="features">
      <div className="section-head">
        <div className="tech-badge">CORE CAPABILITIES</div>
        <h2>Engineered for real developer stacks.</h2>
        <p>Everything you need to orchestrate fullstack applications locally without friction.</p>
      </div>

      <div className="pillars-grid">
        {pillars.map((p, idx) => (
          <div key={idx} className="pillar-card">
            <div className="pillar-top">
              <span className="pillar-badge">{p.badge}</span>
              <kbd className="pillar-kbd">{p.kbd}</kbd>
            </div>
            <h3>{p.title}</h3>
            <p>{p.desc}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function getInitialOS(): "macos" | "windows" | "linux" {
  if (typeof window !== "undefined" && window.navigator) {
    const userAgent = window.navigator.userAgent.toLowerCase();
    if (userAgent.includes("win")) return "windows";
    if (userAgent.includes("mac")) return "macos";
    if (userAgent.includes("linux")) return "linux";
  }
  return "macos";
}

function InstallWidget() {
  const [selectedOS, setSelectedOS] = useState<"macos" | "windows" | "linux">(
    getInitialOS,
  );
  const [method, setMethod] = useState<"cli" | "brew" | "desktop" | "cargo">(
    "cli",
  );
  const [copied, setCopied] = useState(false);

  const cargoCommand = "cargo install devpane";
  const brewCommand = "brew install Ranjeesh0701/tap/devpane";
  const repoReleases = "https://github.com/Ranjeesh0701/devpane/releases/latest";

  const getQuickCommand = () => {
    if (selectedOS === "windows") {
      return "irm https://raw.githubusercontent.com/Ranjeesh0701/devpane/main/scripts/install.ps1 | iex";
    }
    return "curl -fsSL https://raw.githubusercontent.com/Ranjeesh0701/devpane/main/scripts/install.sh | sh";
  };

  const copyText = (text: string) => {
    trackCopyInstall(selectedOS, text);
    void navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="install-section" id="install">
      <div className="section-head">
        <div className="tech-badge">ONE-LINE SETUP</div>
        <h2>Install Devpane on your machine.</h2>
        <p>Pre-built standalone binaries, package managers, or native desktop installers.</p>
      </div>

      <div className="install-card-main">
        <div className="os-select-bar">
          <button
            className={`os-btn ${selectedOS === "macos" ? "active" : ""}`}
            onClick={() => setSelectedOS("macos")}
            type="button"
          >
             macOS
          </button>
          <button
            className={`os-btn ${selectedOS === "windows" ? "active" : ""}`}
            onClick={() => setSelectedOS("windows")}
            type="button"
          >
            ⊞ Windows
          </button>
          <button
            className={`os-btn ${selectedOS === "linux" ? "active" : ""}`}
            onClick={() => setSelectedOS("linux")}
            type="button"
          >
            🐧 Linux
          </button>
        </div>

        <div className="method-select-bar">
          <button
            className={`method-btn ${method === "cli" ? "active" : ""}`}
            onClick={() => setMethod("cli")}
            type="button"
          >
            Quick Install
          </button>
          {selectedOS !== "windows" && (
            <button
              className={`method-btn ${method === "brew" ? "active" : ""}`}
              onClick={() => setMethod("brew")}
              type="button"
            >
              Homebrew
            </button>
          )}
          <button
            className={`method-btn ${method === "cargo" ? "active" : ""}`}
            onClick={() => setMethod("cargo")}
            type="button"
          >
            Cargo
          </button>
          <button
            className={`method-btn ${method === "desktop" ? "active" : ""}`}
            onClick={() => setMethod("desktop")}
            type="button"
          >
            Desktop GUI
          </button>
        </div>

        <div className="method-body">
          {method === "cli" && (
            <div>
              <div className="cmd-row">
                <code>{getQuickCommand()}</code>
                <button
                  className="copy-btn"
                  onClick={() => copyText(getQuickCommand())}
                  type="button"
                >
                  {copied ? "Copied ✓" : "Copy"}
                </button>
              </div>

              <div className="direct-downloads">
                <span className="direct-label">Or direct binary packages:</span>
                <div className="direct-links">
                  {selectedOS === "macos" && (
                    <>
                      <a
                        className="download-link"
                        href={`${repoReleases}/download/devpane-darwin-arm64.tar.gz`}
                        onClick={() => trackDownload("macos", "devpane-darwin-arm64.tar.gz")}
                      >
                        Apple Silicon (.tar.gz) ↓
                      </a>
                      <a
                        className="download-link"
                        href={`${repoReleases}/download/devpane-darwin-x64.tar.gz`}
                        onClick={() => trackDownload("macos", "devpane-darwin-x64.tar.gz")}
                      >
                        Intel Mac (.tar.gz) ↓
                      </a>
                    </>
                  )}
                  {selectedOS === "windows" && (
                    <a
                      className="download-link"
                      href={`${repoReleases}/download/devpane-windows-x64.zip`}
                      onClick={() => trackDownload("windows", "devpane-windows-x64.zip")}
                    >
                      Windows x64 (.zip) ↓
                    </a>
                  )}
                  {selectedOS === "linux" && (
                    <a
                      className="download-link"
                      href="https://github.com/Ranjeesh0701/devpane/releases"
                      target="_blank"
                      rel="noreferrer"
                      onClick={() => trackGithubClick("linux_releases")}
                    >
                      Browse Linux Releases on GitHub ↗
                    </a>
                  )}
                </div>
              </div>
            </div>
          )}

          {method === "brew" && (
            <div>
              <div className="cmd-row">
                <code>{brewCommand}</code>
                <button
                  className="copy-btn"
                  onClick={() => copyText(brewCommand)}
                  type="button"
                >
                  {copied ? "Copied ✓" : "Copy"}
                </button>
              </div>
            </div>
          )}

          {method === "cargo" && (
            <div>
              <div className="cmd-row">
                <code>{cargoCommand}</code>
                <button
                  className="copy-btn"
                  onClick={() => copyText(cargoCommand)}
                  type="button"
                >
                  {copied ? "Copied ✓" : "Copy"}
                </button>
              </div>
            </div>
          )}

          {method === "desktop" && (
            <div className="desktop-install-box">
              <p>
                Native standalone desktop app with an embedded background supervisor. Zero configuration required.
              </p>
              <div className="direct-links">
                {selectedOS === "macos" && (
                  <a
                    className="download-link primary-download"
                    href="https://github.com/Ranjeesh0701/devpane/releases"
                    target="_blank"
                    rel="noreferrer"
                    onClick={() => trackDownload("macos_desktop", "dmg")}
                  >
                    Download macOS .dmg Installer ↓
                  </a>
                )}
                {selectedOS === "windows" && (
                  <a
                    className="download-link primary-download"
                    href="https://github.com/Ranjeesh0701/devpane/releases"
                    target="_blank"
                    rel="noreferrer"
                    onClick={() => trackDownload("windows_desktop", "msi")}
                  >
                    Download Windows .msi Installer ↓
                  </a>
                )}
                {selectedOS === "linux" && (
                  <a
                    className="download-link primary-download"
                    href="https://github.com/Ranjeesh0701/devpane/releases"
                    target="_blank"
                    rel="noreferrer"
                    onClick={() => trackDownload("linux_desktop", "packages")}
                  >
                    Download Linux AppImage / .deb ↓
                  </a>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function App() {
  return (
    <div className="app-shell">
      <header className="navbar">
        <div className="nav-container">
          <a className="nav-brand" href="#home">
            <img
              src="/logo-dark.svg"
              alt="Devpane logo"
              className="brand-logo"
              width="28"
              height="28"
            />
            <span className="brand-name">devpane</span>
            <span className="brand-badge">v0.4.0</span>
          </a>

          <nav className="nav-links">
            <a href="#demo">Workspace</a>
            <a href="#benchmarks">Benchmarks</a>
            <a href="#config">Stacks</a>
            <a href="#mcp">AI MCP Server</a>
            <a href="#features">Features</a>
            <a href="#install">Install</a>
            <a
              href="https://github.com/Ranjeesh0701/devpane"
              target="_blank"
              rel="noreferrer"
              onClick={() => trackGithubClick("nav")}
              className="nav-github-link"
            >
              GitHub ↗
            </a>
          </nav>

          <a className="nav-cta" href="#install">
            Install CLI ↗
          </a>
        </div>
      </header>

      <main className="main-content">
        {/* HERO SECTION */}
        <section className="hero-section" id="home">
          <div className="hero-content">
            <div className="tech-badge hero-badge">
              <span>✦</span> 100% NATIVE RUST • ZERO RUNTIME BLOAT
            </div>

            <h1 className="hero-heading">
              The native process manager for multi-service stacks.
            </h1>

            <p className="hero-subheading">
              Stop juggling disconnected terminal tabs or 2 GB Docker containers.
              Devpane gives you <strong>~3ms instant startup</strong>, interactive VT100 terminal panes,
              smart port conflict killing, and a native Model Context Protocol (MCP) server for Cursor &amp; Claude.
            </p>

            <div className="hero-cta-group">
              <a className="btn btn-primary" href="#install">
                Get Started in 10s ↗
              </a>
              <a className="btn btn-secondary" href="#demo">
                View Interactive Workspace ↓
              </a>
            </div>

            <div className="hero-metrics-strip">
              <div className="metric-item">
                <span className="metric-val">&lt; 3ms</span>
                <span className="metric-label">Startup Latency</span>
              </div>
              <div className="metric-sep" />
              <div className="metric-item">
                <span className="metric-val">11 MB</span>
                <span className="metric-label">Memory Footprint</span>
              </div>
              <div className="metric-sep" />
              <div className="metric-item">
                <span className="metric-val">100%</span>
                <span className="metric-label">Native Rust</span>
              </div>
              <div className="metric-sep" />
              <div className="metric-item">
                <span className="metric-val">MCP</span>
                <span className="metric-label">AI Ready</span>
              </div>
            </div>
          </div>

          <div className="hero-terminal-wrapper" id="demo">
            <HeroPreview />
          </div>
        </section>

        {/* BENCHMARKS */}
        <BenchmarkTable />

        {/* STACK CONFIGURATION EXPLORER */}
        <StackConfigSection />

        {/* AI MCP SECTION */}
        <MCPConfigSection />

        {/* FEATURES GRID */}
        <FeaturesGrid />

        {/* INSTALLATION WIDGET */}
        <InstallWidget />

        {/* BOTTOM CTA */}
        <section className="bottom-cta-section">
          <div className="bottom-cta-card">
            <h2>Ready to simplify your local stack?</h2>
            <p>One declarative file. Near-instant startup. Full developer control.</p>
            <div className="bottom-cta-actions">
              <a className="btn btn-primary" href="#install">
                Install Devpane Now ↗
              </a>
              <a
                className="btn btn-secondary"
                href="https://github.com/Ranjeesh0701/devpane"
                target="_blank"
                rel="noreferrer"
                onClick={() => trackGithubClick("footer_cta")}
              >
                View on GitHub ↗
              </a>
            </div>
          </div>
        </section>
      </main>

      <footer className="footer">
        <div className="footer-container">
          <div className="footer-brand">
            <img src="/logo-dark.svg" alt="Devpane" width="22" height="22" />
            <span>devpane</span>
            <span className="footer-copy">© {new Date().getFullYear()} Ranjeesh • MIT Licensed</span>
          </div>

          <div className="footer-links">
            <a href="https://github.com/Ranjeesh0701/devpane" target="_blank" rel="noreferrer">
              GitHub
            </a>
            <a href="https://crates.io/crates/devpane" target="_blank" rel="noreferrer">
              Crates.io
            </a>
            <a href="#install">Install</a>
            <a href="#mcp">MCP Server</a>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default App;
