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

      <div className="benchmark-table-wrapper">
        <table className="benchmark-table">
          <thead>
            <tr>
              <th>Architecture &amp; Feature</th>
              <th className="highlight-col">Devpane (Native Rust)</th>
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

function MCPConfigSection() {
  const [activeClient, setActiveClient] = useState<"cursor" | "claude">("cursor");
  const [copied, setCopied] = useState(false);

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
          <h3>Agent Capabilities via Devpane</h3>
          <ul className="mcp-tool-list">
            <li>
              <div className="tool-name">
                <code>Real-Time Error Inspection</code>
              </div>
              <p>Agents inspect crash stacktraces and process logs to diagnose failures automatically.</p>
            </li>
            <li>
              <div className="tool-name">
                <code>Autonomous Service Restarts</code>
              </div>
              <p>Triggers clean, zero-zombie reloads right after your coding agent applies code changes.</p>
            </li>
            <li>
              <div className="tool-name">
                <code>Environment &amp; Dependency Verification</code>
              </div>
              <p>Ensures <code>.env</code> keys and service dependencies are ready before tests run.</p>
            </li>
            <li>
              <div className="tool-name">
                <code>Zero Terminal Hijacking</code>
              </div>
              <p>Communicates over stdio JSON-RPC without stealing your active window or shell focus.</p>
            </li>
          </ul>
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
            <a href="#mcp">AI MCP</a>
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
            <HeroTerminal />
          </div>
        </section>

        {/* BENCHMARKS */}
        <BenchmarkTable />

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
