import { useState, useMemo, useEffect } from "react";
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

const INITIAL_SERVICES: TuiService[] = [
  {
    id: "all",
    key: "0",
    name: "All services",
    icon: "≡",
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
      {
        num: 8,
        level: "INFO",
        prefix: "[api]",
        text: "POST /api/v1/services/sync 200 OK (1.1ms)",
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
        text: "Compiling api v0.4.1 (/Workspace/realworld-stack/api)",
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
      { num: 6, level: "INFO", text: "GET /api/v1/sessions 200 OK (0.6ms)" },
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
    url: "localhost:5173",
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
      { num: 6, level: "INFO", text: "GET /src/App.tsx 200 OK" },
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
      { num: 2, level: "INFO", text: "Connected to redis://127.0.0.1:6379/0" },
      {
        num: 3,
        level: "INFO",
        text: "Subscribed to stream 'events' (0 pending)",
      },
      { num: 4, level: "INFO", text: "✓ Heartbeat ACK • worker pool healthy" },
      { num: 5, level: "INFO", text: "Job #4089 processed in 14ms (success)" },
      { num: 6, level: "INFO", text: "Waiting for incoming event payloads..." },
    ],
  },
];

const CLI_COMMANDS = [
  {
    cmd: "devpane up",
    desc: "Start supervised stack & launch interactive TUI",
    output: [
      "✓ Loaded devpane.yaml (3 services declared)",
      "✓ Daemon socket verified: /var/run/devpane.sock",
      "✓ [port-killer] Checking port :8080, :5173... free",
      "✓ [db] Postgres ready (healthcheck port 5432 OK in 12ms)",
      "✓ [api] cargo run --bin server [PID 34470] bound to :8080",
      "✓ [web] bun run dev [PID 34475] bound to :5173",
      "✓ Interactive TUI mounted at 136x30 (Press '?' for help)",
    ],
  },
  {
    cmd: "devpane list",
    desc: "Inspect live processes, PIDs, memory, and status",
    output: [
      "NAME     STATUS    PID     PORT    MEMORY   CPU    UPTIME",
      "──────────────────────────────────────────────────────────",
      "db       healthy   34468   5432    44.1 MB  0.2%   12m 44s",
      "api      healthy   34470   8080    32.4 MB  0.8%   12m 42s",
      "web      healthy   34475   5173    45.6 MB  0.0%   12m 41s",
      "worker   healthy   34482   -        8.2 MB  0.0%   12m 41s",
      "──────────────────────────────────────────────────────────",
      "Total: 4 services running · 130.3 MB total memory footprint",
    ],
  },
  {
    cmd: "devpane sync",
    desc: "Verify .env completeness, dependencies, & hash lock",
    output: [
      "Scanning environment variables against .env.example...",
      "✓ DATABASE_URL present",
      "✓ REDIS_URL present",
      "✓ API_SECRET_KEY present",
      "Checking local package lockfiles against devpane.lock...",
      "✓ Cargo.lock: Up to date (hash matches)",
      "✓ package.json: 24 dependencies verified",
      "✓ All team workspace preconditions satisfied (in 9ms)",
    ],
  },
  {
    cmd: "devpane mcp",
    desc: "Start native Model Context Protocol JSON-RPC over stdio",
    output: [
      "devpane-mcp v0.4.1 starting up...",
      "Transport: stdio (JSON-RPC 2.0)",
      "Registered tools: [",
      "  'devpane_list_services',",
      "  'devpane_get_logs',",
      "  'devpane_restart_service',",
      "  'devpane_rebuild_service',",
      "  'devpane_sync_environment'",
      "]",
      "AI Coding Assistant connected (Cursor / Claude Desktop / Windsurf)",
    ],
  },
];

function getInitialOS(): "macos" | "windows" | "linux" {
  if (typeof window !== "undefined" && window.navigator) {
    const userAgent = window.navigator.userAgent.toLowerCase();
    if (userAgent.includes("win")) return "windows";
    if (userAgent.includes("mac")) return "macos";
    if (userAgent.includes("linux")) return "linux";
  }
  return "macos";
}

function CopyButton({
  copied,
  onCopy,
  label = "Copy to clipboard",
  className = "copy-accent-btn",
}: {
  copied: boolean;
  onCopy: () => void;
  label?: string;
  className?: string;
}) {
  return (
    <button
      className={`${className} ${copied ? "copied" : ""}`}
      onClick={onCopy}
      type="button"
      aria-label={copied ? "Copied" : label}
      title={copied ? "Copied to clipboard!" : label}
    >
      {copied ? (
        <svg
          width="14"
          height="14"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="copy-svg check-icon"
        >
          <polyline points="3.5 8.5 6.5 11.5 12.5 4.5" />
        </svg>
      ) : (
        <svg
          width="14"
          height="14"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="copy-svg"
        >
          <rect x="5.5" y="5.5" width="8" height="8" rx="1.5" />
          <path d="M10.5 5.5V3.5a1.5 1.5 0 0 0-1.5-1.5H3.5a1.5 1.5 0 0 0-1.5 1.5v5.5a1.5 1.5 0 0 0 1.5 1.5h2" />
        </svg>
      )}
    </button>
  );
}

