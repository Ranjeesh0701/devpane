export type ProcessStatus = "running" | "exited";

export type ProcessSession = {
  id: string;
  pid: number;
  name: string;
  command: string;
  start_time_seconds: number;
  start_time_useconds: number;
  status: ProcessStatus;
  exit_code: number | null;
  project_dir?: string | null;
  cpu_percent?: number | null;
  memory_bytes?: number | null;
};

export type NewSessionInput = {
  name: string;
  program: string;
  args: string[];
};
