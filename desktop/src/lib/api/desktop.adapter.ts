import { invoke } from "@tauri-apps/api/core";
import type {
  ProcessSession,
  NewSessionInput,
} from "../../features/sessions/types";

export const desktopAdapter = {
  listSessions(): Promise<ProcessSession[]> {
    return invoke<ProcessSession[]>("list_sessions");
  },

  startSession(input: NewSessionInput): Promise<ProcessSession> {
    return invoke<ProcessSession>("start_session", input);
  },
  stopSession: (id: string) => invoke<void>("stop_session", { id }),
  restartSession: (id: string) => invoke<ProcessSession>("restart_session", { id }),
  rebuildSession: (name: string) => invoke<void>("rebuild_session", { name }),
  restartProject: (projectDir: string) =>
    invoke<void>("restart_project", { projectDir }),
  syncProject: (projectDir: string, installDeps = true) =>
    invoke<{
      env_created: boolean;
      missing_env_keys: string[];
      added_env_keys: string[];
      dependencies_installed: string[];
      lockfile_updated: boolean;
      warnings: string[];
    }>("sync_project", { projectDir, installDeps }),
  stopProject: (projectDir: string) =>
    invoke<void>("stop_project", { projectDir }),
  stopAllSessions: () => invoke<void>("stop_all_sessions"),
  getSessionLogs(id: string): Promise<string> {
    return invoke<string>("get_session_logs", { id });
  },
  getCliStatus(): Promise<{ installed: boolean; path: string | null }> {
    return invoke<{ installed: boolean; path: string | null }>("get_cli_status");
  },
};
