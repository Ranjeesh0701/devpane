import { useMemo, useState } from "react";
import { Icon } from "../../../components/ui/Icon";
import { Tooltip } from "../../../components/ui/Tooltip";
import { formatStarted } from "../utils/formatStarted";
import { extractLocalhostUrls, openInBrowser } from "../../../lib/browser";
import type { ProcessSession } from "../types";
import { ProcessStatusBadge } from "../../../components/ui/ProcessStatusBadge";
import { SessionMoreMenu } from "./SessionMoreMenu";

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
                  <div className="project-group-text">
                    <div className="project-group-title">
                      <strong>{group.name}</strong>
                      <span className="project-group-meta">
                        {runningInGroup} running ·{" "}
                        {group.sessions.length - runningInGroup} exited
                      </span>
                    </div>
                    {group.projectDir && (
                      <span
                        className="project-group-path"
                        title={group.projectDir}
                      >
                        {group.projectDir}
                      </span>
                    )}
                  </div>
                </div>
                <div className="project-group-actions">
                  {onSyncProject && group.projectDir && (
                    <Tooltip content="Synchronize .env keys and lockfile dependencies">
                      <button
                        className="project-sync-btn"
                        onClick={() => void handleSyncProject(group.projectDir!)}
                        disabled={syncingProject === group.projectDir}
                        aria-label={`Synchronize environment for ${group.name}`}
                      >
                        <Icon
                          name="refresh"
                          size={12}
                          className={
                            syncingProject === group.projectDir
                              ? "refresh-spinning"
                              : ""
                          }
                        />
                        <span>
                          {syncingProject === group.projectDir
                            ? "Syncing..."
                            : "Sync"}
                        </span>
                      </button>
                    </Tooltip>
                  )}
                  {onRestartProject && group.projectDir && (
                    <Tooltip
                      content={`Restart all ${group.sessions.length} services in ${group.name}`}
                    >
                      <button
                        className="project-restart-btn"
                        onClick={() => void handleRestartProject(group.projectDir!)}
                        disabled={restartingProject === group.projectDir}
                        aria-label={`Restart all services in ${group.name}`}
                      >
                        <Icon
                          name="refresh"
                          size={12}
                          className={
                            restartingProject === group.projectDir
                              ? "refresh-spinning"
                              : ""
                          }
                        />
                        <span>
                          {restartingProject === group.projectDir
                            ? "Restarting..."
                            : "Restart"}
                        </span>
                      </button>
                    </Tooltip>
                  )}
                  {runningInGroup > 0 && onStopProject && group.projectDir && (
                    <Tooltip content={`Stop all services in ${group.name}`}>
                      <button
                        className="project-stop-btn"
                        onClick={() => void handleStopProject(group.projectDir!)}
                        disabled={stoppingProject === group.projectDir}
                        aria-label={`Stop all services in ${group.name}`}
                      >
                        <Icon name="square" size={12} />
                        <span>
                          {stoppingProject === group.projectDir
                            ? "Stopping..."
                            : "Stop"}
                        </span>
                      </button>
                    </Tooltip>
                  )}
                </div>
              </div>
            )}
            <div className="project-table-container">
              <table className="session-table">
                <thead>
                  <tr>
                    <th className="th-process">PROCESS</th>
                    <th className="th-command">COMMAND</th>
                    <th className="th-pid">PID</th>
                    <th className="th-cpu">CPU</th>
                    <th className="th-mem">MEMORY</th>
                    <th className="th-started">STARTED</th>
                    <th className="th-status">STATUS</th>
                    <th className="th-actions">
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
                        <td className="td-process">
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
                                  <Tooltip content={`Open http://localhost:${port} in browser`}>
                                    <button
                                      className="service-port-badge"
                                      onClick={() =>
                                        void openInBrowser(
                                          primaryUrl || `http://localhost:${port}`,
                                        )
                                      }
                                      aria-label={`Open http://localhost:${port} in browser`}
                                      type="button"
                                    >
                                      :{port}
                                    </button>
                                  </Tooltip>
                                )}
                              </div>
                              <span title={session.id}>
                                PID {session.pid} · {session.id.slice(0, 8)}…
                              </span>
                            </div>
                          </div>
                        </td>
                        <td className="td-command">
                          <code className="command-cell" title={session.command}>
                            {session.command}
                          </code>
                        </td>
                        <td className="td-pid">
                          <span className="pid-cell">#{session.pid}</span>
                        </td>
                        <td className="td-cpu">
                          <span className="resource-cell resource-cpu">
                            {session.status === "running"
                              ? formatCpu(session.cpu_percent)
                              : "-"}
                          </span>
                        </td>
                        <td className="td-mem">
                          <span className="resource-cell resource-mem">
                            {session.status === "running"
                              ? formatMemory(session.memory_bytes)
                              : "-"}
                          </span>
                        </td>
                        <td className="td-started">
                          <span className="started-cell">
                            <Icon name="clock" size={14} />
                            {formatStarted(session)}
                          </span>
                        </td>
                        <td className="td-status">
                          <ProcessStatusBadge session={session} />
                        </td>
                        <td className="td-actions">
                          <div className="row-actions">
                            {primaryUrl && session.status === "running" && (
                              <Tooltip content={`Open ${primaryUrl} in browser`}>
                                <button
                                  className="row-action row-action-url"
                                  aria-label={`Open ${primaryUrl} in browser`}
                                  onClick={() => void openInBrowser(primaryUrl)}
                                >
                                  <Icon name="external" size={15} />
                                </button>
                              </Tooltip>
                            )}
                            <Tooltip content="View live logs">
                              <button
                                className="row-action"
                                aria-label={`View logs for ${session.name}`}
                                onClick={() => onViewLogs(session)}
                              >
                                <Icon name="terminal" size={15} />
                              </button>
                            </Tooltip>
                            {onRestart && (
                              <Tooltip
                                content={
                                  session.status === "running"
                                    ? "Restart service"
                                    : "Start service"
                                }
                              >
                                <button
                                  className="row-action"
                                  onClick={() => void handleRestart(session.id)}
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
                              </Tooltip>
                            )}
                            <SessionMoreMenu
                              session={session}
                              primaryUrl={primaryUrl}
                              onViewLogs={onViewLogs}
                              onRestart={onRestart ? handleRestart : undefined}
                              onRebuild={onRebuild ? handleRebuild : undefined}
                              onStop={handleStop}
                              isRestarting={restartingId === session.id}
                              isRebuilding={rebuildingName === session.name}
                              isStopping={stoppingId === session.id}
                            />
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
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
