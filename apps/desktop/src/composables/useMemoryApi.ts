import type {
  MemoryFile,
  MemorySearchMode,
  RecentActivityFilter,
  RecentResult,
  SearchResult,
} from '../lib/workflow-types';
import { getTauriFetch } from '../lib/http';

interface SearchMemoriesOptions {
  mode?: MemorySearchMode;
  pageSize?: number;
  page?: number;
  folder?: string;
}

interface RecentActivityOptions {
  filter?: RecentActivityFilter;
  limit?: number;
  timespan?: string;
  includeContent?: boolean;
}

async function getErrorMessage(response: Response): Promise<string> {
  const contentType = response.headers.get('content-type') ?? '';
  const bodyText = await response.text();

  if (contentType.includes('application/json') && bodyText.trim()) {
    try {
      const payload = JSON.parse(bodyText) as unknown;

      if (typeof payload === 'string' && payload.trim()) {
        return payload;
      }

      if (payload && typeof payload === 'object') {
        const message = 'message' in payload ? payload.message : undefined;
        if (typeof message === 'string' && message.trim()) {
          return message;
        }

        const error = 'error' in payload ? payload.error : undefined;
        if (typeof error === 'string' && error.trim()) {
          return error;
        }
      }
    } catch {
      // Fall back to the raw response body below when the server returns invalid JSON.
    }
  }

  return bodyText.trim() || `Request failed with status ${response.status}`;
}

/**
 * Derive a fetchable API origin from the current browser location.
 *
 * Returning an absolute origin keeps requests same-origin in the browser while
 * also allowing jsdom/undici integration tests to issue real network calls,
 * because Node's fetch implementation does not accept relative URLs.
 *
 * Exported for integration testing.
 */
export function getApiBase(): string {
  if (typeof window === 'undefined') {
    return '';
  }

  // Always target the maenifold backend directly.
  // In tauri dev, window.location.origin is the Vite server (localhost:1420),
  // NOT the MCP backend. Returning that origin causes 404s on /mcp and /memory.
  // Both dev and production use tauri-plugin-http which bypasses CORS.
  return import.meta.env.VITE_MCP_BASE_URL ?? 'http://127.0.0.1:8080';
}

async function fetchJson<T>(path: string, params: URLSearchParams): Promise<T> {
  const apiBase = getApiBase();
  const query = params.toString();
  const url = query ? `${apiBase}${path}?${query}` : `${apiBase}${path}`;
  const httpFetch = await getTauriFetch();
  const response = await httpFetch(url);

  if (!response.ok) {
    throw new Error(await getErrorMessage(response));
  }

  return await response.json() as T;
}

export function useMemoryApi() {
  async function readMemory(uri: string): Promise<MemoryFile> {
    const params = new URLSearchParams({ uri });
    return fetchJson<MemoryFile>('/api/memory/read', params);
  }

  async function searchMemories(
    query: string,
    opts: SearchMemoriesOptions = {}
  ): Promise<SearchResult> {
    const params = new URLSearchParams({ query });

    if (opts.mode) {
      params.set('mode', opts.mode);
    }

    if (opts.pageSize !== undefined) {
      params.set('pageSize', String(opts.pageSize));
    }

    if (opts.page !== undefined) {
      params.set('page', String(opts.page));
    }

    if (opts.folder) {
      params.set('folder', opts.folder);
    }

    return fetchJson<SearchResult>('/api/memory/search', params);
  }

  async function recentActivity(
    opts: RecentActivityOptions = {}
  ): Promise<RecentResult> {
    const params = new URLSearchParams();

    if (opts.filter) {
      params.set('filter', opts.filter);
    }

    if (opts.limit !== undefined) {
      params.set('limit', String(opts.limit));
    }

    if (opts.timespan) {
      params.set('timespan', opts.timespan);
    }

    if (opts.includeContent !== undefined) {
      params.set('includeContent', String(opts.includeContent));
    }

    return fetchJson<RecentResult>('/api/memory/recent', params);
  }

  return {
    readMemory,
    searchMemories,
    recentActivity,
  };
}
