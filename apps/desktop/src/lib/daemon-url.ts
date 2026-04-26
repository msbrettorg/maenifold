export const DAEMON_BASE_URL = import.meta.env.VITE_DAEMON_URL ?? 'http://localhost:8080';
const AUTH_FAILED_MESSAGE = 'Authorization failed. The daemon may have restarted — please reload the app.';

export function getDaemonUrl(
  path = '',
  query?: Record<string, string | number | boolean | null | undefined>,
): string {
  const url = new URL(path ? (path.startsWith('/') ? path : `/${path}`) : '', DAEMON_BASE_URL);
  if (query) {
    for (const [key, value] of Object.entries(query)) {
      if (value != null) {
        url.searchParams.set(key, String(value));
      }
    }
  }
  return url.toString();
}

export async function authenticatedFetch(
  input: RequestInfo | URL,
  init: RequestInit = {},
): Promise<Response> {
  if (typeof window === 'undefined' || !window.electronAPI?.getAuthToken) {
    return fetch(input, init);
  }

  const buildInit = async (): Promise<RequestInit> => {
    const token = await window.electronAPI?.getAuthToken?.();
    if (!token) throw new Error(AUTH_FAILED_MESSAGE);
    const headers = new Headers(init.headers);
    headers.set('Authorization', `Bearer ${token}`);
    return { ...init, headers };
  };

  let response = await fetch(input, await buildInit());
  if (response.status !== 401) return response;

  response = await fetch(input, await buildInit());
  if (response.status === 401) {
    throw new Error(`${AUTH_FAILED_MESSAGE} Retry the action after the daemon is ready.`);
  }
  return response;
}
