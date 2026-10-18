import { useState } from "react";
import "./App.css";
import { trackDownload, trackCopyInstall, trackGithubClick } from "./lib/analytics";

const services = [
  { name: "backend", status: "running", command: "cargo run", active: true },
  { name: "frontend", status: "running", command: "bun run dev", active: false },
  { name: "worker", status: "running", command: "bun run worker", active: false },
  { name: "Shell 1", status: "interactive", command: "zsh", active: false },
] as const;

const features = [
  {
    number: "01",
    icon: "⌁",
    title: "Interactive Terminal Workspace",
    description:
      "A tmux-inspired TUI with VT100 terminal emulation. Inspect live service logs, switch tabs with Tab, and spawn interactive shells with Ctrl+T.",
    tag: "TERMINAL WORKSPACE",
  },
  {
    number: "02",
    icon: "⚡",
    title: "Resource Metrics & Health Checks",
    description:
      "Real-time CPU % and RAM monitoring per process. Define depends_on startup chains with port-readiness health checks and auto port-conflict killing.",
    tag: "METRICS & HEALTH",
  },
  {
    number: "03",
    icon: "▤",
    title: "Declarative Config & Auto .env",
    description:
      "Configure your stack in devpane.yaml. Automatically loads .env and .env.local with zero external wrappers or direnv required.",
    tag: "LOCAL ORCHESTRATION",
  },
  {
    number: "04",
    icon: "🖥",
    title: "Zero-Click Standalone Desktop",
    description:
      "Native Tauri desktop app with an embedded background daemon. Open the app and it works immediately without manual CLI or terminal setup.",
    tag: "DESKTOP DASHBOARD",
  },
  {
    number: "05",
    icon: "🔍",
    title: "In-Pane Log Search & Filter",
    description:
      "Press / in any service log tab to filter log streams live with real-time match counters. Esc clears filters instantly without losing position.",
    tag: "LOG SEARCH",
  },
  {
    number: "06",
    icon: "⌘",
    title: "AI-Agent & IPC Ready",
    description:
      "Whether you manage processes manually or AI agents launch them via IPC, Devpane keeps session states structured, persisted, and observable.",
    tag: "AGENT-COMPATIBLE",
  },
];

function ArrowIcon() {
  return <span aria-hidden="true">↗</span>;
}

function ProcessPreview() {
  return (
    <div className="app-window terminal-window">
      <div className="window-titlebar">
        <div className="window-dots" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
        <span className="window-title">devpane up - interactive workspace</span>
        <span className="window-version">v0.2.0</span>
      </div>

      <div className="terminal-body">
        <div className="terminal-context">
          <span className="agent-indicator">
            <span className="agent-indicator-dot" />
            LIVE DAEMON SUPERVISOR
          </span>
          <span className="terminal-muted">devpane.yaml loaded • .env cascaded</span>
        </div>

        {/* Tab bar header */}
        <div style={{ display: "flex", gap: "6px", margin: "14px 0 10px", flexWrap: "wrap" }}>
          {services.map((svc) => (
            <span
              key={svc.name}
              style={{
                padding: "3px 9px",
                borderRadius: "4px",
                fontSize: "10px",
                fontFamily: "monospace",
                background: svc.active ? "rgb(139 156 255 / 22%)" : "rgb(255 255 255 / 4%)",
                border: svc.active ? "1px solid rgb(139 156 255 / 40%)" : "1px solid transparent",
                color: svc.active ? "#c9d2ff" : "#888c9f",
                fontWeight: svc.active ? 600 : 400,
              }}
            >
              <span
                style={{
                  color: svc.status === "running" ? "var(--success)" : "#888c9f",
                  marginRight: "5px",
                }}
              >
                ●
              </span>
              {svc.name}
            </span>
          ))}
        </div>

        {/* Live log pane emulation */}
        <div
          style={{
            background: "#101115",
            border: "1px solid var(--border)",
            borderRadius: "6px",
            padding: "12px",
            fontFamily: "monospace",
            fontSize: "11px",
            lineHeight: 1.6,
            minHeight: "135px",
          }}
        >
          <div style={{ color: "var(--subtle)" }}>[devpane:daemon] Service 'backend' started (PID 48921) • 0.8% CPU • 34.2 MB</div>
          <div style={{ color: "#aab5ff" }}>   Compiling devpane v0.2.0 (/Workspace/devpane)</div>
          <div style={{ color: "var(--success)" }}>    Finished `dev` profile in 0.82s</div>
          <div style={{ color: "#e2e4ed" }}>[server] Listening on http://127.0.0.1:8080</div>
          <div style={{ color: "#e2e4ed" }}>[server] IPC socket ready at /tmp/devpane.sock</div>
          <div style={{ color: "#55d6a2" }}>
            [server] Connected to 2 service watchers <span className="cursor" />
          </div>
        </div>

        <div className="terminal-bottom">
          <div className="terminal-summary">
            <span className="terminal-green">● 3 running</span>
            <span className="terminal-muted">Tab: Switch • /: Search • Ctrl+T: Shell • Ctrl+W: Close • q: Quit</span>
          </div>
          <span className="preview-label">INTERACTIVE TUI</span>
        </div>
      </div>
    </div>
  );
}

