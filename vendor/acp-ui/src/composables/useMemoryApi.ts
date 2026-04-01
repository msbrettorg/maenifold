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
 * In Tauri v2 with frontendDist, the webview loads from http://localhost:{port}
 * — the same origin as the axum server — so relative fetch URLs work without
 * any base URL override.  Only fall back to an explicit http://localhost:8080
 * base if the page is served under a non-HTTP protocol (e.g. a legacy
 * tauri://localhost origin), which would make relative URLs unreachable.
 *
 * Exported for unit testing.
 */
export function getApiBase(): string {
  if (typeof window === 'undefined') {
    return '';
  }

  const { protocol } = window.location;
  if (protocol === 'http:' || protocol === 'https:') {
    // Same-origin: relative URLs reach the axum server directly.
    return '';
  }

  // Exotic protocol (e.g. tauri:) — construct an explicit base URL.
  return 'http://localhost:8080';
}

async function fetchJson<T>(path: string, params: URLSearchParams): Promise<T> {
  const apiBase = getApiBase();
  const query = params.toString();
  const url = query ? `${apiBase}${path}?${query}` : `${apiBase}${path}`;
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
