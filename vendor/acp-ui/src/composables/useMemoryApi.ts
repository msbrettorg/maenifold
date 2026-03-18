import type {
  MemoryFile,
  MemorySearchMode,
  RecentActivityFilter,
  RecentResult,
  SearchResult,
} from '../lib/workflow-types';

export interface SearchMemoriesOptions {
  mode?: MemorySearchMode;
  pageSize?: number;
  page?: number;
  folder?: string;
}

export interface RecentActivityOptions {
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
 * In --app (Tauri) mode the SPA is served from tauri://localhost, so relative
 * fetch URLs never reach the axum HTTP server.  Detect Tauri via the globals
 * it injects and build an explicit http://localhost:{port} base URL instead.
 * In --http mode the SPA is served by the same axum process, so an empty base
 * (i.e. relative URLs) works fine.
 */
function getApiBase(): string {
  const isTauri =
    typeof window !== 'undefined' &&
    ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);

  if (!isTauri) {
    return '';
  }

  // window.location.port is empty when the protocol is "tauri:" — fall back to
  // the default axum server port used by both --http and --app modes.
  const port = window.location.port || '8080';
  return `http://localhost:${port}`;
}

const API_BASE = getApiBase();

async function fetchJson<T>(path: string, params: URLSearchParams): Promise<T> {
  const query = params.toString();
  const url = query ? `${API_BASE}${path}?${query}` : `${API_BASE}${path}`;
  const response = await fetch(url);

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
