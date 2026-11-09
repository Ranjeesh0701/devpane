import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";
import { Topbar } from "./Topbar";
import type { NavItem } from "./navigation";
import type { Theme } from "../../lib/theme";

type Props = {
  activePage: NavItem;
  onNavigate: (page: NavItem) => void;
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  children: ReactNode;
};

export function AppLayout({
  activePage,
  onNavigate,
  theme,
  onThemeChange,
  children,
}: Props) {
  return (
    <div className="app-shell">
      <Sidebar
        activePage={activePage}
        onNavigate={onNavigate}
        theme={theme}
        onThemeChange={onThemeChange}
      />
      <main className="main-area">
        <Topbar page={activePage} />
        <div className="content-scroll-area">{children}</div>
      </main>
    </div>
  );
}
