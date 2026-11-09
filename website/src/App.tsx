import { useState } from "react";
import "./App.css";
import {
  trackDownload,
  trackCopyInstall,
  trackGithubClick,
} from "./lib/analytics";

type ServiceTab = {
  id: string;
  name: string;
  port?: number;
  logs: { text: string; color?: string }[];
};

const DEMO_SERVICES: ServiceTab[] = [
  {
    id: "backend",
    name: "backend",
    port: 8080,
    logs: [
      { text: "[devpane:daemon] Service 'backend' started (PID 58120) • 0.8% CPU • 32.4 MB", color: "muted" },
      { text: "   Compiling devpane-server v0.3.1 (/Workspace/devpane)", color: "indigo" },
      { text: "   Finished `dev` profile in 0.74s", color: "success" },
      { text: "[server] Listening on http://localhost:8080", color: "white" },
      { text: "[health] Readiness check OK: HTTP 200 at /health (8ms)", color: "success" },
      { text: "[ipc] Socket active at /tmp/devpane.sock", color: "muted" },
      { text: "[auth] Session JWT verifier initialized from .env", color: "white" },
      { text: "[db] Connection pool verified: 10 connections idle", color: "white" },
      { text: "GET /api/v1/sessions 200 OK (0.6ms)", color: "success" },
      { text: "POST /api/v1/services/sync 200 OK (1.1ms)", color: "success" },
    ],
  },
  {
    id: "frontend",
    name: "frontend",
    port: 5173,
    logs: [
      { text: "[devpane:daemon] Service 'frontend' started (PID 58124) • 1.1% CPU • 49.2 MB", color: "muted" },
      { text: "VITE v6.0.4 ready in 128 ms", color: "white" },
      { text: "  ➜  Local:   http://localhost:5173/", color: "accent" },
      { text: "  ➜  Network: use --host to expose", color: "muted" },
      { text: "  ➜  press h + enter to show help", color: "muted" },
      { text: "[hmr] devpane watcher synchronized (2 watched paths)", color: "success" },
      { text: "[vite] page reload /src/App.tsx (22ms)", color: "indigo" },
    ],
  },
  {
    id: "worker",
    name: "worker",
    logs: [
      { text: "[devpane:daemon] Service 'worker' started (PID 58130) • 0.4% CPU • 26.1 MB", color: "muted" },
      { text: "[queue:redis] Connected to redis://localhost:6379/0", color: "white" },
      { text: "[worker:1] Subscribed to topic 'event-stream' (0 pending)", color: "white" },
      { text: "[worker:1] Heartbeat ACK • worker pool healthy", color: "success" },
      { text: "[worker:1] Job #4089 processed in 14ms (status: success)", color: "success" },
      { text: "[worker:1] Waiting for incoming event payloads...", color: "muted" },
    ],
  },
  {
    id: "shell",
    name: "🐚 shell",
    logs: [
      { text: "devpane-workspace % devpane list", color: "white" },
      { text: "  ● backend   PID 58120   RUNNING   :8080    32.4 MB", color: "success" },
      { text: "  ● frontend  PID 58124   RUNNING   :5173    49.2 MB", color: "success" },
      { text: "  ● worker    PID 58130   RUNNING            26.1 MB", color: "success" },
      { text: "", color: "white" },
      { text: "devpane-workspace % devpane mcp --status", color: "white" },
      { text: "  ✓ Model Context Protocol (MCP) server ready for Cursor & Claude", color: "accent" },
      { text: "  ✓ IPC socket active • 3 services supervised", color: "muted" },
      { text: "", color: "white" },
      { text: "devpane-workspace % _", color: "accent" },
    ],
  },
];

function SimplifiedTerminal() {
  const [activeIdx, setActiveIdx] = useState(0);
  const activeService = DEMO_SERVICES[activeIdx];

  return (
    <div className="terminal-card">
      <div className="terminal-header">
        <div className="terminal-dots" aria-hidden="true">
          <span className="dot dot-red" />
          <span className="dot dot-yellow" />
          <span className="dot dot-green" />
        </div>
        <div className="terminal-title">
          <span className="title-cmd">devpane up</span>
        </div>
        <div className="terminal-live-tag">
          <span className="pulse-dot" /> LIVE DAEMON
        </div>
      </div>

      <div className="terminal-tabs-bar">
        {DEMO_SERVICES.map((svc, i) => (
          <button
            key={svc.id}
            className={`terminal-tab ${i === activeIdx ? "active" : ""}`}
            onClick={() => setActiveIdx(i)}
            type="button"
          >
            <span className="tab-dot" />
            <span className="tab-name">{svc.name}</span>
            {svc.port && <span className="tab-port">:{svc.port}</span>}
          </button>
        ))}
      </div>

      <div className="terminal-viewport">
        <div className="terminal-lines">
          {activeService.logs.map((line, idx) => (
            <div key={idx} className={`terminal-line line-${line.color || "white"}`}>
              {line.text}
            </div>
          ))}
        </div>
      </div>

      <div className="terminal-footer">
        <div className="shortcut-hints">
          <span>Tab: Switch</span>
          <span>•</span>
          <span>Ctrl+T: Shell</span>
          <span>•</span>
          <span>r: Restart</span>
          <span>•</span>
          <span>/: Search</span>
          <span>•</span>
          <span>q: Quit</span>
        </div>
        <span className="terminal-status">● 3 running</span>
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
                <code>devpane_sync_environment</code>
              </div>
              <p>Ensures <code>.env</code> keys and dependencies match before tests run.</p>
            </li>
          </ul>
        </div>
      </div>
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

function FeaturesGrid() {
  const pillars = [
    {
      badge: "TERMINAL WORKSPACE",
      kbd: "Ctrl+T",
      title: "Interactive VT100 Panes & Shells",
      desc: "Tmux-inspired ergonomics with zero configuration. Smooth scrollback, instant tab switching, search filtering, and interactive PTY shell sessions.",
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
      badge: "ORCHESTRATION & HEALTH",
      kbd: "depends_on",
      title: "Dependency Chains & Health Checks",
      desc: "Define dependency graphs in devpane.yaml. Dependent services wait for TCP port or HTTP endpoint readiness before launching.",
    },
    {
      badge: "TEAM ENVIRONMENT SYNC",
      kbd: "devpane sync",
      title: "Automatic .env & Dependency Sync",
      desc: "Compares .env with .env.example, flags missing keys, auto-installs missing dependencies, and hashes configurations into devpane.lock.",
    },
    {
      badge: "STANDALONE GUI",
      kbd: "Tauri v2",
      title: "Zero-Click Standalone Desktop",
      desc: "Prefer a graphical UI? Run the native Tauri desktop app with an embedded supervisor. Real-time CPU%, RAM metrics, and live log tailing.",
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
            <span className="brand-badge">v0.3.1</span>
          </a>

          <nav className="nav-links">
            <a href="#demo">Workspace</a>
            <a href="#benchmarks">Benchmarks</a>
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
            <SimplifiedTerminal />
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