function WorkflowDiagram() {
  return (
    <div className="workflow-diagram">
      <div className="workflow-node">
        <div className="workflow-node-icon agent-node-icon">
          <span>✳</span>
        </div>
        <div className="workflow-node-copy">
          <strong>AI coding agent</strong>
          <span>Writes code and requests processes</span>
        </div>
        <span className="workflow-node-number">01</span>
      </div>

      <div className="workflow-connector">
        <span />
        <span className="workflow-connector-label">Process commands</span>
      </div>

      <div className="workflow-node workflow-node-highlight">
        <div className="workflow-node-icon devbox-node-icon">
          <span>d</span>
        </div>
        <div className="workflow-node-copy">
          <strong>Devpane</strong>
          <span>Runs and tracks development processes</span>
        </div>
        <span className="workflow-node-number">02</span>
      </div>

      <div className="workflow-connector">
        <span />
        <span className="workflow-connector-label">
          Process state and output
        </span>
      </div>

      <div className="workflow-node">
        <div className="workflow-node-icon dashboard-node-icon">
          <span>▦</span>
        </div>
        <div className="workflow-node-copy">
          <strong>Developer dashboard</strong>
          <span>See status, inspect logs, stay in control</span>
        </div>
        <span className="workflow-node-number">03</span>
      </div>
    </div>
  );
}

