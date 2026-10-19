import { Icon } from "../../../components/ui/Icon";
import { Button } from "../../../components/ui/Button";

type Props = {
  kind: "empty" | "no-results" | "error" | "loading";
  message?: string;
  onRetry?: () => void;
  searching?: boolean;
};

export function SessionEmptyState({ kind, message, onRetry }: Props) {
  if (kind === "loading") {
    return (
      <div className="state-panel">
        <div className="loading-orb" />
        <h3>Loading sessions</h3>
        <p>Connecting to your local Devpane daemon...</p>
      </div>
    );
  }
  if (kind === "error") {
    return (
      <div className="state-panel error-panel">
        <div className="state-icon error-state-icon">
          <Icon name="alert" size={21} />
        </div>
        <h3>Couldn't connect to Devpane</h3>
        <p>{message}</p>
        <p className="state-hint">
          Make sure the Devpane daemon is running, then try again.
        </p>
        {onRetry && (
          <Button icon={<Icon name="refresh" size={15} />} onClick={onRetry}>
            Try again
          </Button>
        )}
      </div>
    );
  }
  const noResults = kind === "no-results";
  return (
    <div className="state-panel">
      <div className="state-icon">
        <Icon name={noResults ? "search" : "terminal"} size={22} />
      </div>
      <h3>{noResults ? "No matching sessions" : "Your workspace is quiet"}</h3>
      <p>
        {noResults
          ? "Try another name, command, PID, or session ID."
          : "Start a process with the Devpane CLI. It will show up here when you refresh."}
      </p>
      {!noResults && (
        <div className="terminal-tip">
          <span className="terminal-prompt">$</span>
          <code>devpane start &lt;name&gt; &lt;program&gt; [args...]</code>
          <button
            className="copy-tip"
            onClick={() =>
              void navigator.clipboard?.writeText(
                "devpane start <name> <program> [args...]",
              )
            }
          >
            Copy
          </button>
        </div>
      )}
    </div>
  );
}
