import type { ReactNode, SVGProps } from "react";

type IconName =
  | "grid"
  | "activity"
  | "settings"
  | "terminal"
  | "plus"
  | "refresh"
  | "search"
  | "chevron"
  | "command"
  | "more"
  | "clock"
  | "alert"
  | "sun"
  | "moon"
  | "check"
  | "copy"
  | "square"
  | "loader"
  | "close"
  | "globe"
  | "external";

const paths: Record<IconName, ReactNode> = {
  globe: (
    <>
      <circle cx="12" cy="12" r="10" />
      <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20M2 12h20" />
    </>
  ),
  external: (
    <>
      <path d="M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
    </>
  ),
  grid: (
    <>
      <rect x="3.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="1.5" />
    </>
  ),
  activity: <path d="M3 12h4l3-8 4 16 3-8h4" />,
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path
        d="M19 13.5a7 7 0 0 0 0-3l1.5-1.2-1.8-3.1-1.8.6a7 7 0 0 0-2.6-1.5L14 3h-4l-.3 2.3A7 7 0 0 0 7.1 6.8l-1.8-.6-1.8 3.1L5 10.5a7 7 0 0 0 0 3l-1.5 1.2 1.8 3.1 1.8-.6a7 7 0 0 0 2.6 1.5L10 21h4l.3-2.3a7 7 0 0 0 2.6-1.5l1.8.6 1.8-3.1Z"
        transform="translate(0 -0.2) scale(1)"
      />
    </>
  ),
  terminal: (
    <>
      <path d="m4 5 6 6-6 6" />
      <path d="M12 18h8" />
    </>
  ),
  plus: (
    <>
      <path d="M12 5v14M5 12h14" />
    </>
  ),
  refresh: (
    <>
      <path d="M20 7v5h-5" />
      <path d="M4 17v-5h5" />
      <path d="M5.6 9A7 7 0 0 1 18 6l2 6M4 12l2 6a7 7 0 0 0 12.4-3" />
    </>
  ),
  search: (
    <>
      <circle cx="10.8" cy="10.8" r="6.8" />
      <path d="m16 16 4 4" />
    </>
  ),
  chevron: <path d="m9 18 6-6-6-6" />,
  command: (
    <>
      <path d="M9 7V5a2 2 0 1 0-2 2h10a2 2 0 1 0-2-2v14a2 2 0 1 0 2-2H7a2 2 0 1 0 2 2V7Z" />
    </>
  ),
  more: (
    <>
      <circle cx="5" cy="12" r="1" />
      <circle cx="12" cy="12" r="1" />
      <circle cx="19" cy="12" r="1" />
    </>
  ),
  clock: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3 2" />
    </>
  ),
  alert: (
    <>
      <path d="M12 3 2.8 19h18.4L12 3Z" />
      <path d="M12 9v4m0 3h.01" />
    </>
  ),
  sun: (
    <>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2m0 16v2M4.93 4.93l1.42 1.42m11.3 11.3 1.42 1.42M2 12h2m16 0h2M4.93 19.07l1.42-1.42m11.3-11.3 1.42-1.42" />
    </>
  ),
  moon: <path d="M20.8 14.2A8.5 8.5 0 0 1 9.8 3.2 8.6 8.6 0 1 0 20.8 14.2Z" />,
  check: <path d="m5 12 4 4L19 6" />,
  copy: (
    <>
      <rect x="8" y="8" width="12" height="12" rx="2" />
      <path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3" />
    </>
  ),
  square: <rect x="6" y="6" width="12" height="12" rx="2" />,
  loader: (
    <>
      <path d="M12 2v4" />
      <path d="m16.2 7.8 2.8-2.8" />
      <path d="M18 12h4" />
    </>
  ),
  close: (
    <>
      <path d="m18 6-12 12" />
      <path d="m6 6 12 12" />
    </>
  ),
};

type Props = SVGProps<SVGSVGElement> & { name: IconName; size?: number };

export function Icon({ name, size = 18, ...props }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      {paths[name]}
    </svg>
  );
}
