import { useMemo, useState } from "react";
import { Icon } from "../../../components/ui/Icon";
import { formatStarted } from "../utils/formatStarted";
import { extractLocalhostUrls, openInBrowser } from "../../../lib/browser";
import type { ProcessSession } from "../types";
import { ProcessStatusBadge } from "../../../components/ui/ProcessStatusBadge";

type Props = {
  sessions: ProcessSession[];
  onStop: (id: string) => Promise<void>;
  onRestart?: (id: string) => Promise<void>;
  onRebuild?: (name: string) => Promise<void>;
  onStopProject?: (projectDir: string) => Promise<void>;
  onRestartProject?: (projectDir: string) => Promise<void>;
  onSyncProject?: (projectDir: string) => Promise<unknown>;
  onViewLogs: (session: ProcessSession) => void;
};

function formatMemory(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || bytes <= 0) return "-";
  const mb = bytes / (1024 * 1024);
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)} GB`;
  }
  return `${mb.toFixed(1)} MB`;
}

function formatCpu(cpu: number | null | undefined): string {
  if (cpu === null || cpu === undefined) return "-";
  return `${cpu.toFixed(1)}%`;
}

function getServiceType(name: string): {
  icon: "globe" | "settings" | "hexagon" | "terminal";
  type: string;
  color: string;
  bg: string;
} {
  const lower = name.toLowerCase();
  if (lower.includes("web") || lower.includes("front") || lower.includes("ui")) {
    return {
      icon: "globe",
      type: "Frontend",
      color: "#8b9cff",
      bg: "rgba(139, 156, 255, 0.12)",
    };
  }
  if (
    lower.includes("worker") ||
    lower.includes("job") ||
    lower.includes("queue") ||
    lower.includes("cron")
  ) {
    return {
      icon: "settings",
      type: "Worker",
      color: "#f0bf69",
      bg: "rgba(240, 191, 105, 0.12)",
    };
  }
  if (
    lower.includes("api") ||
    lower.includes("server") ||
    lower.includes("backend")
  ) {
    return {
      icon: "hexagon",
      type: "API",
      color: "#a78bfa",
      bg: "rgba(167, 139, 250, 0.12)",
    };
  }
  return {
    icon: "terminal",
    type: "Service",
    color: "#55d6a2",
    bg: "rgba(85, 214, 162, 0.12)",
  };
}

function detectServicePort(command: string): number | null {
  const urls = extractLocalhostUrls(command);
  for (const url of urls) {
    const match = url.match(/:(\d+)/);
    if (match) {
      const p = parseInt(match[1], 10);
      if (p > 0 && p < 65536) return p;
    }
  }
  const portMatch = command.match(
    /(?:--port[=\s]+|-p\s+|PORT=(\d+)|\b(?:localhost|127\.0\.0\.1):)(\d+)/i,
  );
  if (portMatch) {
    const val = portMatch[1] || portMatch[2];
    if (val) {
      const p = parseInt(val, 10);
      if (p > 0 && p < 65536) return p;
    }
  }
  return null;
}

export function SessionTable({
  sessions,
  onStop,
  onRestart,
  onRebuild,
  onStopProject,
  onRestartProject,
  onSyncProject,
  onViewLogs,
}: Props) {
  const [stoppingId, setStoppingId] = useState<string | null>(null);
  const [restartingId, setRestartingId] = useState<string | null>(null);
  const [rebuildingName, setRebuildingName] = useState<string | null>(null);
  const [stoppingProject, setStoppingProject] = useState<string | null>(null);
  const [restartingProject, setRestartingProject] = useState<string | null>(null);
  const [syncingProject, setSyncingProject] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const groups = useMemo(() => {
    const map = new Map<
      string,
      { name: string; projectDir: string | null; sessions: ProcessSession[] }
    >();

    for (const session of sessions) {
      const key = session.project_dir || "__standalone__";
      if (!map.has(key)) {
        let name = "Standalone Services";
        if (session.project_dir) {
          const normalized = session.project_dir.replace(/\\/g, "/");
          const parts = normalized.split("/").filter(Boolean);
          name = parts[parts.length - 1] || session.project_dir;
        }
        map.set(key, {
          name,
          projectDir: session.project_dir || null,
          sessions: [],
        });
      }
      map.get(key)!.sessions.push(session);
    }

    return Array.from(map.values());
  }, [sessions]);

  async function handleStop(id: string) {
    setStoppingId(id);
    setActionError(null);
    try {
      await onStop(id);
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setStoppingId(null);
    }
  }

  async function handleRestart(id: string) {
    setRestartingId(id);
    setActionError(null);
    try {
      if (onRestart) {
        await onRestart(id);
      }
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRestartingId(null);
    }
  }

  async function handleRebuild(name: string) {
    setRebuildingName(name);
    setActionError(null);
    try {
      if (onRebuild) {
        await onRebuild(name);
      }
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRebuildingName(null);
    }
  }

  async function handleStopProject(projectDir: string) {
    setStoppingProject(projectDir);
    setActionError(null);
    try {
      if (onStopProject) {
        await onStopProject(projectDir);
      }
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setStoppingProject(null);
    }
  }

  async function handleRestartProject(projectDir: string) {
    setRestartingProject(projectDir);
    setActionError(null);
    try {
      if (onRestartProject) {
        await onRestartProject(projectDir);
      }
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRestartingProject(null);
    }
  }

  async function handleSyncProject(projectDir: string) {
    setSyncingProject(projectDir);
    setActionError(null);
    try {
      if (onSyncProject) {
        await onSyncProject(projectDir);
      }
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSyncingProject(null);
    }
  }

  if (sessions.length === 0) return null;

  return (
    <div className="session-table-wrap">
      {groups.map((group) => {
        const runningInGroup = group.sessions.filter(
          (s) => s.status === "running"
        ).length;
        const hasProject = Boolean(group.projectDir);

        return (
          <div key={group.name + (group.projectDir || "")} className="project-group">
            {hasProject && (
              <div className="project-group-header">
                <div className="project-group-info">
                  <div className="project-group-icon">
                    <Icon name="grid" size={16} />
                  </div>
                  <div>
                    <div className="project-group-title">
                      <strong>{group.name}</strong>
                      <span className="project-group-path" title={group.projectDir!}>
                        {group.projectDir}
                      </span>
                    </div>
                    <span className="project-group-meta">
                      {runningInGroup} running ·{" "}
                      {group.sessions.length - runningInGroup} exited
                    </span>
                  </div>
                </div>
                <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                  {onSyncProject && group.projectDir && (
                    <button
                      className="project-sync-btn"
                      onClick={() => void handleSyncProject(group.projectDir!)}
                      disabled={syncingProject === group.projectDir}
                      title={`Synchronize team environment and dependencies for ${group.name}`}
                    >
                      <Icon
                        name="refresh"
                        size={12}
                        className={syncingProject === group.projectDir ? "refresh-spinning" : ""}
                      />
                      <span>
                        {syncingProject === group.projectDir
                          ? "Syncing..."
                          : "Sync Env"}
                      </span>
                    </button>
                  )}
                  {onRestartProject && group.projectDir && (
                    <button
                      className="project-restart-btn"
                      onClick={() => void handleRestartProject(group.projectDir!)}
                      disabled={restartingProject === group.projectDir}
                      title={`Restart all services in ${group.name}`}
                    >
                      <Icon
                        name="refresh"
                        size={12}
                        className={restartingProject === group.projectDir ? "refresh-spinning" : ""}
                      />
                      <span>
                        {restartingProject === group.projectDir
                          ? "Restarting..."
                          : "Restart Project"}
                      </span>
                    </button>
                  )}
                  {runningInGroup > 0 && onStopProject && group.projectDir && (
                    <button
                      className="project-stop-btn"
                      onClick={() => void handleStopProject(group.projectDir!)}
                      disabled={stoppingProject === group.projectDir}
                      title={`Stop all services in ${group.name}`}
                    >
                      <Icon name="square" size={12} />
                      <span>
                        {stoppingProject === group.projectDir
                          ? "Stopping..."
                          : "Stop Project"}
                      </span>
                    </button>
                  )}
                </div>
              </div>
            )}
            <table className="session-table">
              <thead>
                <tr>
                  <th>PROCESS</th>
                  <th>COMMAND</th>
                  <th>PID</th>
                  <th>CPU</th>
                  <th>MEMORY</th>
                  <th>STARTED</th>
                  <th>STATUS</th>
                  <th>
                    <span className="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {group.sessions.map((session) => {
                  const svcType = getServiceType(session.name || "");
                  const port = detectServicePort(session.command);
                  const detectedUrls = extractLocalhostUrls(session.command);
                  const primaryUrl =
                    detectedUrls[0] || (port ? `http://localhost:${port}` : null);

                  return (
                    <tr key={session.id}>
                      <td>
                        <div className="process-cell">
                          <div
                            className="process-icon"
                            style={{
                              color: svcType.color,
                              borderColor: `${svcType.color}33`,
                              background: svcType.bg,
                            }}
                            title={`${svcType.type} service`}
                          >
                            <Icon name={svcType.icon} size={16} />
                          </div>
                          <div className="process-copy">
                            <div className="process-name-wrap">
                              <strong>{session.name || "Unnamed process"}</strong>
                              {port && (
                                <button
                                  className="service-port-badge"
                                  onClick={() =>
                                    void openInBrowser(
                                      primaryUrl || `http://localhost:${port}`,
                                    )
                                  }
                                  title={`Open http://localhost:${port} in browser`}
                                  type="button"
                                >
                                  :{port}
                                </button>
                              )}
                            </div>
                            <span title={session.id}>
                              PID {session.pid} · {session.id.slice(0, 8)}…
                            </span>
                          </div>
                        </div>
                      </td>
                      <td>
                        <code className="command-cell" title={session.command}>
                          {session.command}
                        </code>
                      </td>
                      <td>
                        <span className="pid-cell">#{session.pid}</span>
                      </td>
                      <td>
                        <span className="resource-cell resource-cpu">
                          {session.status === "running"
                            ? formatCpu(session.cpu_percent)
                            : "-"}
                        </span>
                      </td>
                      <td>
                        <span className="resource-cell resource-mem">
                          {session.status === "running"
                            ? formatMemory(session.memory_bytes)
                            : "-"}
                        </span>
                      </td>
                      <td>
                        <span className="started-cell">
                          <Icon name="clock" size={14} />
                          {formatStarted(session)}
                        </span>
                      </td>
                      <td>
                        <ProcessStatusBadge session={session} />
                      </td>
                      <td>
                        <div className="row-actions">
                          {primaryUrl && session.status === "running" && (
                            <button
                              className="row-action row-action-url"
                              aria-label={`Open ${primaryUrl} in browser`}
                              onClick={() => void openInBrowser(primaryUrl)}
                              title={`Open ${primaryUrl} in browser`}
                            >
                              <Icon name="external" size={15} />
                            </button>
                          )}
                          <button
                            className="row-action"
                            aria-label={`View logs for ${session.name}`}
                            onClick={() => onViewLogs(session)}
                            title="View logs"
                          >
                            <Icon name="terminal" size={15} />
                          </button>
                          <button
                            className="row-action"
                            aria-label={`Copy session ID for ${session.name}`}
                            onClick={() =>
                              void navigator.clipboard?.writeText(session.id)
                            }
                            title="Copy session ID"
                          >
                            <Icon name="copy" size={15} />
                          </button>
                          {onRestart && (
                            <button
                              className="row-action"
                              onClick={() => void handleRestart(session.id)}
                              title="Restart service"
                              aria-label={`Restart ${session.name}`}
                              disabled={restartingId === session.id}
                            >
                              <Icon
                                name="refresh"
                                size={14}
                                className={
                                  restartingId === session.id
                                    ? "refresh-spinning"
                                    : ""
                                }
                              />
                            </button>
                          )}
                          {onRebuild && session.name && (
                            <button
                              className="row-action"
                              onClick={() => void handleRebuild(session.name!)}
                              title="Rebuild and restart service"
                              aria-label={`Rebuild ${session.name}`}
                              disabled={rebuildingName === session.name}
                            >
                              <Icon
                                name="wrench"
                                size={14}
                                className={
                                  rebuildingName === session.name
                                    ? "refresh-spinning"
                                    : ""
                                }
                              />
                            </button>
                          )}
                          {session.status === "running" && (
                            <button
                              className="row-action row-action-danger"
                              onClick={() => void handleStop(session.id)}
                              title="Stop session"
                              aria-label={`Stop ${session.name}`}
                              disabled={stoppingId === session.id}
                            >
                              <Icon name="square" size={14} />
                            </button>
                          )}
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        );
      })}
      {actionError && (
        <p className="error-message" role="alert">
          {actionError}
        </p>
      )}
      <div className="table-footer">
        <span>
          Showing <strong>{sessions.length}</strong> session
          {sessions.length === 1 ? "" : "s"} (
          <strong style={{ color: "var(--green)" }}>
            {sessions.filter((s) => s.status === "running").length} running
          </strong>{" "}
          · {sessions.filter((s) => s.status !== "running").length} exited)
        </span>
        <span className="live-indicator">
          <span /> Supervised
        </span>
      </div>
    </div>
  );
}
