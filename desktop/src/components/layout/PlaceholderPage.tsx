import { Icon } from "../ui/Icon";
import type { NavItem } from "./navigation";

const content: Record<
  Exclude<NavItem, "sessions">,
  { description: string; icon: "activity" | "settings" }
> = {
  activity: {
    description:
      "A timeline of process starts, stops, and daemon events will live here.",
    icon: "activity",
  },
  settings: {
    description:
      "Appearance and application preferences will be configurable here.",
    icon: "settings",
  },
};

export function PlaceholderPage({ page }: { page: NavItem }) {
  if (page === "sessions") return null;
  const item = content[page];
  const title = page === "activity" ? "Activity" : "Settings";
  return (
    <section className="page-content secondary-page">
      <div className="eyebrow">
        <span className="eyebrow-line" /> DEVPANE WORKSPACE
      </div>
      <h1>
        {title}
        <span className="heading-period">.</span>
      </h1>
      <div className="coming-soon">
        <div className="state-icon">
          <Icon name={item.icon} size={22} />
        </div>
        <h3>{title} is coming next</h3>
        <p>{item.description}</p>
      </div>
    </section>
  );
}
