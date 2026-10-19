import { useEffect, useState } from "react";
import type { Theme } from "../../../lib/theme";
import { api } from "../../../lib/api";

type Props = {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
};

export function SettingsPage({ theme, onThemeChange }: Props) {
  const [cliStatus, setCliStatus] = useState<{ installed: boolean; path: string | null } | null>(null);

  useEffect(() => {
    void api.getCliStatus().then(setCliStatus).catch(() => null);
  }, []);

  return (
    <section className="page-content settings-page">
      <div className="eyebrow">
        <span className="eyebrow-line" /> DEVPANE WORKSPACE
      </div>

      <div className="settings-heading">
        <div>
          <h1>
            Settings<span className="heading-period">.</span>
          </h1>
          <p className="page-description">Customize your Devpane workspace.</p>
        </div>
      </div>

      <section className="settings-section">
        <div className="settings-section-heading">
          <div className="settings-section-icon">◐</div>
          <div>
            <h2>Appearance</h2>
            <p>Choose how Devpane looks on your device.</p>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <strong>Color theme</strong>
            <span>Your preference applies throughout the app.</span>
          </div>

          <div className="theme-options" role="group" aria-label="Color theme">
            <button
              className={`theme-option ${theme === "light" ? "is-selected" : ""}`}
              aria-pressed={theme === "light"}
              onClick={() => onThemeChange("light")}
            >
              Light
            </button>
            <button
              className={`theme-option ${theme === "dark" ? "is-selected" : ""}`}
              aria-pressed={theme === "dark"}
              onClick={() => onThemeChange("dark")}
            >
              Dark
            </button>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-heading">
          <div className="settings-section-icon">⌘</div>
          <div>
            <h2>Process defaults</h2>
            <p>Options for launching new processes.</p>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <strong>Default working directory</strong>
            <span>Working-directory preferences aren't configurable yet.</span>
          </div>
          <span className="settings-tag">Coming later</span>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <strong>Default shell</strong>
            <span>
              Processes currently use the command provided when launched.
            </span>
          </div>
          <span className="settings-tag">Coming later</span>
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-heading">
          <div className="settings-section-icon">&gt;_</div>
          <div>
            <h2>CLI &amp; Daemon</h2>
            <p>Background process supervisor and terminal CLI integration.</p>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <strong>Embedded Daemon</strong>
            <span>Zero-setup process supervisor is automatically active in the background.</span>
          </div>
          <span className="settings-tag" style={{ color: "#38ef7d", borderColor: "rgba(56, 239, 125, 0.3)" }}>
            Active
          </span>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <strong>Terminal CLI (<code>devpane</code>)</strong>
            <span>
              {cliStatus?.installed
                ? `Detected at ${cliStatus.path}`
                : "Install via Homebrew: brew install Ranjeesh0701/devpane/devpane, or Cargo: cargo install devpane"}
            </span>
          </div>
          <span className="settings-tag">
            {cliStatus?.installed ? "Installed" : "Optional"}
          </span>
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-heading">
          <div className="settings-section-icon">i</div>
          <div>
            <h2>About</h2>
            <p>Information about your application.</p>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <strong>Devpane Desktop</strong>
            <span>Local process and workspace management for developers.</span>
          </div>
          <span className="settings-tag">v0.3.1</span>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <strong>Platform</strong>
            <span>macOS &amp; Windows</span>
          </div>
        </div>
      </section>
    </section>
  );
}
