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
  stopProject: (projectDir: string) =>
    invoke<void>("stop_project", { projectDir }),
  stopAllSessions: () => invoke<void>("stop_all_sessions"),
  getSessionLogs(id: string): Promise<string> {
    return invoke<string>("get_session_logs", { id });
  },
};