function App() {
  return (
    <div className="site-shell">
      <header className="site-header">
        <a className="brand" href="#home" aria-label="Devpane home">
          <span className="brand-mark">d</span>
          <span>devpane</span>
        </a>

        <nav className="main-nav" aria-label="Main navigation">
          <a href="#features">Why Devpane</a>
          <a href="#workflow">How it works</a>
          <a href="#install">Install</a>
          <a
            href="https://github.com/Ranjeesh0701/devpane"
            target="_blank"
            rel="noreferrer"
            onClick={() => trackGithubClick("header_nav")}
          >
            GitHub <ArrowIcon />
          </a>
        </nav>

        <a className="header-cta" href="#install">
          Install Devpane <ArrowIcon />
        </a>
      </header>

      <main>
        <section className="hero section-container" id="home">
          <div className="hero-copy">
            <div className="eyebrow">
              <span className="eyebrow-dot" />
              BUILT WITH RUST • LIGHTWEIGHT & FAST
            </div>

            <h1>
              Run your stack.
              <br />
              <span>Stay in control.</span>
            </h1>

            <p className="hero-description">
              Stop juggling disconnected terminal tabs. Devpane is a blazingly fast
              process manager and terminal workspace built in native Rust - giving you
              instant startup, minimal RAM usage, and full control over your development stack.
            </p>

            <div className="hero-actions">
              <a className="button button-primary" href="#workflow">
                See how it works <ArrowIcon />
              </a>
              <a className="button button-secondary" href="#features">
                Why Devpane <span aria-hidden="true">↓</span>
              </a>
            </div>

            <div className="hero-note">
              <span className="note-check">✓</span>
              100% Native Rust core with near-zero overhead, interactive TUI, and desktop dashboard.
            </div>
          </div>

          <div className="hero-visual">
            <div className="visual-glow" />
            <div className="visual-label">
              <span className="visual-label-dot" />
              YOUR DEVELOPMENT WORKSPACE
            </div>
            <ProcessPreview />
            <div className="floating-chip">
              <span className="status-dot" />
              Your processes, at a glance
            </div>
          </div>
        </section>

        <section className="principles">
          <div className="section-container principles-inner">
            <p>FROM LOCAL SCRIPTS TO MULTI-SERVICE WORKSPACES</p>
            <div className="principle-items">
              <span>
                <i>01</i> Launch
              </span>
              <span>
                <i>02</i> Supervise
              </span>
              <span>
                <i>03</i> Interact
              </span>
            </div>
          </div>
        </section>

        <section className="features section-container" id="features">
          <div className="section-heading">
            <div className="eyebrow">THE LOCAL RUNTIME LAYER</div>
            <h2>Full visibility over your entire stack.</h2>
            <p>
              Whether you're developing manually or pairing with AI coding agents,
              Devpane keeps every server, worker, and task manageable from a single pane.
            </p>
          </div>

          <div className="feature-grid">
            {features.map((feature) => (
              <article className="feature-card" key={feature.number}>
                <div className="feature-card-top">
                  <span className="feature-icon">{feature.icon}</span>
                  <span className="feature-number">{feature.number}</span>
                </div>
                <span className="feature-tag">{feature.tag}</span>
                <h3>{feature.title}</h3>
                <p>{feature.description}</p>
                <a href="#workflow" className="feature-link">
                  How it fits <ArrowIcon />
                </a>
              </article>
            ))}
          </div>
        </section>

        <section className="workflow-section" id="workflow">
          <div className="section-container workflow-inner">
            <div className="workflow-copy">
              <div className="eyebrow">A BETTER DEVELOPMENT LOOP</div>
              <h2>Built for developers. Ready for AI agents.</h2>
              <p>
                Run <code style={{ color: "var(--accent)" }}>devpane up</code> to launch your whole environment into interactive
                terminal tabs. Coding agents can also invoke Devpane commands via IPC to spin up servers,
                run migrations, and inspect logs - while you stay in full control.
              </p>

              <div className="workflow-benefits">
                <div>
                  <span className="benefit-check">✓</span>
                  <span>Interactive VT100 terminal panes & shells</span>
                </div>
                <div>
                  <span className="benefit-check">✓</span>
                  <span>Daemon-backed supervision & session persistence</span>
                </div>
                <div>
                  <span className="benefit-check">✓</span>
                  <span>Agent-friendly IPC interface & desktop GUI</span>
                </div>
              </div>
            </div>

            <WorkflowDiagram />
          </div>
        </section>

        <section className="install-section section-container" id="install">
          <div className="section-heading">
            <div className="eyebrow">GET STARTED IN SECONDS</div>
            <h2>Install Devpane on your machine.</h2>
            <p>
              Pre-built binaries, single-command Cargo installs, and native desktop applications
              for macOS and Windows.
            </p>
          </div>

          <InstallWidget />
        </section>

        <section className="get-started section-container" id="get-started">
          <div className="get-started-panel">
            <div className="eyebrow">
              LOCAL DEVELOPMENT, SIMPLIFIED
            </div>
            <h2>One command for your entire dev stack.</h2>
            <p>
              Define your services in devpane.yaml and run devpane up.
              Experience clean terminal multiplexing with interactive shell tabs.
            </p>
            <a
              className="button button-primary"
              href="#install"
            >
              Get Started Now <ArrowIcon />
            </a>
          </div>
        </section>
      </main>

      <footer className="site-footer">
        <a className="brand footer-brand" href="#home">
          <span className="brand-mark">d</span>
          <span>devpane</span>
        </a>
        <p>A clearer view of the processes behind your code.</p>
        <span className="footer-copyright">
          © {new Date().getFullYear()} Devpane
        </span>
      </footer>
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
  const [selectedOS, setSelectedOS] = useState<"macos" | "windows" | "linux">(getInitialOS);
  const [method, setMethod] = useState<"cli" | "brew" | "desktop" | "cargo">("cli");
  const [copied, setCopied] = useState(false);

  const cargoCommand = "cargo install devpane";
  const brewCommand = "brew install Ranjeesh0701/devpane/devpane";
  const repoReleases = "https://github.com/Ranjeesh0701/devpane/releases/latest";

  function copyText(text: string) {
    trackCopyInstall(selectedOS, text);
    void navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div className="install-box">
      {/* OS Selector Tabs */}
      <div className="install-os-tabs">
        <button
          className={`install-tab ${selectedOS === "macos" ? "active" : ""}`}
          onClick={() => setSelectedOS("macos")}
        >
          <span className="os-icon"></span> macOS
        </button>
        <button
          className={`install-tab ${selectedOS === "windows" ? "active" : ""}`}
          onClick={() => setSelectedOS("windows")}
        >
          <span className="os-icon">⊞</span> Windows
        </button>
        <button
          className={`install-tab ${selectedOS === "linux" ? "active" : ""}`}
          onClick={() => setSelectedOS("linux")}
        >
          <span className="os-icon">🐧</span> Linux
        </button>
      </div>

      {/* Method Subtabs */}
      <div className="install-method-tabs">
        <button
          className={`method-tab ${method === "cli" ? "active" : ""}`}
          onClick={() => setMethod("cli")}
        >
          Quick Install
        </button>
        {selectedOS !== "windows" && (
          <button
            className={`method-tab ${method === "brew" ? "active" : ""}`}
            onClick={() => setMethod("brew")}
          >
            Homebrew
          </button>
        )}
        <button
          className={`method-tab ${method === "cargo" ? "active" : ""}`}
          onClick={() => setMethod("cargo")}
        >
          Cargo
        </button>
        <button
          className={`method-tab ${method === "desktop" ? "active" : ""}`}
          onClick={() => setMethod("desktop")}
        >
          Desktop App GUI
        </button>
      </div>

      {/* Content based on method and OS */}
      <div className="install-content">
        {method === "cli" && (
          <div className="install-card">
            <div className="install-details">
              <h4>
                {selectedOS === "macos" && "macOS Standalone CLI (devpane + daemon)"}
                {selectedOS === "windows" && "Windows Standalone CLI (devpane.exe + daemon.exe)"}
                {selectedOS === "linux" && "Linux x86_64 Standalone CLI"}
              </h4>
              <p>
                Run the install command in your terminal to automatically download, extract,
                and configure both <code>devpane</code> and <code>daemon</code> into your <code>$PATH</code>:
              </p>
            </div>

            {/* Terminal Install Command */}
            <div className="install-terminal-box">
              <code>
                {selectedOS === "windows"
                  ? "irm https://raw.githubusercontent.com/Ranjeesh0701/devpane/main/scripts/install.ps1 | iex"
                  : "curl -fsSL https://raw.githubusercontent.com/Ranjeesh0701/devpane/main/scripts/install.sh | sh"}
              </code>
              <button
                className="copy-button"
                onClick={() => {
                  const cmd =
                    selectedOS === "windows"
                      ? "irm https://raw.githubusercontent.com/Ranjeesh0701/devpane/main/scripts/install.ps1 | iex"
                      : "curl -fsSL https://raw.githubusercontent.com/Ranjeesh0701/devpane/main/scripts/install.sh | sh";
                  copyText(cmd);
                }}
              >
                {copied ? "Copied!" : "Copy"}
              </button>
            </div>

            <div style={{ marginTop: "10px", fontSize: "11px", color: "var(--subtle)" }}>
              Or download the pre-built binaries directly:
            </div>

            <div className="install-action-row">
              {selectedOS === "macos" && (
                <>
                  <a
                    className="button button-primary"
                    href={`${repoReleases}/download/devpane-darwin-arm64.tar.gz`}
                    target="_blank"
                    rel="noreferrer"
                    onClick={() => trackDownload("macos", "devpane-darwin-arm64.tar.gz")}
                  >
                    Download Apple Silicon (.tar.gz) ↓
                  </a>
                  <a
                    className="button button-secondary"
                    href={`${repoReleases}/download/devpane-darwin-x64.tar.gz`}
                    target="_blank"
                    rel="noreferrer"
                    onClick={() => trackDownload("macos", "devpane-darwin-x64.tar.gz")}
                  >
                    Download Intel Mac (.tar.gz) ↓
                  </a>
                </>
              )}
              {selectedOS === "windows" && (
                <a
                  className="button button-primary"
                  href={`${repoReleases}/download/devpane-windows-x64.zip`}
                  target="_blank"
                  rel="noreferrer"
                  onClick={() => trackDownload("windows", "devpane-windows-x64.zip")}
                >
                  Download Windows x64 (.zip) ↓
                </a>
              )}
              {selectedOS === "linux" && (
                <a
                  className="button button-primary"
                  href="https://github.com/Ranjeesh0701/devpane/releases"
                  target="_blank"
                  rel="noreferrer"
                  onClick={() => trackGithubClick("linux_releases_button")}
                >
                  Browse Linux Releases on GitHub ↓
                </a>
              )}
            </div>
          </div>
        )}

        {method === "brew" && (
          <div className="install-card">
            <div className="install-details">
              <h4>Install via Homebrew (macOS & Linux)</h4>
              <p>Install Devpane and its background daemon via the official tap:</p>
            </div>
            <div className="install-terminal-box">
              <code>{brewCommand}</code>
              <button className="copy-button" onClick={() => copyText(brewCommand)}>
                {copied ? "Copied!" : "Copy"}
              </button>
            </div>
          </div>
        )}

        {method === "desktop" && (
          <div className="install-card">
            <div className="install-details">
              <h4>
                {selectedOS === "macos" && "Devpane for macOS (.dmg installer)"}
                {selectedOS === "windows" && "Devpane for Windows (.msi installer)"}
                {selectedOS === "linux" && "Devpane for Linux (.AppImage / .deb)"}
              </h4>
              <p>
                Zero-click standalone desktop application with an embedded background daemon.
                No terminal or manual daemon setup required - launch the app and immediately monitor
                real-time CPU %, RAM, and stream live logs visually.
              </p>
            </div>
            <div className="install-action-row">
              {selectedOS === "macos" && (
                <a
                  className="button button-primary"
                  href="https://github.com/Ranjeesh0701/devpane/releases"
                  target="_blank"
                  rel="noreferrer"
                  onClick={() => trackDownload("macos_desktop", "dmg")}
                >
                  Download macOS DMG Installer ↓
                </a>
              )}
              {selectedOS === "windows" && (
                <a
                  className="button button-primary"
                  href="https://github.com/Ranjeesh0701/devpane/releases"
                  target="_blank"
                  rel="noreferrer"
                  onClick={() => trackDownload("windows_desktop", "msi")}
                >
                  Download Windows MSI Installer ↓
                </a>
              )}
              {selectedOS === "linux" && (
                <a
                  className="button button-primary"
                  href="https://github.com/Ranjeesh0701/devpane/releases"
                  target="_blank"
                  rel="noreferrer"
                  onClick={() => trackDownload("linux_desktop", "packages")}
                >
                  Download Linux Packages ↓
                </a>
              )}
            </div>
          </div>
        )}

        {method === "cargo" && (
          <div className="install-card">
            <div className="install-details">
              <h4>Install via Cargo</h4>
              <p>Installs <code>devpane</code> directly into your <code>~/.cargo/bin</code>:</p>
            </div>
            <div className="install-terminal-box">
              <code>{cargoCommand}</code>
              <button className="copy-button" onClick={() => copyText(cargoCommand)}>
                {copied ? "Copied!" : "Copy"}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
