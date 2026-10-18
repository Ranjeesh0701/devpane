import { Icon } from "../../../components/ui/Icon";
import { StatusDot } from "../../../components/ui/StatusDot";

type Props = {
  count: number;
  loading: boolean;
  connected: boolean;
  lastUpdated: Date | null;
};

export function SessionStats({
  count,
  loading,
  connected,
  lastUpdated,
}: Props) {
  const updated = lastUpdated
    ? lastUpdated.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    : "-";
  return (
    <div className="stats-grid">
      <article className="stat-card">
        <div className="stat-top">
          <span className="stat-label">Total sessions</span>
          <span className="stat-icon">
            <Icon name="grid" size={17} />
          </span>
        </div>
        <div className="stat-value">{loading ? "-" : count}</div>
        <div className="stat-foot">
          <span className="stat-foot-muted">Persisted sessions</span>
        </div>
      </article>
      <article className="stat-card">
        <div className="stat-top">
          <span className="stat-label">Daemon connection</span>
          <span
            className={`stat-icon ${connected ? "icon-green" : "icon-danger"}`}
          >
            <Icon name={connected ? "activity" : "alert"} size={17} />
          </span>
        </div>
        <div className="stat-value stat-value-status">
          {connected ? (loading ? "Checking" : "Online") : "Offline"}
          <StatusDot
            status={connected ? (loading ? "pending" : "online") : "offline"}
          />
        </div>
        <div className="stat-foot">
          <span className="stat-foot-muted">Unix socket connection</span>
        </div>
      </article>
      <article className="stat-card">
        <div className="stat-top">
          <span className="stat-label">Last refreshed</span>
          <span className="stat-icon">
            <Icon name="clock" size={17} />
          </span>
        </div>
        <div className="stat-value stat-value-time">{updated}</div>
        <div className="stat-foot">
          <span className="stat-foot-muted">Local machine time</span>
        </div>
      </article>
    </div>
  );
}
