import { useEffect, useId, useState, type FormEvent } from "react";
import { Button } from "../../../components/ui/Button";
import { Icon } from "../../../components/ui/Icon";
import type { NewSessionInput } from "../types";

type Props = {
  onClose: () => void;
  onSubmit: (input: NewSessionInput) => Promise<void>;
};

export function NewSessionDialog({ onClose, onSubmit }: Props) {
  const nameId = useId();
  const programId = useId();
  const argsId = useId();
  const [name, setName] = useState("");
  const [program, setProgram] = useState("");
  const [argsText, setArgsText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !submitting) onClose();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, submitting]);

  useEffect(() => {
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      document.body.style.overflow = previousOverflow;
    };
  }, []);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    const trimmedName = name.trim();
    const trimmedProgram = program.trim();
    if (!trimmedName || !trimmedProgram) {
      setError("Add a session name and executable to continue.");
      return;
    }

    // One argument per line keeps spaces inside an individual argument intact.
    const args = argsText
      .split("\n")
      .map((argument) => argument.trim())
      .filter(Boolean);

    setSubmitting(true);
    try {
      await onSubmit({ name: trimmedName, program: trimmedProgram, args });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div
      className="dialog-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !submitting) onClose();
      }}
    >
      <section
        className="session-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-session-title"
      >
        <div className="dialog-heading">
          <div className="dialog-title-icon">
            <Icon name="terminal" size={19} />
          </div>
          <div className="dialog-heading-copy">
            <h2 id="new-session-title">Start a session</h2>
            <p>Launch a process and let Devpane manage it.</p>
          </div>
          <button
            className="icon-button dialog-close"
            type="button"
            onClick={onClose}
            disabled={submitting}
            aria-label="Close dialog"
          >
            ×
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="dialog-fields">
            <div className="form-field">
              <label htmlFor={nameId}>Session name</label>
              <input
                id={nameId}
                autoFocus
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="api-server"
                maxLength={80}
                required
                disabled={submitting}
              />
              <span className="field-help">
                A label to help you identify this process.
              </span>
            </div>
            <div className="form-field">
              <label htmlFor={programId}>Executable</label>
              <input
                id={programId}
                value={program}
                onChange={(event) => setProgram(event.target.value)}
                placeholder="npm"
                required
                disabled={submitting}
              />
              <span className="field-help">
                The executable name or path, for example{" "}
                <code>/usr/bin/python3</code>.
              </span>
            </div>
            <div className="form-field">
              <label htmlFor={argsId}>
                Arguments <span className="optional-label">OPTIONAL</span>
              </label>
              <textarea
                id={argsId}
                value={argsText}
                onChange={(event) => setArgsText(event.target.value)}
                placeholder={"run\ndev"}
                rows={3}
                disabled={submitting}
              />
              <span className="field-help">
                One argument per line. Example: <code>npm</code> with{" "}
                <code>run</code> and <code>dev</code>.
              </span>
            </div>
          </div>

          {error && (
            <div className="dialog-error" role="alert">
              <Icon name="alert" size={15} />
              <span>{error}</span>
            </div>
          )}

          <div className="dialog-footer">
            <span className="dialog-footer-note">
              <span className="dialog-footer-dot" /> Runs under the Devpane
              daemon
            </span>
            <div className="dialog-actions">
              <Button type="button" onClick={onClose} disabled={submitting}>
                Cancel
              </Button>
              <Button
                type="submit"
                variant="primary"
                disabled={submitting}
                icon={
                  submitting ? (
                    <span className="button-spinner" />
                  ) : (
                    <Icon name="plus" size={15} />
                  )
                }
              >
                {submitting ? "Starting…" : "Start session"}
              </Button>
            </div>
          </div>
        </form>
      </section>
    </div>
  );
}
