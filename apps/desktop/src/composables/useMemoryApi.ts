import type {
  MemoryFile,
  MemorySearchMode,
  RecentActivityFilter,
  RecentResult,
  SearchResult,
} from '../lib/workflow-types';
import { unwrapToolData } from './pipeline-mcp-transport';
import { getSharedMcpClient } from './usePipelineMcp';

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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function toNonEmptyString(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }

  const normalized = value.trim();
  return normalized.length > 0 ? normalized : null;
}

function extractUri(value: unknown): string | null {
  if (typeof value === 'string') {
    const direct = value.match(/memory:\/\/[^\s)]+/i);
    return direct?.[0] ?? null;
  }

  if (!isRecord(value)) {
    return null;
  }

  for (const key of ['uri', 'memoryUri', 'identifier']) {
    const candidate = toNonEmptyString(value[key]);
    if (candidate) {
      return candidate;
    }
  }

  return null;
}

function normalizeSearchItem(value: unknown): SearchResult['results'][number] | null {
  if (!isRecord(value)) {
    return null;
  }

  const uri = extractUri(value);
  if (!uri) {
    return null;
  }

  return {
    uri,
    title: toNonEmptyString(value.title) ?? uri,
    snippet: toNonEmptyString(value.snippet) ?? toNonEmptyString(value.preview) ?? '',
    score:
      typeof value.score === 'number'
        ? value.score
        : typeof value.rank === 'number'
          ? value.rank
          : 0,
  };
}

function normalizeReadMemoryResponse(value: unknown): MemoryFile {
  const data = unwrapToolData<unknown>(value);
  if (!isRecord(data)) {
    throw new Error('Memory read returned an unexpected response.');
  }

  const uri = extractUri(data);
  const content = typeof data.content === 'string' ? data.content : null;
  if (!uri || content === null) {
    throw new Error('Memory read did not include the expected content.');
  }

  return {
    uri,
    title: toNonEmptyString(data.title) ?? uri,
    content,
    created: typeof data.created === 'string' ? data.created : null,
    modified: typeof data.modified === 'string' ? data.modified : null,
  };
}

function normalizeSearchResponse(value: unknown, fallbackPageSize: number): SearchResult {
  const data = unwrapToolData<unknown>(value);
  const record = isRecord(data) ? data : {};
  const collection = Array.isArray(record.results)
    ? record.results
    : Array.isArray(record.items)
      ? record.items
      : Array.isArray(data)
        ? data
        : [];

  const results = collection
    .map((entry) => normalizeSearchItem(entry))
    .filter((entry): entry is SearchResult['results'][number] => entry !== null)
    .sort((left, right) => right.score - left.score);

  return {
    results,
    totalCount: typeof record.totalCount === 'number' ? record.totalCount : results.length,
    page: typeof record.page === 'number' ? record.page : 1,
    pageSize: typeof record.pageSize === 'number' ? record.pageSize : fallbackPageSize,
  };
}

function normalizeRecentItem(value: unknown): RecentResult['items'][number] | null {
  if (!isRecord(value)) {
    return null;
  }

  const uri = extractUri(value);
  const modified =
    toNonEmptyString(value.modified)
    ?? toNonEmptyString(value.updatedAt)
    ?? toNonEmptyString(value.created);

  if (!uri || !modified) {
    return null;
  }

  const rawSections = Array.isArray(value.sections)
    ? value.sections
    : Array.isArray(value.headings)
      ? value.headings
      : [];

  return {
    uri,
    title: toNonEmptyString(value.title) ?? uri,
    modified,
    sections: rawSections.flatMap((section) => {
      if (typeof section === 'string') {
        return [section];
      }

      if (isRecord(section)) {
        const title = toNonEmptyString(section.title) ?? toNonEmptyString(section.heading);
        return title ? [title] : [];
      }

      return [];
    }),
  };
}

function normalizeRecentResponse(value: unknown): RecentResult {
  const data = unwrapToolData<unknown>(value);
  const record = isRecord(data) ? data : {};
  const collection = Array.isArray(record.items)
    ? record.items
    : Array.isArray(record.results)
      ? record.results
      : Array.isArray(data)
        ? data
        : [];

  return {
    items: collection
      .map((entry) => normalizeRecentItem(entry))
      .filter((entry): entry is RecentResult['items'][number] => entry !== null),
  };
}

export function useMemoryApi() {
  const mcp = getSharedMcpClient();

  async function ensureConnected(): Promise<void> {
    if (mcp.connected.value) {
      return;
    }

    await mcp.connect();
  }

  async function readMemory(identifier: string): Promise<MemoryFile> {
    await ensureConnected();
    return normalizeReadMemoryResponse(
      await mcp.callTool('ReadMemory', { identifier }),
    );
  }

  async function searchMemories(
    query: string,
    opts: SearchMemoriesOptions = {},
  ): Promise<SearchResult> {
    await ensureConnected();

    const pageSize = opts.pageSize ?? 10;
    const payload: Record<string, unknown> = {
      query,
      pageSize,
    };

    if (opts.mode) {
      payload.mode = opts.mode;
    }

    if (opts.page !== undefined) {
      payload.page = opts.page;
    }

    if (opts.folder) {
      payload.folder = opts.folder;
    }

    return normalizeSearchResponse(
      await mcp.callTool('SearchMemories', payload),
      pageSize,
    );
  }

  async function recentActivity(
    opts: RecentActivityOptions = {},
  ): Promise<RecentResult> {
    await ensureConnected();

    const payload: Record<string, unknown> = {};

    if (opts.filter) {
      payload.filter = opts.filter;
    }

    if (opts.limit !== undefined) {
      payload.limit = opts.limit;
    }

    if (opts.timespan) {
      payload.timespan = opts.timespan;
    }

    if (opts.includeContent !== undefined) {
      payload.includeContent = opts.includeContent;
    }

    return normalizeRecentResponse(
      await mcp.callTool('RecentActivity', payload),
    );
  }

  return {
    readMemory,
    searchMemories,
    recentActivity,
  };
}
