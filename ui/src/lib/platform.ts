// Platform detection helpers for the frontend.
//
// Uses `navigator.userAgent` which works inside both desktop Tauri webviews
// (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux) and the
// mobile webviews that Tauri 2 ships on iOS / Android, without requiring an
// extra Tauri plugin or Rust round-trip.
//
// In addition to the OS, we distinguish between running inside a Tauri
// shell ("Tauri host") vs a plain browser ("web"). The Tauri 2 runtime
// injects a `__TAURI_INTERNALS__` global into the webview before the page
// scripts execute, so its presence is a reliable signal.

export type Platform =
  | 'macos'
  | 'linux'
  | 'windows'
  | 'ios'
  | 'android'
  | 'unknown';

let cached: Platform | null = null;

/** Detect the current platform from `navigator.userAgent`. */
export function getPlatform(): Platform {
  if (cached !== null) return cached;
  const ua = typeof navigator !== 'undefined' ? navigator.userAgent || '' : '';
  // iPadOS 13+ reports as Mac in UA; treat touch-capable Macs as iOS.
  const isIPad =
    /Macintosh/i.test(ua) &&
    typeof navigator !== 'undefined' &&
    (navigator as Navigator & { maxTouchPoints?: number }).maxTouchPoints !==
      undefined &&
    ((navigator as Navigator & { maxTouchPoints: number }).maxTouchPoints ?? 0) > 1;
  if (/iPhone|iPad|iPod/i.test(ua) || isIPad) cached = 'ios';
  else if (/Android/i.test(ua)) cached = 'android';
  else if (/Mac/i.test(ua)) cached = 'macos';
  else if (/Windows/i.test(ua)) cached = 'windows';
  else if (/Linux/i.test(ua)) cached = 'linux';
  else cached = 'unknown';
  return cached;
}

/** True if the page is running inside a Tauri webview (desktop or mobile). */
export function isTauriHost(): boolean {
  if (typeof window === 'undefined') return false;
  // Tauri 2 injects `__TAURI_INTERNALS__` into the webview at startup. The
  // older `__TAURI__` global is also kept for compatibility.
  const w = window as unknown as Record<string, unknown>;
  return (
    typeof w.__TAURI_INTERNALS__ !== 'undefined' ||
    typeof w.__TAURI__ !== 'undefined'
  );
}

/** True when running as a plain web app in a browser tab (not Tauri). */
export function isWeb(): boolean {
  return !isTauriHost();
}

/** True for iOS / Android **inside a Tauri shell**. Plain mobile browsers are
 * reported as `web` instead, since they have no native bridge. */
export function isMobile(): boolean {
  if (!isTauriHost()) return false;
  const p = getPlatform();
  return p === 'ios' || p === 'android';
}

/** True for macOS / Linux / Windows **inside a Tauri shell**. */
export function isDesktop(): boolean {
  if (!isTauriHost()) return false;
  const p = getPlatform();
  return p === 'macos' || p === 'linux' || p === 'windows';
}

/** True when the current host can only talk to remote (websocket / http)
 * agents — i.e. mobile Tauri or any browser. Used to hide stdio agents from
 * the UI and reject them in the config form. */
export function restrictedTransports(): boolean {
  return isMobile() || isWeb();
}

/** True when the host exposes a real local filesystem the agent can poke at
 * via the `fs/*` ACP RPCs. Only Tauri desktop qualifies. */
export function hasLocalFs(): boolean {
  return isDesktop();
}