export default function App() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [services, setServices] = useState<TuiService[]>(INITIAL_SERVICES);
  const [activeServiceId, setActiveServiceId] = useState<string>("web");
  const [logFilter, setLogFilter] = useState<"ALL" | "INFO" | "WARN" | "ERROR">(
    "ALL",
  );
  const [searchQuery, setSearchQuery] = useState("");
  const [wrapLogs, setWrapLogs] = useState(true);
  const [liveStreamActive, setLiveStreamActive] = useState(true);
  const [notificationToast, setNotificationToast] = useState<string | null>(
    null,
  );

  const [selectedCliCmd, setSelectedCliCmd] = useState(0);

  const [comparisonMode, setComparisonMode] = useState<"after" | "before">(
    "after",
  );

  const [portStep, setPortStep] = useState<
    "idle" | "conflict" | "killing" | "resolved"
  >("idle");

  const [selectedOS, setSelectedOS] = useState<"macos" | "windows" | "linux">(
    getInitialOS,
  );
  const [installMethod, setInstallMethod] = useState<
    "curl" | "brew" | "cargo" | "desktop"
  >("curl");
  const [copiedInstall, setCopiedInstall] = useState(false);

  const activeService = useMemo(() => {
    return services.find((s) => s.id === activeServiceId) || services[2];
  }, [services, activeServiceId]);

  const repoReleases =
    "https://github.com/Ranjeesh0701/devpane/releases/latest";
  const brewCommand = "brew install Ranjeesh0701/tap/devpane";
  const cargoCommand = "cargo install devpane";

  const getQuickCommand = () => {
    if (selectedOS === "windows") {
      return "irm https://raw.githubusercontent.com/Ranjeesh0701/devpane/main/scripts/install.ps1 | iex";
    }
    return "curl -fsSL https://raw.githubusercontent.com/Ranjeesh0701/devpane/main/scripts/install.sh | sh";
  };

  const copyInstallSnippet = (text: string) => {
    trackCopyInstall(selectedOS, text);
    void navigator.clipboard.writeText(text);
    setCopiedInstall(true);
    setTimeout(() => setCopiedInstall(false), 2000);
  };

  useEffect(() => {
    if (!liveStreamActive) return;

    const mockFeed = [
      {
        prefix: "[api]",
        text: "GET /api/v1/sessions 200 OK (0.5ms)",
        level: "INFO" as const,
      },
      {
        prefix: "[web]",
        text: "Vite HMR update: src/App.tsx in 14ms",
        level: "INFO" as const,
      },
      {
        prefix: "[worker]",
        text: "Queue worker ACK: stream 'events' processed (0 pending)",
        level: "INFO" as const,
      },
      {
        prefix: "[api]",
        text: "GET /health 200 OK (readiness check passed in 1ms)",
        level: "INFO" as const,
      },
      {
        prefix: "[api]",
        text: "POST /api/v1/services/sync 200 OK (0.9ms)",
        level: "INFO" as const,
      },
      {
        prefix: "[worker]",
        text: "Job #4092 processed in 11ms (success)",
        level: "INFO" as const,
      },
    ];

    let idx = 0;
    const interval = setInterval(() => {
      const item = mockFeed[idx % mockFeed.length];
      idx++;

      setServices((prev) =>
        prev.map((s) => {
          const match =
            s.id === "all" || s.id === item.prefix.replace(/[[\]]/g, "");
          if (match) {
            const nextNum = s.logs.length + 1;
            return {
              ...s,
              logs: [
                ...s.logs.slice(-14),
                {
                  num: nextNum,
                  level: item.level,
                  prefix: item.prefix,
                  text: item.text,
                },
              ],
            };
          }
          return s;
        }),
      );
    }, 3400);

    return () => clearInterval(interval);
  }, [liveStreamActive]);

  const handleRestartService = (targetId?: string) => {
    const id = targetId || activeServiceId;
    const timestamp = new Date().toLocaleTimeString();

    setNotificationToast(`↺ Restarting ${id}... compiled in 0.2s · Ready`);
    setTimeout(() => setNotificationToast(null), 2500);

    setServices((prev) =>
      prev.map((s) => {
        if (s.id === id || id === "all") {
          return {
            ...s,
            uptime: "0m 01s",
            logs: [
              ...s.logs,
              {
                num: s.logs.length + 1,
                level: "INFO",
                text: `[${s.id}] Graceful reload triggered at ${timestamp} — connections re-established (18ms)`,
              },
            ],
          };
        }
        return s;
      }),
    );
  };

  const handleCycleService = () => {
    const ids = ["all", "api", "web", "worker"];
    const nextIdx = (ids.indexOf(activeServiceId) + 1) % ids.length;
    setActiveServiceId(ids[nextIdx]);
  };

  const triggerPortSimulation = () => {
    setPortStep("conflict");
    setTimeout(() => {
      setPortStep("killing");
      setTimeout(() => {
        setPortStep("resolved");
        setTimeout(() => {
          setPortStep("idle");
        }, 2200);
      }, 1100);
    }, 1000);
  };

  const displayLogs = activeService.logs.filter((line) => {
    if (logFilter !== "ALL" && line.level !== logFilter) return false;
    if (searchQuery.trim()) {
      return line.text.toLowerCase().includes(searchQuery.toLowerCase());
    }
    return true;
  });

  return (
    <div className="devpane-root">
      <div className="bg-gradient-aurora" aria-hidden="true" />
      <div className="bg-grid-overlay" aria-hidden="true" />
      <div className="bg-radial-glow" aria-hidden="true" />

      {notificationToast && (
        <div className="action-toast">
          <span className="toast-icon">✓</span>
          <span>{notificationToast}</span>
        </div>
      )}

      <header className="dev-header">
        <div className="header-inner">
          <a className="brand-lockup" href="#home">
            <img
              src="/logo-dark.svg"
              alt="Devpane logo"
              className="brand-symbol"
              width="24"
              height="24"
            />
            <span className="brand-title">devpane</span>
            <span className="brand-version-pill">v0.4.1</span>
          </a>

          <nav className="header-nav-links">
            <a href="#cockpit">Workspace</a>
            <a href="#comparison">Why Devpane</a>
            <a href="#bento">Features</a>
            <a href="#install">Install</a>
          </nav>

          <div className="header-actions">
            <a
              href="https://github.com/Ranjeesh0701/devpane"
              target="_blank"
              rel="noreferrer"
              onClick={() => trackGithubClick("nav_star")}
              className="github-star-btn"
            >
              <svg
                className="gh-icon"
                viewBox="0 0 16 16"
                width="14"
                height="14"
                fill="currentColor"
              >
                <path d="M8 .25a.75.75 0 0 1 .673.418l1.882 3.815 4.21.612a.75.75 0 0 1 .416 1.279l-3.046 2.97.719 4.192a.75.75 0 0 1-1.088.791L8 12.347l-3.766 1.98a.75.75 0 0 1-1.088-.79l.72-4.194L.818 6.374a.75.75 0 0 1 .416-1.28l4.21-.611L7.327.668A.75.75 0 0 1 8 .25Z" />
              </svg>
              <span>GitHub</span>
            </a>

            <a className="nav-primary-install-btn" href="#install">
              Get Started ↗
            </a>

            <button
              className={`mobile-menu-toggle ${mobileMenuOpen ? "open" : ""}`}
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
              type="button"
              aria-label="Toggle navigation menu"
            >
              <span className="burger-bar" />
              <span className="burger-bar" />
              <span className="burger-bar" />
            </button>
          </div>
        </div>

        {mobileMenuOpen && (
          <div className="mobile-nav-panel">
            <nav className="mobile-nav-links">
              <a href="#cockpit" onClick={() => setMobileMenuOpen(false)}>
                Workspace TUI
              </a>
              <a href="#comparison" onClick={() => setMobileMenuOpen(false)}>
                Why Devpane
              </a>
              <a href="#bento" onClick={() => setMobileMenuOpen(false)}>
                Features
              </a>
              <a href="#install" onClick={() => setMobileMenuOpen(false)}>
                Installation
              </a>
              <a
                href="https://github.com/Ranjeesh0701/devpane"
                target="_blank"
                rel="noreferrer"
                onClick={() => {
                  trackGithubClick("mobile_nav");
                  setMobileMenuOpen(false);
                }}
                className="mobile-github-link"
              >
                GitHub Repository ↗
              </a>
            </nav>
            <div className="mobile-cta-wrap">
              <a
                className="btn-primary full-width"
                href="#install"
                onClick={() => setMobileMenuOpen(false)}
              >
                Install Devpane Free
              </a>
            </div>
          </div>
        )}
      </header>

      <main className="dev-main">
        <section className="hero-command-center" id="home">
          <div className="hero-announcement-chip">
            <span className="chip-sparkle">✦</span>
            <span className="chip-label">
              NATIVE RUST PROCESS MANAGER &amp; TERMINAL WORKSPACE
            </span>
            <span className="chip-arrow">→</span>
          </div>

          <h1 className="hero-headline">
            Run your full stack.
            <span className="headline-gradient-text">
              Never fight terminal tabs again.
            </span>
          </h1>

          <p className="hero-lede">
            Devpane orchestrates your backend, frontend, background queues, and
            AI coding agents inside a blazingly fast VT100 terminal workspace.
            Built in <strong>100% native Rust</strong> with zero zombie
            processes, automated port reclaiming, and native Model Context
            Protocol (MCP).
          </p>

          <div className="hero-quick-install-bar">
            <div className="install-snippet-input">
              <span className="terminal-dollar">$</span>
              <code>{getQuickCommand()}</code>
            </div>
            <CopyButton
              copied={copiedInstall}
              onCopy={() => copyInstallSnippet(getQuickCommand())}
              className="quick-copy-button"
              label="Copy install command"
            />
          </div>

          <div className="hero-button-row">
            <a className="btn-primary" href="#cockpit">
              Explore Live Workspace ↓
            </a>
            <a className="btn-secondary" href="#install">
              Download Desktop App  ⊞ 🐧
            </a>
          </div>

          <div className="hero-vitals-strip">
            <div className="vital-card">
              <span className="vital-value">&lt; 3 ms</span>
              <span className="vital-name">Cold Startup Latency</span>
              <span className="vital-desc">Compiled native Rust binary</span>
            </div>
            <div className="vital-card">
              <span className="vital-value">~11 MB</span>
              <span className="vital-name">Idle Memory Footprint</span>
              <span className="vital-desc">99% lighter than Docker VM</span>
            </div>
            <div className="vital-card">
              <span className="vital-value">0 Defunct</span>
              <span className="vital-name">Zombie Reaping</span>
              <span className="vital-desc">Non-blocking waitpid teardown</span>
            </div>
            <div className="vital-card">
              <span className="vital-value">Native MCP</span>
              <span className="vital-name">AI Agent Integration</span>
              <span className="vital-desc">Cursor, Claude &amp; Windsurf</span>
            </div>
          </div>
        </section>

        <section className="cockpit-section" id="cockpit">
          <div className="cockpit-container tui-cockpit-exclusive">
            <div className="cockpit-terminal-titlebar">
              <div className="terminal-dots-area">
                <span className="ctrl-dot red" />
                <span className="ctrl-dot yellow" />
                <span className="ctrl-dot green" />
                <span className="tui-window-title-text">
                  realworld-stack — devpane up — 136×30
                </span>
              </div>

              <div className="tui-search-input-wrap">
                <span className="search-icon">🔍</span>
                <input
                  type="text"
                  className="tui-search-field"
                  placeholder="Filter logs in real time..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                />
                {searchQuery && (
                  <button
                    className="clear-search-btn"
                    onClick={() => setSearchQuery("")}
                    type="button"
                  >
                    ✕
                  </button>
                )}
              </div>

              <div className="terminal-top-controls">
                <button
                  className={`live-stream-toggle ${liveStreamActive ? "active" : ""}`}
                  onClick={() => setLiveStreamActive(!liveStreamActive)}
                  type="button"
                  title={
                    liveStreamActive
                      ? "Click to pause stream"
                      : "Click to resume live stream"
                  }
                >
                  <span
                    className={`live-beacon-dot ${liveStreamActive ? "pulsing" : ""}`}
                  />
                  <span>{liveStreamActive ? "LIVE FEED" : "PAUSED"}</span>
                </button>
                <button
                  className="tui-quick-action-btn restart"
                  onClick={() => handleRestartService()}
                  type="button"
                  title="Simulate service restart"
                >
                  ↺ Restart (R)
                </button>
                <button
                  className="tui-quick-action-btn next"
                  onClick={handleCycleService}
                  type="button"
                  title="Switch to next pane"
                >
                  Tab Next ⇥
                </button>
              </div>
            </div>

            <div className="tui-subheader-bar">
              <div className="subheader-left">
                <span className="tui-brand-diamond">◆</span>
                <strong className="tui-brand-text">devpane</strong>
                <span className="tui-brand-ver">v0.4.1</span>
                <span className="tui-brand-sep">/</span>
                <span className="tui-project-tag">realworld-stack</span>
              </div>
              <div className="subheader-center">
                <span className="active-service-tag">{activeService.name}</span>
                <span className="active-health-tag">
                  ✓ {activeService.statusText}
                </span>
              </div>
              <div className="subheader-right">
                <span className="hint-pill">
                  Interactive · Click services or filters
                </span>
              </div>
            </div>

            <div className="tui-workspace-layout">
              <aside className="tui-sidebar-column">
                <div className="sidebar-header-row">
                  <span className="sidebar-label">SERVICES</span>
                  <span className="sidebar-count">
                    {services.length - 1} Running
                  </span>
                </div>

                <div className="sidebar-service-list">
                  {services.map((svc) => (
                    <button
                      key={svc.id}
                      className={`sidebar-svc-item ${svc.id === activeServiceId ? "selected" : ""}`}
                      onClick={() => setActiveServiceId(svc.id)}
                      type="button"
                    >
                      <div className="svc-top">
                        <span className="svc-icon">{svc.icon}</span>
                        <span className="svc-name">{svc.name}</span>
                        <span className="svc-key">[{svc.key}]</span>
                      </div>
                      <div className="svc-subline">{svc.subline}</div>
                    </button>
                  ))}
                </div>

                <div className="sidebar-meta-footer">
                  <div className="meta-line">
                    <span>Daemon IPC</span>
                    <span className="text-green">Connected (unix socket)</span>
                  </div>
                  <div className="meta-line">
                    <span>Auto-Reap</span>
                    <span className="text-green">Active (0 zombies)</span>
                  </div>
                  <div className="meta-line">
                    <span>Port Guard</span>
                    <span className="text-indigo">Armed :8080 :5173</span>
                  </div>
                </div>
              </aside>

              <div className="tui-main-column">
                <div className="telemetry-bar">
                  <div className="telemetry-card animated-metric">
                    <span className="telem-label">Memory</span>
                    <div className="telem-row">
                      <strong className="telem-val">
                        {activeService.memory}
                      </strong>
                      <span className="telem-bar green">
                        {activeService.memoryBar}
                      </span>
                    </div>
                  </div>

                  <div className="telemetry-card animated-metric">
                    <span className="telem-label">CPU Usage</span>
                    <div className="telem-row">
                      <strong className="telem-val">{activeService.cpu}</strong>
                      <span className="telem-bar indigo">
                        {activeService.cpuBar}
                      </span>
                    </div>
                  </div>

                  <div className="telemetry-card animated-metric">
                    <span className="telem-label">Service Uptime</span>
                    <div className="telem-row">
                      <strong className="telem-val">
                        {activeService.uptime}
                      </strong>
                      <span className="status-live-pill">
                        <span className="pulse-dot" />{" "}
                        {activeService.statusText}
                      </span>
                    </div>
                  </div>
                </div>

                <div className="log-viewer-pane">
                  <div className="log-viewer-header">
                    <div className="filter-chips">
                      <button
                        className={`filter-chip ${logFilter === "ALL" ? "active" : ""}`}
                        onClick={() => setLogFilter("ALL")}
                        type="button"
                      >
                        All ({displayLogs.length})
                      </button>
                      <button
                        className={`filter-chip ${logFilter === "INFO" ? "active" : ""}`}
                        onClick={() => setLogFilter("INFO")}
                        type="button"
                      >
                        Info
                      </button>
                      <button
                        className={`filter-chip ${logFilter === "WARN" ? "active" : ""}`}
                        onClick={() => setLogFilter("WARN")}
                        type="button"
                      >
                        Warn
                      </button>
                      <button
                        className={`filter-chip ${logFilter === "ERROR" ? "active" : ""}`}
                        onClick={() => setLogFilter("ERROR")}
                        type="button"
                      >
                        Error
                      </button>
                    </div>

                    <div className="log-actions">
                      <button
                        className="toggle-wrap-btn"
                        onClick={() => setWrapLogs(!wrapLogs)}
                        type="button"
                      >
                        Wrap: {wrapLogs ? "ON" : "OFF"}
                      </button>
                      {activeService.url && (
                        <a
                          className="direct-service-link"
                          href={`http://${activeService.url}`}
                          target="_blank"
                          rel="noreferrer"
                        >
                          http://{activeService.url} ↗
                        </a>
                      )}
                    </div>
                  </div>

                  <div
                    className={`log-stream-output ${wrapLogs ? "wrap-enabled" : ""}`}
                  >
                    {displayLogs.length === 0 ? (
                      <div className="empty-logs-placeholder">
                        <span>
                          No log entries match the current filter or search
                          query.
                        </span>
                      </div>
                    ) : (
                      displayLogs.map((log) => (
                        <div
                          key={log.num}
                          className="log-entry-row animated-log-entry"
                        >
                          <span className="entry-index">{log.num}</span>
                          <span
                            className={`entry-tag level-${log.level.toLowerCase()}`}
                          >
                            {log.level}
                          </span>
                          {log.prefix && (
                            <span className="entry-prefix">{log.prefix}</span>
                          )}
                          <span className="entry-msg">
                            {log.url
                              ? log.text.split(log.url).map((part, i, arr) => (
                                  <span key={i}>
                                    {part}
                                    {i < arr.length - 1 && (
                                      <span className="text-link-highlight">
                                        {log.url}
                                      </span>
                                    )}
                                  </span>
                                ))
                              : log.text}
                          </span>
                        </div>
                      ))
                    )}
                  </div>

                  <div className="log-stream-footer">
                    <span className="stream-badge">
                      <span className="pulse-dot" /> Streaming real-time stdout
                      / stderr via native Rust VT100
                    </span>
                  </div>
                </div>
              </div>
            </div>

            <div className="screen-statusbar">
              <div className="status-left">
                <span className="text-green">✓ 3/3 services healthy</span>
              </div>
              <div className="status-hotkeys">
                <button
                  className="hotkey-trigger"
                  onClick={handleCycleService}
                  type="button"
                >
                  <kbd>Tab</kbd> Next Service
                </button>
                <button
                  className="hotkey-trigger"
                  onClick={() => handleRestartService()}
                  type="button"
                >
                  <kbd>R</kbd> Restart
                </button>
                <button
                  className="hotkey-trigger"
                  onClick={() => setWrapLogs(!wrapLogs)}
                  type="button"
                >
                  <kbd>W</kbd> Toggle Wrap
                </button>
                <span>
                  <kbd>Ctrl+T</kbd> New Shell PTY
                </span>
                <span>
                  <kbd>Q</kbd> Exit
                </span>
              </div>
              <div className="status-right">
                <span className="text-indigo">Port Killer Active</span>
              </div>
            </div>
          </div>
        </section>

        <section className="comparison-section" id="comparison">
          <div className="section-head-centered">
            <span className="tech-badge">THE ARCHITECTURAL DIFFERENCE</span>
            <h2>Why developers ditch messy terminal windows.</h2>
            <p>
              Compare the friction of juggling disconnected processes against
              the unified control of Devpane.
            </p>
          </div>

          <div className="comparison-toggle-bar">
            <button
              className={`comp-toggle-btn ${comparisonMode === "after" ? "active" : ""}`}
              onClick={() => setComparisonMode("after")}
              type="button"
            >
              ✦ With Devpane (Unified Control)
            </button>
            <button
              className={`comp-toggle-btn ${comparisonMode === "before" ? "active" : ""}`}
              onClick={() => setComparisonMode("before")}
              type="button"
            >
              ✕ Without Devpane (The Chaos)
            </button>
          </div>

          <div className="comparison-view-container">
            {comparisonMode === "after" ? (
              <div className="comparison-card modern-flow">
                <div className="comp-card-badge positive">
                  THE MODERN DEVPANE WORKSPACE
                </div>
                <h3>One unified, declarative developer cockpit.</h3>
                <div className="comp-points-grid">
                  <div className="comp-point">
                    <span className="point-icon green">⚡</span>
                    <div>
                      <strong>Sub-3ms Instant Startup</strong>
                      <p>
                        Native compiled Rust binary starts up before your shell
                        prompt can even blink.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon green">🛡</span>
                    <div>
                      <strong>Automated Port Conflict Killer</strong>
                      <p>
                        Never see <code>EADDRINUSE 8080</code> again. Devpane
                        automatically reclaims lingering sockets.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon green">🪶</span>
                    <div>
                      <strong>~11 MB Total Memory Footprint</strong>
                      <p>
                        Lightweight native threads. No Node.js runtime bloat, no
                        heavy virtual machine layer.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon green">🤖</span>
                    <div>
                      <strong>AI Agent MCP Server</strong>
                      <p>
                        Claude Desktop and Cursor inspect errors and restart
                        services via structured local stdio.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon green">🔄</span>
                    <div>
                      <strong>Daemon Persistence</strong>
                      <p>
                        Processes stay alive across terminal windows. Reattach
                        to your workspace at any time.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon green">🧹</span>
                    <div>
                      <strong>Zero-Zombie Reaping</strong>
                      <p>
                        Child exit interception and non-blocking waitpid ensure
                        clean tree teardown on exit.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            ) : (
              <div className="comparison-card legacy-pain">
                <div className="comp-card-badge negative">
                  THE TRADITIONAL DEVELOPER STRUGGLE
                </div>
                <h3>Disconnected terminal tabs and silent crashes.</h3>
                <div className="comp-points-grid">
                  <div className="comp-point">
                    <span className="point-icon red">✕</span>
                    <div>
                      <strong>7 Proliferating Terminal Tabs</strong>
                      <p>
                        Scattered windows across multiple monitors with hidden
                        crashes and lost error logs.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon red">✕</span>
                    <div>
                      <strong>Endless EADDRINUSE Port Errors</strong>
                      <p>
                        Wasting minutes hunting down forgotten PIDs with{" "}
                        <code>lsof -i :8080</code> and <code>kill -9</code>.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon red">✕</span>
                    <div>
                      <strong>1.8 GB Docker VM Overhead</strong>
                      <p>
                        Spinning fans and sluggish file watchers just to run
                        local backend services.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon red">✕</span>
                    <div>
                      <strong>Orphaned Zombie Processes</strong>
                      <p>
                        Dead child processes left in{" "}
                        <code>&lt;defunct&gt;</code> state accumulating system
                        leaks.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon red">✕</span>
                    <div>
                      <strong>Accidental Terminal Window Closures</strong>
                      <p>
                        Close a tab by mistake and your database, backend, and
                        background queue all silently terminate.
                      </p>
                    </div>
                  </div>
                  <div className="comp-point">
                    <span className="point-icon red">✕</span>
                    <div>
                      <strong>Zero AI Agent Visibility</strong>
                      <p>
                        AI coding assistants have no way to inspect server logs
                        without stealing your active shell.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </section>

        <section className="bento-section" id="bento">
          <div className="section-head-centered">
            <span className="tech-badge">ENGINEERING HIGHLIGHTS</span>
            <h2>Architected for production developer stacks.</h2>
            <p>
              Every tool and capability you need to orchestrate complex services
              locally.
            </p>
          </div>

          <div className="bento-grid">
            <div className="bento-card span-2 bento-vt100">
              <svg
                className="bento-bg-watermark"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
                aria-hidden="true"
              >
                <rect x="2" y="3" width="20" height="18" rx="3" />
                <polyline points="6 9 10 12 6 15" />
                <line x1="12" y1="15" x2="17" y2="15" />
              </svg>

              <div className="bento-header">
                <span className="bento-pill">TERMINAL MULTIPLEXER</span>
                <span className="bento-subpill">VT100 / PTY Engine</span>
              </div>
              <h3>Interactive Terminal Panes &amp; Dedicated Shells</h3>
              <p>
                A tmux-inspired TUI with zero configuration. Switch service logs
                with <code>Tab</code>, collapse the sidebar with <code>m</code>,
                and spawn independent interactive PTY shell sessions with{" "}
                <code>Ctrl+T</code>.
              </p>
              <div className="bento-keys-preview">
                <div className="key-item">
                  <kbd>Tab</kbd>
                  <span>Next Pane</span>
                </div>
                <div className="key-item">
                  <kbd>Ctrl+T</kbd>
                  <span>New PTY Shell</span>
                </div>
                <div className="key-item">
                  <kbd>Ctrl+W</kbd>
                  <span>Close Tab</span>
                </div>
                <div className="key-item">
                  <kbd>PageUp</kbd>
                  <span>Scrollback</span>
                </div>
                <div className="key-item">
                  <kbd>/</kbd>
                  <span>Regex Search</span>
                </div>
                <div className="key-item">
                  <kbd>R</kbd>
                  <span>Hot Restart</span>
                </div>
              </div>
            </div>

            <div className="bento-card span-1 bento-zombie">
              <svg
                className="bento-bg-watermark"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
                aria-hidden="true"
              >
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                <polyline points="9 12 11 14 15 10" />
              </svg>

              <div className="bento-header">
                <span className="bento-pill">SUPERVISOR</span>
                <span className="bento-subpill">waitpid</span>
              </div>
              <h3>Zero-Zombie Reaping</h3>
              <p>
                No more <code>&lt;defunct&gt;</code> processes in{" "}
                <code>ps</code>. Devpane isolates process groups and intercepts
                child exit signals.
              </p>
              <div className="mini-tree-visual">
                <div className="tree-node daemon">
                  devpane-daemon (PID 1024)
                </div>
                <div className="tree-branch">
                  <div className="tree-node child">api :8080 [healthy]</div>
                  <div className="tree-node child">web :5173 [healthy]</div>
                </div>
              </div>
            </div>

            <div className="bento-card span-1 bento-port">
              <svg
                className="bento-bg-watermark"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
                aria-hidden="true"
              >
                <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" />
              </svg>

              <div className="bento-header">
                <span className="bento-pill">SOCKET GUARD</span>
                <span className="bento-subpill">kill-port</span>
              </div>
              <h3>Smart Port Reclaimer</h3>
              <p>
                Say goodbye to <code>EADDRINUSE</code> errors. Devpane
                identifies lingering processes on required ports and frees them
                automatically.
              </p>
              <div className="port-simulator-box">
                <div className="port-status-row">
                  <span className="port-name">:8080</span>
                  <span className={`port-indicator ${portStep}`}>
                    {portStep === "idle" && "✓ Socket Ready"}
                    {portStep === "conflict" && "⚠️ EADDRINUSE (PID 4821)"}
                    {portStep === "killing" && "↺ Reaping defunct socket..."}
                    {portStep === "resolved" && "✓ Freed in 3.4ms"}
                  </span>
                </div>
                <button
                  className="port-sim-btn"
                  onClick={triggerPortSimulation}
                  disabled={portStep !== "idle"}
                  type="button"
                >
                  {portStep === "idle"
                    ? "Simulate Conflict Test"
                    : "Reclaiming Port..."}
                </button>
              </div>
            </div>

            <div className="bento-card span-2 bento-mcp">
              <svg
                className="bento-bg-watermark"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
                aria-hidden="true"
              >
                <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
                <circle cx="12" cy="12" r="4" />
              </svg>

              <div className="bento-header">
                <span className="bento-pill">AI INTEGRATION</span>
                <span className="bento-subpill">JSON-RPC / stdio</span>
              </div>
              <h3>Model Context Protocol (MCP) Server Built-In</h3>
              <p>
                Allow AI coding assistants in Cursor, Claude Desktop, and
                Windsurf to supervise processes, fetch error stack traces, and
                restart services without hijacking your shell focus.
              </p>
              <div className="mcp-badges-row">
                <span className="mcp-tool-pill">
                  <code>devpane_list_services</code>
                </span>
                <span className="mcp-tool-pill">
                  <code>devpane_get_logs</code>
                </span>
                <span className="mcp-tool-pill">
                  <code>devpane_restart_service</code>
                </span>
                <span className="mcp-tool-pill">
                  <code>devpane_sync_environment</code>
                </span>
              </div>
            </div>

            <div className="bento-card span-1 bento-sync">
              <svg
                className="bento-bg-watermark"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
                aria-hidden="true"
              >
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>

              <div className="bento-header">
                <span className="bento-pill">TEAM SYNC</span>
                <span className="bento-subpill">devpane sync</span>
              </div>
              <h3>Automatic .env &amp; Lock Sync</h3>
              <p>
                Flags missing environment keys, automatically runs dependency
                updates, and generates verified checksums in{" "}
                <code>devpane.lock</code>.
              </p>
              <div className="env-diff-preview">
                <div className="diff-line plus">+ DATABASE_URL (synced)</div>
                <div className="diff-line plus">+ JWT_SECRET (verified)</div>
                <div className="diff-line check">
                  ✓ All lockfile hashes valid
                </div>
              </div>
            </div>

            <div className="bento-card span-1 bento-deps">
              <svg
                className="bento-bg-watermark"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
                aria-hidden="true"
              >
                <circle cx="6" cy="6" r="3" />
                <circle cx="6" cy="18" r="3" />
                <circle cx="18" cy="12" r="3" />
                <line x1="8.5" y1="7.5" x2="15.5" y2="10.5" />
                <line x1="8.5" y1="16.5" x2="15.5" y2="13.5" />
              </svg>

              <div className="bento-header">
                <span className="bento-pill">ORCHESTRATION</span>
                <span className="bento-subpill">depends_on</span>
              </div>
              <h3>Health Readiness Chains</h3>
              <p>
                Prevent startup race conditions. Services wait for upstream TCP
                sockets or HTTP endpoints before booting.
              </p>
              <div className="deps-chain-preview">
                <div className="dep-step">
                  <span className="dep-dot" />
                  <span className="dep-name">db :5432</span>
                  <span className="dep-status">TCP Ready</span>
                </div>
                <div className="dep-connector">↓</div>
                <div className="dep-step">
                  <span className="dep-dot" />
                  <span className="dep-name">api :8080 /health</span>
                  <span className="dep-status">200 OK</span>
                </div>
                <div className="dep-connector">↓</div>
                <div className="dep-step">
                  <span className="dep-dot" />
                  <span className="dep-name">web :5173</span>
                  <span className="dep-status">Mounted</span>
                </div>
              </div>
            </div>

            <div className="bento-card span-1 bento-logs">
              <svg
                className="bento-bg-watermark"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
                aria-hidden="true"
              >
                <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
              </svg>

              <div className="bento-header">
                <span className="bento-pill">PERFORMANCE</span>
                <span className="bento-subpill">sub-ms</span>
              </div>
              <h3>High-Performance Log Engine</h3>
              <p>
                Stream 50,000+ lines without UI stutter. Viewport-windowed slice
                rendering maintains a buttery 60 FPS even under heavy build
                logs.
              </p>
              <div className="fps-counter-badge">
                <span className="fps-val">60 FPS</span>
                <span className="fps-sub">Zero render lag at 100k lines</span>
              </div>
            </div>
          </div>
        </section>

        <section className="cli-tour-section" id="cli">
          <div className="section-head-centered">
            <span className="tech-badge">ERGONOMIC CLI</span>
            <h2>Take the CLI command tour.</h2>
            <p>
              Devpane provides an intuitive suite of commands for day-to-day
              workflow.
            </p>
          </div>

          <div className="cli-playground-wrapper">
            <div className="cli-command-selector">
              {CLI_COMMANDS.map((item, idx) => (
                <button
                  key={item.cmd}
                  className={`cli-cmd-button ${idx === selectedCliCmd ? "selected" : ""}`}
                  onClick={() => setSelectedCliCmd(idx)}
                  type="button"
                >
                  <span className="cmd-chevron">❯</span>
                  <div className="cmd-text-wrap">
                    <code>{item.cmd}</code>
                    <span className="cmd-desc">{item.desc}</span>
                  </div>
                </button>
              ))}
            </div>

            <div className="cli-terminal-display">
              <div className="terminal-header-mini">
                <div className="terminal-dots-mini">
                  <span className="dot red" />
                  <span className="dot yellow" />
                  <span className="dot green" />
                </div>
                <span className="terminal-cwd">
                  ~/Workspace/realworld-stack
                </span>
                <span className="terminal-badge-mini">
                  bash / zsh / powershell
                </span>
              </div>

              <div className="terminal-body-mini">
                <div className="prompt-line">
                  <span className="user-prompt">devpane-user@local:~$</span>
                  <span className="active-typed-cmd">
                    {CLI_COMMANDS[selectedCliCmd].cmd}
                  </span>
                </div>
                <div className="command-output-lines">
                  {CLI_COMMANDS[selectedCliCmd].output.map((line, i) => (
                    <div key={i} className="output-line">
                      {line}
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </section>

        <section className="benchmarks-section" id="benchmarks">
          <div className="section-head-centered">
            <span className="tech-badge">HARD BENCHMARKS</span>
            <h2>Lightweight native Rust. Real numbers.</h2>
            <p>
              Local development shouldn't require heavyweight VM layers or heavy
              Node.js runtimes.
            </p>
          </div>

          <div className="visual-benchmark-meters">
            <div className="meter-benchmark-card">
              <h4>Cold Startup Latency (Lower is better)</h4>
              <div className="bar-row">
                <span className="bar-name">Devpane (Rust)</span>
                <div className="bar-track">
                  <div className="bar-fill rust" style={{ width: "2%" }} />
                </div>
                <span className="bar-val text-green">~3 ms</span>
              </div>
              <div className="bar-row">
                <span className="bar-name">Overmind (Go/Tmux)</span>
                <div className="bar-track">
                  <div className="bar-fill" style={{ width: "12%" }} />
                </div>
                <span className="bar-val">~35 ms</span>
              </div>
              <div className="bar-row">
                <span className="bar-name">PM2 (Node.js)</span>
                <div className="bar-track">
                  <div className="bar-fill" style={{ width: "38%" }} />
                </div>
                <span className="bar-val">~420 ms</span>
              </div>
              <div className="bar-row">
                <span className="bar-name">Docker Compose</span>
                <div className="bar-track">
                  <div className="bar-fill docker" style={{ width: "95%" }} />
                </div>
                <span className="bar-val">~2,400 ms</span>
              </div>
            </div>

            <div className="meter-benchmark-card">
              <h4>Idle Memory Footprint (Lower is better)</h4>
              <div className="bar-row">
                <span className="bar-name">Devpane (Rust)</span>
                <div className="bar-track">
                  <div className="bar-fill rust" style={{ width: "3%" }} />
                </div>
                <span className="bar-val text-green">~11 MB</span>
              </div>
              <div className="bar-row">
                <span className="bar-name">Overmind</span>
                <div className="bar-track">
                  <div className="bar-fill" style={{ width: "10%" }} />
                </div>
                <span className="bar-val">~28 MB</span>
              </div>
              <div className="bar-row">
                <span className="bar-name">PM2 (Node.js)</span>
                <div className="bar-track">
                  <div className="bar-fill" style={{ width: "25%" }} />
                </div>
                <span className="bar-val">~68 MB</span>
              </div>
              <div className="bar-row">
                <span className="bar-name">Docker Desktop VM</span>
                <div className="bar-track">
                  <div className="bar-fill docker" style={{ width: "98%" }} />
                </div>
                <span className="bar-val">~1,800 MB</span>
              </div>
            </div>
          </div>

          <div className="benchmark-table-scroller">
            <table className="benchmark-data-table">
              <thead>
                <tr>
                  <th className="sticky-feature-col">
                    Architecture &amp; Feature
                  </th>
                  <th className="highlight-column">Devpane (Native Rust)</th>
                  <th>Docker Compose</th>
                  <th>PM2</th>
                  <th>Overmind</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td className="sticky-feature-col">
                    <strong>Startup Latency</strong>
                  </td>
                  <td className="highlight-column">
                    <span className="badge-good">~3 ms</span>
                  </td>
                  <td>~2,400 ms</td>
                  <td>~420 ms</td>
                  <td>~35 ms</td>
                </tr>
                <tr>
                  <td className="sticky-feature-col">
                    <strong>Idle Memory Footprint</strong>
                  </td>
                  <td className="highlight-column">
                    <span className="badge-good">~11 MB</span>
                  </td>
                  <td>~1,800 MB (VM)</td>
                  <td>~68 MB</td>
                  <td>~28 MB</td>
                </tr>
                <tr>
                  <td className="sticky-feature-col">
                    <strong>Runtime Dependencies</strong>
                  </td>
                  <td className="highlight-column">
                    <span className="badge-good">100% Native Rust</span>
                  </td>
                  <td>Docker Engine</td>
                  <td>Node.js runtime</td>
                  <td>Ruby / Tmux</td>
                </tr>
                <tr>
                  <td className="sticky-feature-col">
                    <strong>Auto Port-Conflict Killer</strong>
                  </td>
                  <td className="highlight-column">
                    <span className="badge-good">Automatic</span>
                  </td>
                  <td>Fails (EADDRINUSE)</td>
                  <td>Manual</td>
                  <td>Manual</td>
                </tr>
                <tr>
                  <td className="sticky-feature-col">
                    <strong>Zero-Zombie Reaping &amp; Tree Teardown</strong>
                  </td>
                  <td className="highlight-column">
                    <span className="badge-good">Automatic (waitpid)</span>
                  </td>
                  <td>Docker Daemon</td>
                  <td>Leaves defunct zombies</td>
                  <td>Manual signal trapping</td>
                </tr>
                <tr>
                  <td className="sticky-feature-col">
                    <strong>Built-in AI MCP Server</strong>
                  </td>
                  <td className="highlight-column">
                    <span className="badge-good">Built-in (devpane mcp)</span>
                  </td>
                  <td>No</td>
                  <td>No</td>
                  <td>No</td>
                </tr>
                <tr>
                  <td className="sticky-feature-col">
                    <strong>Team .env &amp; Lockfile Sync</strong>
                  </td>
                  <td className="highlight-column">
                    <span className="badge-good">Built-in (devpane sync)</span>
                  </td>
                  <td>No</td>
                  <td>No</td>
                  <td>No</td>
                </tr>
                <tr>
                  <td className="sticky-feature-col">
                    <strong>Dual TUI + Standalone Desktop</strong>
                  </td>
                  <td className="highlight-column">
                    <span className="badge-good">Both Included</span>
                  </td>
                  <td>CLI / Docker Desktop</td>
                  <td>CLI only</td>
                  <td>TUI only</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <section className="install-hub-section" id="install">
          <div className="section-head-centered">
            <span className="tech-badge">ZERO-FRICTION SETUP</span>
            <h2>Get Devpane running in 10 seconds.</h2>
            <p>
              Single binary downloads, Homebrew tap, Cargo, or native standalone
              desktop apps.
            </p>
          </div>

          <div className="install-hub-card">
            <div className="hub-os-tabs">
              <button
                className={`os-pill ${selectedOS === "macos" ? "active" : ""}`}
                onClick={() => setSelectedOS("macos")}
                type="button"
              >
                <span className="os-label-full">
                   macOS (Apple Silicon &amp; Intel)
                </span>
                <span className="os-label-short"> macOS</span>
              </button>
              <button
                className={`os-pill ${selectedOS === "windows" ? "active" : ""}`}
                onClick={() => setSelectedOS("windows")}
                type="button"
              >
                <span className="os-label-full">
                  ⊞ Windows (CMD &amp; PowerShell)
                </span>
                <span className="os-label-short">⊞ Windows</span>
              </button>
              <button
                className={`os-pill ${selectedOS === "linux" ? "active" : ""}`}
                onClick={() => setSelectedOS("linux")}
                type="button"
              >
                <span className="os-label-full">🐧 Linux (x86_64 / arm64)</span>
                <span className="os-label-short">🐧 Linux</span>
              </button>
            </div>

            <div className="hub-method-bar">
              <button
                className={`method-chip ${installMethod === "curl" ? "active" : ""}`}
                onClick={() => setInstallMethod("curl")}
                type="button"
              >
                Quick Install Script
              </button>
              {selectedOS !== "windows" && (
                <button
                  className={`method-chip ${installMethod === "brew" ? "active" : ""}`}
                  onClick={() => setInstallMethod("brew")}
                  type="button"
                >
                  Homebrew
                </button>
              )}
              <button
                className={`method-chip ${installMethod === "cargo" ? "active" : ""}`}
                onClick={() => setInstallMethod("cargo")}
                type="button"
              >
                Cargo (crates.io)
              </button>
              <button
                className={`method-chip ${installMethod === "desktop" ? "active" : ""}`}
                onClick={() => setInstallMethod("desktop")}
                type="button"
              >
                Standalone Desktop App
              </button>
            </div>

            <div className="hub-content-pane">
              {installMethod === "curl" && (
                <div className="method-view">
                  <div className="code-copy-row">
                    <code>{getQuickCommand()}</code>
                    <CopyButton
                      copied={copiedInstall}
                      onCopy={() => copyInstallSnippet(getQuickCommand())}
                      className="copy-accent-btn"
                      label="Copy quick install script"
                    />
                  </div>

                  <div className="binary-downloads-row">
                    <span className="downloads-label">
                      Direct standalone CLI binaries:
                    </span>
                    <div className="download-pills">
                      {selectedOS === "macos" && (
                        <>
                          <a
                            className="binary-dl-link"
                            href={`${repoReleases}/download/devpane-darwin-arm64.tar.gz`}
                            onClick={() =>
                              trackDownload(
                                "macos",
                                "devpane-darwin-arm64.tar.gz",
                              )
                            }
                          >
                            macOS Apple Silicon (.tar.gz) ↓
                          </a>
                          <a
                            className="binary-dl-link"
                            href={`${repoReleases}/download/devpane-darwin-x64.tar.gz`}
                            onClick={() =>
                              trackDownload(
                                "macos",
                                "devpane-darwin-x64.tar.gz",
                              )
                            }
                          >
                            macOS Intel (.tar.gz) ↓
                          </a>
                        </>
                      )}
                      {selectedOS === "windows" && (
                        <a
                          className="binary-dl-link"
                          href={`${repoReleases}/download/devpane-windows-x64.zip`}
                          onClick={() =>
                            trackDownload("windows", "devpane-windows-x64.zip")
                          }
                        >
                          Windows x64 (.zip) ↓
                        </a>
                      )}
                      {selectedOS === "linux" && (
                        <a
                          className="binary-dl-link"
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

              {installMethod === "brew" && (
                <div className="method-view">
                  <div className="code-copy-row">
                    <code>{brewCommand}</code>
                    <CopyButton
                      copied={copiedInstall}
                      onCopy={() => copyInstallSnippet(brewCommand)}
                      className="copy-accent-btn"
                      label="Copy brew install command"
                    />
                  </div>
                  <p className="method-hint">
                    Installs both <code>devpane</code> CLI and background daemon
                    binary via official tap.
                  </p>
                </div>
              )}

              {installMethod === "cargo" && (
                <div className="method-view">
                  <div className="code-copy-row">
                    <code>{cargoCommand}</code>
                    <CopyButton
                      copied={copiedInstall}
                      onCopy={() => copyInstallSnippet(cargoCommand)}
                      className="copy-accent-btn"
                      label="Copy cargo install command"
                    />
                  </div>
                  <p className="method-hint">
                    Compiles from source on your machine using standard Rust
                    toolchain (Rust 1.80+).
                  </p>
                </div>
              )}

              {installMethod === "desktop" && (
                <div className="method-view desktop-install-view">
                  <p className="desktop-desc">
                    Native standalone desktop application built with Tauri v2.
                    Features an embedded background supervisor, live CPU/RAM
                    meters, and graphical log tailing without any terminal
                    setup.
                  </p>
                  <div className="desktop-install-buttons">
                    {selectedOS === "macos" && (
                      <a
                        className="btn-primary"
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
                        className="btn-primary"
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
                        className="btn-primary"
                        href="https://github.com/Ranjeesh0701/devpane/releases"
                        target="_blank"
                        rel="noreferrer"
                        onClick={() =>
                          trackDownload("linux_desktop", "packages")
                        }
                      >
                        Download Linux AppImage / .deb ↓
                      </a>
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        </section>

        <section className="grand-cta-section">
          <div className="grand-cta-box">
            <span className="grand-cta-badge">
              READY TO TRANSFORM YOUR WORKSPACE?
            </span>
            <h2>Take control of your local processes today.</h2>
            <p>
              Free, open source, and built in native Rust. One declarative file
              is all it takes to unify your stack.
            </p>
            <div className="grand-cta-actions">
              <a className="btn-primary large" href="#install">
                Get Started in 10s ↗
              </a>
              <a
                className="btn-secondary large"
                href="https://github.com/Ranjeesh0701/devpane"
                target="_blank"
                rel="noreferrer"
                onClick={() => trackGithubClick("footer_cta")}
              >
                Star on GitHub ★
              </a>
            </div>
          </div>
        </section>
      </main>

      <footer className="dev-footer">
        <div className="footer-grid-container">
          <div className="footer-col brand-col">
            <div className="footer-brand-title">
              <img src="/logo-dark.svg" alt="Devpane" width="22" height="22" />
              <span>devpane</span>
              <span className="footer-ver-badge">v0.4.1</span>
            </div>
            <p className="footer-tagline">
              A clearer view of the processes behind your code. Native Rust
              process supervisor and terminal workspace.
            </p>
            <span className="footer-copyright">
              © {new Date().getFullYear()} Ranjeesh · MIT Licensed
            </span>
          </div>

          <div className="footer-col">
            <h5>Product</h5>
            <div className="footer-links-list">
              <a href="#cockpit">Interactive Cockpit</a>
              <a href="#comparison">Why Devpane</a>
              <a href="#bento">Superpowers</a>
              <a href="#cli">CLI Tour</a>
              <a href="#benchmarks">Benchmarks</a>
            </div>
          </div>

          <div className="footer-col">
            <h5>Ecosystem</h5>
            <div className="footer-links-list">
              <a
                href="https://github.com/Ranjeesh0701/devpane"
                target="_blank"
                rel="noreferrer"
              >
                GitHub Repository ↗
              </a>
              <a
                href="https://crates.io/crates/devpane"
                target="_blank"
                rel="noreferrer"
              >
                Crates.io Package ↗
              </a>
              <a
                href="https://github.com/Ranjeesh0701/devpane/releases"
                target="_blank"
                rel="noreferrer"
              >
                GitHub Releases ↗
              </a>
              <a href="#install">Homebrew Tap</a>
            </div>
          </div>

          <div className="footer-col">
            <h5>Protocol &amp; Tools</h5>
            <div className="footer-links-list">
              <a
                href="https://github.com/Ranjeesh0701/devpane#ai-agent-integration-mcp-server"
                target="_blank"
                rel="noreferrer"
              >
                Model Context Protocol (MCP)
              </a>
              <a
                href="https://github.com/Ranjeesh0701/devpane#1-initialize-or-configure-devpaneyaml"
                target="_blank"
                rel="noreferrer"
              >
                devpane.yaml Specification
              </a>
              <a href="#install">Desktop Application (Tauri)</a>
              <a
                href="https://github.com/Ranjeesh0701/devpane/blob/main/LICENSE"
                target="_blank"
                rel="noreferrer"
              >
                MIT License
              </a>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}
