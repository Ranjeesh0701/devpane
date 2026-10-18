type PostHogInstance = typeof import("posthog-js").default;

let posthogInstance: PostHogInstance | null = null;
let isInitialized = false;

export async function initAnalytics() {
  if (isInitialized) return;

  const key = import.meta.env.VITE_POSTHOG_KEY;
  const host = import.meta.env.VITE_POSTHOG_HOST || "https://us.i.posthog.com";

  if (key) {
    try {
      const { default: posthog } = await import("posthog-js");
      posthog.init(key, {
        api_host: host,
        autocapture: true,
        capture_pageview: true,
        capture_pageleave: true,
        persistence: "localStorage",
      });
      posthogInstance = posthog;
      isInitialized = true;
    } catch (e) {
      console.warn("Failed to initialize analytics", e);
    }
  }
}

export function trackEvent(name: string, properties?: Record<string, unknown>) {
  if (posthogInstance) {
    posthogInstance.capture(name, properties);
  } else if (import.meta.env.DEV) {
    console.debug(`[Analytics event] ${name}`, properties);
  }
}

export function trackDownload(platform: string, target: string) {
  trackEvent("download_clicked", {
    platform,
    target,
  });
}

export function trackCopyInstall(os: string, command: string) {
  trackEvent("install_command_copied", {
    os,
    command,
  });
}

export function trackGithubClick(location: string) {
  trackEvent("github_link_clicked", {
    location,
  });
}
