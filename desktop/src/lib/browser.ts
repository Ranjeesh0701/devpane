import { openUrl } from "@tauri-apps/plugin-opener";

export async function openInBrowser(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch {
    window.open(url, "_blank");
  }
}

export function extractLocalhostUrls(text: string): string[] {
  if (!text) return [];
  const regex = /https?:\/\/(?:localhost|127\.0\.0\.1)(?::\d+)?(?:\/[^\s"']*)?/gi;
  const matches = text.match(regex);
  if (!matches) return [];
  // Clean trailing punctuation like ., ;, )
  const cleaned = matches.map((u) => u.replace(/[.,;)]+$/, ""));
  return Array.from(new Set(cleaned));
}
