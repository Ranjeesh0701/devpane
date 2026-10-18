import { Icon } from "../ui/Icon";
import { StatusDot } from "../ui/StatusDot";
import { NAV_ITEMS, type NavItem } from "./navigation";
import type { Theme } from "../../lib/theme";

type Props = {
  activePage: NavItem;
  onNavigate: (page: NavItem) => void;
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
};

export function Sidebar({
  activePage,
  onNavigate,
  theme,
  onThemeChange,
}: Props) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">
          <Icon name="terminal" size={21} />
        </div>
        <div className="brand-copy">
          <span className="brand-name">devpane</span>
          <span className="brand-caption">PROCESS WORKSPACE</span>
        </div>
        <button
          className="icon-button workspace-menu"
          aria-label="Workspace menu"
        >
          <Icon name="more" />
        </button>
      </div>

      <div className="workspace-switcher">
        <div className="workspace-avatar">D</div>
        <div className="workspace-copy">
          <strong>Local machine</strong>
          <span>macOS · Desktop</span>
        </div>
        <StatusDot status="online" label="Local workspace" />
      </div>

      <div className="nav-label">WORKSPACE</div>
      <nav className="primary-nav" aria-label="Main navigation">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            className={`nav-item ${activePage === item.id ? "active" : ""}`}
            onClick={() => onNavigate(item.id)}
          >
            <Icon name={item.icon} />
            <span>{item.label}</span>
          </button>
        ))}
      </nav>

      <div className="sidebar-bottom">
        <div className="sidebar-footer">
          <div className="avatar-small">DV</div>
          <div className="footer-copy">
            <strong>Devbox Desktop</strong>
            <span>Developer preview</span>
          </div>
          <button
            className="icon-button theme-button"
            onClick={() => onThemeChange(theme === "dark" ? "light" : "dark")}
            aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
          >
            <Icon name={theme === "dark" ? "sun" : "moon"} size={17} />
          </button>
        </div>
      </div>
    </aside>
  );
}
