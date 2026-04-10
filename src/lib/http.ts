/**
 * Tauri-aware fetch. In the packaged app, routes HTTP through the Rust plugin
 * (bypassing CORS). In browser mode, falls back to native fetch.
 *
 * Detection: check for the Tauri IPC channel (`window.__TAURI_INTERNALS__`)
 * before importing the plugin. The module imports fine in a browser, but its
 * fetch() throws at call-time because the Rust invoke channel is absent.
 */

let resolvedFetch: typeof globalThis.fetch | null = null;

function hasTauriRuntime(): boolean {
  return typeof window !== 'undefined' &&
    '__TAURI_INTERNALS__' in window;
}

export async function getTauriFetch(): Promise<typeof globalThis.fetch> {
  if (resolvedFetch) {
    return resolvedFetch;
  }

  if (hasTauriRuntime()) {
    try {
      const mod = await import('@tauri-apps/plugin-http');
      resolvedFetch = mod.fetch;
      return resolvedFetch;
    } catch {
      // Plugin missing — fall through to native fetch.
    }
  }

  resolvedFetch = globalThis.fetch.bind(globalThis);
  return resolvedFetch;
}
