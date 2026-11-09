import type { ProcessSession } from "../../features/sessions/types";

type Props = {
  session: ProcessSession;
};

export function ProcessStatusBadge({ session }: Props) {
  const isRunning = session.status === "running";
  const hasFailed = !isRunning && session.exit_code != null && session.exit_code !== 0;

  return (
    <span
      className={`process-status ${
        isRunning
          ? "process-status--running"
          : hasFailed
            ? "process-status--failed"
            : "process-status--exited"
      }`}
    >
      <span className="process-status__dot" />

      {isRunning
        ? "Running"
        : session.exit_code == null
          ? "Exited"
          : `Exited · ${session.exit_code}`}
    </span>
  );
}
