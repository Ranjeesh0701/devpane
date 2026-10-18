import "./App.css";

const services = [
  { name: "backend", status: "running", command: "cargo run", active: true },
  { name: "frontend", status: "running", command: "bun run dev", active: false },
  { name: "worker", status: "running", command: "bun run worker", active: false },
  { name: "Shell 1", status: "interactive", command: "zsh", active: false },
] as const;

const features = [
  {
    number: "01",
    icon: "⌘",
    title: "AI Process Supervision",
    description:
      "Coding agents like Claude and Gemini can run devpane up and spawn services via daemon without littering your terminal.",
    tag: "AGENT-FRIENDLY",
  },
  {
    number: "02",
    icon: "⌁",
    title: "Interactive Workspace & Panes",
    description:
      "A tmux-inspired TUI with VT100 terminal emulation. Switch tabs with Tab, spawn new shells with Ctrl+T, and scroll logs smoothly.",
    tag: "TERMINAL WORKSPACE",
  },
  {
    number: "03",
    icon: "▤",
    title: "Declarative devpane.yaml",
    description:
      "Define all services, working directories, and commands once in devpane.yaml. devpane up starts everything; devpane down cleans up.",
    tag: "ONE WORKSPACE",
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
        <span className="window-title">devpane up — interactive workspace</span>
        <span className="window-version">v0.1.0</span>
      </div>

      <div className="terminal-body">
        <div className="terminal-context">
          <span className="agent-indicator">
            <span className="agent-indicator-dot" />
            LIVE DAEMON SUPERVISOR
          </span>
          <span className="terminal-muted">devpane.yaml loaded</span>
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
          <div style={{ color: "var(--subtle)" }}>[devpane:daemon] Service 'backend' started (PID 48921)</div>
          <div style={{ color: "#aab5ff" }}>   Compiling devpane v0.1.0 (/Workspace/devpane)</div>
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
            <span className="terminal-muted">Tab: Switch • Ctrl+T: Shell • Ctrl+Q: Quit</span>
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
          <a href="https://github.com/Ranjeesh0701/devbox" target="_blank" rel="noreferrer">
            GitHub <ArrowIcon />
          </a>
        </nav>

        <a className="header-cta" href="https://github.com/Ranjeesh0701/devbox" target="_blank" rel="noreferrer">
          Explore Devpane <ArrowIcon />
        </a>
      </header>

      <main>
        <section className="hero section-container" id="home">
          <div className="hero-copy">
            <div className="eyebrow">
              <span className="eyebrow-dot" />
              DEVELOPMENT IN THE AGE OF AI
            </div>

            <h1>
              Let AI build.
              <br />
              <span>Stay in control.</span>
            </h1>

            <p className="hero-description">
              AI can write your code and start your servers. Devpane is built
              to help you see what's running, inspect process output, and
              manage your development workflow from one interactive workspace.
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
              Built around your workflow, not another AI chatbot.
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
            <p>FROM AI-GENERATED CODE TO RUNNING SOFTWARE</p>
            <div className="principle-items">
              <span>
                <i>01</i> Build
              </span>
              <span>
                <i>02</i> Run
              </span>
              <span>
                <i>03</i> Observe
              </span>
            </div>
          </div>
        </section>

        <section className="features section-container" id="features">
          <div className="section-heading">
            <div className="eyebrow">THE MISSING VISIBILITY LAYER</div>
            <h2>Let your agent work. See what it runs.</h2>
            <p>
              AI-assisted development shouldn't mean losing track of the
              processes that power your application.
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
              <h2>From agent instructions to application insight.</h2>
              <p>
                Imagine a workflow where your coding agent can manage processes
                through a predictable interface, while you get a clear visual
                view of what's running and what needs your attention.
              </p>

              <div className="workflow-benefits">
                <div>
                  <span className="benefit-check">✓</span>
                  <span>Consistent process management</span>
                </div>
                <div>
                  <span className="benefit-check">✓</span>
                  <span>Visible process state and output</span>
                </div>
                <div>
                  <span className="benefit-check">✓</span>
                  <span>A human-readable workspace</span>
                </div>
              </div>
            </div>

            <WorkflowDiagram />
          </div>
        </section>

        <section className="get-started section-container" id="get-started">
          <div className="get-started-panel">
            <div className="eyebrow">
              THE FUTURE OF DEVELOPMENT IS DIFFERENT
            </div>
            <h2>Build with AI. Understand what runs.</h2>
            <p>
              Devpane is evolving to make local development processes easier to
              manage in an AI-assisted world.
            </p>
            <a
              className="button button-primary"
              href="https://github.com/Ranjeesh0701/devbox"
              target="_blank"
              rel="noreferrer"
            >
              Explore on GitHub <ArrowIcon />
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

export default App;
