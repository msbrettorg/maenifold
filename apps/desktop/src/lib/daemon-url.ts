export const DAEMON_BASE_URL = import.meta.env.VITE_DAEMON_URL ?? 'http://localhost:8080';

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
