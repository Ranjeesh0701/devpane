import { useState, useRef, useEffect, type ReactNode } from "react";
import { createPortal } from "react-dom";
import "./Tooltip.css";

type TooltipProps = {
  content: string;
  position?: "top" | "bottom" | "left" | "right";
  children: ReactNode;
  delayMs?: number;
};

export function Tooltip({
  content,
  position = "top",
  children,
  delayMs = 150,
}: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number }>({
    top: 0,
    left: 0,
  });
  const triggerRef = useRef<HTMLDivElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const calculateCoords = () => {
    if (!triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    let top = 0;
    let left = 0;

    switch (position) {
      case "top":
        top = rect.top - 6;
        left = rect.left + rect.width / 2;
        break;
      case "bottom":
        top = rect.bottom + 6;
        left = rect.left + rect.width / 2;
        break;
      case "left":
        top = rect.top + rect.height / 2;
        left = rect.left - 6;
        break;
      case "right":
        top = rect.top + rect.height / 2;
        left = rect.right + 6;
        break;
    }
    setCoords({ top, left });
  };

  const show = () => {
    timerRef.current = setTimeout(() => {
      calculateCoords();
      setVisible(true);
    }, delayMs);
  };

  const hide = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setVisible(false);
  };

  useEffect(() => {
    if (!visible) return;
    const onScrollOrResize = () => hide();
    window.addEventListener("scroll", onScrollOrResize, true);
    window.addEventListener("resize", onScrollOrResize);
    return () => {
      window.removeEventListener("scroll", onScrollOrResize, true);
      window.removeEventListener("resize", onScrollOrResize);
    };
  }, [visible]);

  return (
    <div
      ref={triggerRef}
      className="tooltip-anchor"
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocusCapture={show}
      onBlurCapture={hide}
    >
      {children}
      {visible &&
        createPortal(
          <div
            role="tooltip"
            className={`tooltip-portal-bubble tooltip-pos-${position}`}
            style={{
              top: `${coords.top}px`,
              left: `${coords.left}px`,
            }}
          >
            {content}
          </div>,
          document.body,
        )}
    </div>
  );
}
