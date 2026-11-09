import { useState, useRef, useEffect } from "react";
import { createPortal } from "react-dom";
import { Icon } from "../../../components/ui/Icon";
import { Tooltip } from "../../../components/ui/Tooltip";
import { openInBrowser } from "../../../lib/browser";
import type { ProcessSession } from "../types";
import "./SessionMoreMenu.css";

type Props = {
  session: ProcessSession;
  primaryUrl: string | null;
  onViewLogs: (session: ProcessSession) => void;
  onRestart?: (id: string) => Promise<void>;
  onRebuild?: (name: string) => Promise<void>;
  onStop: (id: string) => Promise<void>;
  isRestarting: boolean;
  isRebuilding: boolean;
  isStopping: boolean;
};

export function SessionMoreMenu({
  session,
  primaryUrl,
  onViewLogs,
  onRestart,
  onRebuild,
  onStop,
  isRestarting,
  isRebuilding,
  isStopping,
}: Props) {
  const [isOpen, setIsOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const [menuPos, setMenuPos] = useState<React.CSSProperties>({});
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const updatePosition = () => {
    if (!triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    const openUpwards = spaceBelow < 220;

    const menuWidth = 210;
    const left = Math.max(
      12,
      Math.min(rect.right - menuWidth, window.innerWidth - menuWidth - 12),
    );

    setMenuPos({
      top: openUpwards ? undefined : `${rect.bottom + 4}px`,
      bottom: openUpwards
        ? `${window.innerHeight - rect.top + 4}px`
        : undefined,
      left: `${left}px`,
    });
  };

  const toggle = () => {
    if (!isOpen) {
      updatePosition();
      setIsOpen(true);
    } else {
      setIsOpen(false);
    }
  };

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (
        menuRef.current &&
        !menuRef.current.contains(event.target as Node) &&
        triggerRef.current &&
        !triggerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setIsOpen(false);
        triggerRef.current?.focus();
      }
    };

    const handleScrollOrResize = () => {
      setIsOpen(false);
    };

    window.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("scroll", handleScrollOrResize, true);
    window.addEventListener("resize", handleScrollOrResize);

    return () => {
      window.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("scroll", handleScrollOrResize, true);
      window.removeEventListener("resize", handleScrollOrResize);
    };
  }, [isOpen]);

  const handleCopyId = async () => {
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(session.id);
      setCopied(true);
      setTimeout(() => {
        setCopied(false);
        setIsOpen(false);
      }, 900);
    }
  };

  const isRunning = session.status === "running";

  return (
    <>
      <Tooltip content="More actions" position="top">
        <button
          ref={triggerRef}
          type="button"
          className={`more-menu-trigger ${isOpen ? "is-active" : ""}`}
          onClick={toggle}
          aria-haspopup="menu"
          aria-expanded={isOpen}
          aria-label={`More actions for ${session.name || "session"}`}
        >
          <Icon name="more" size={15} />
        </button>
      </Tooltip>

      {isOpen &&
        createPortal(
          <div
            ref={menuRef}
            className="more-menu-dropdown"
            style={menuPos}
            role="menu"
            aria-label={`Actions for ${session.name || "session"}`}
          >
            <button
              type="button"
              className={`more-menu-item ${copied ? "more-menu-copied" : ""}`}
              onClick={handleCopyId}
              role="menuitem"
            >
              <Icon name={copied ? "check" : "copy"} size={14} />
              <span>{copied ? "Session ID copied!" : "Copy session ID"}</span>
            </button>

            {primaryUrl && isRunning && (
              <button
                type="button"
                className="more-menu-item"
                onClick={() => {
                  setIsOpen(false);
                  void openInBrowser(primaryUrl);
                }}
                role="menuitem"
              >
                <Icon name="external" size={14} />
                <span>Open in browser</span>
              </button>
            )}

            <button
              type="button"
              className="more-menu-item"
              onClick={() => {
                setIsOpen(false);
                onViewLogs(session);
              }}
              role="menuitem"
            >
              <Icon name="terminal" size={14} />
              <span>View live logs</span>
            </button>

            {onRestart && (
              <button
                type="button"
                className="more-menu-item"
                onClick={() => {
                  setIsOpen(false);
                  void onRestart(session.id);
                }}
                disabled={isRestarting}
                role="menuitem"
              >
                <Icon
                  name="refresh"
                  size={14}
                  className={isRestarting ? "refresh-spinning" : ""}
                />
                <span>{isRestarting ? "Restarting…" : "Restart service"}</span>
              </button>
            )}

            {onRebuild && session.name && (
              <button
                type="button"
                className="more-menu-item"
                onClick={() => {
                  setIsOpen(false);
                  void onRebuild(session.name!);
                }}
                disabled={isRebuilding}
                role="menuitem"
              >
                <Icon
                  name="wrench"
                  size={14}
                  className={isRebuilding ? "refresh-spinning" : ""}
                />
                <span>{isRebuilding ? "Rebuilding…" : "Rebuild & restart"}</span>
              </button>
            )}

            {isRunning && (
              <>
                <div className="more-menu-divider" />
                <button
                  type="button"
                  className="more-menu-item more-menu-danger"
                  onClick={() => {
                    setIsOpen(false);
                    void onStop(session.id);
                  }}
                  disabled={isStopping}
                  role="menuitem"
                >
                  <Icon name="square" size={14} />
                  <span>{isStopping ? "Stopping…" : "Stop service"}</span>
                </button>
              </>
            )}
          </div>,
          document.body,
        )}
    </>
  );
}
