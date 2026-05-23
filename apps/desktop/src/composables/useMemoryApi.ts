import type {
  BuildContextResult,
  MemoryFile,
  MemorySearchMode,
  RecentActivityFilter,
  RecentResult,
  SearchResult,
  SimilarConceptResult,
} from '../lib/workflow-types';
import {
  normalizeBuildContextResponse,
  normalizeMemoryWriteResponse,
  normalizeReadMemoryResponse,
  normalizeSimilarConceptsResponse,
} from '../lib/memory-api-helpers';
import {
  normalizeRecentResponse,
  normalizeSearchResponse,
} from '../lib/memory-api-search';
import { unwrapToolData } from '../lib/mcp-response';
import { getSharedMcpClient } from './useSharedMcpClient';

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

interface ListMemoriesOptions {
  path?: string;
}

interface WriteMemoryOptions {
  title: string;
  folder?: string;
  content: string;
}

interface EditMemoryOptions {
  identifier: string;
  content: string;
  checksum: string;
  findText: string;
}

interface MoveMemoryOptions {
  source: string;
  destination: string;
}

interface BuildContextOptions {
  depth?: number;
  maxEntities?: number;
  includeContent?: boolean;
}

export function useMemoryApi() {
  const mcp = getSharedMcpClient();

  async function ensureConnected(): Promise<void> {
    if (mcp.connected.value) return;
    await mcp.connect();
  }

  async function readMemory(identifier: string): Promise<MemoryFile> {
    await ensureConnected();
    return normalizeReadMemoryResponse(
      await mcp.gatewayExecute('maenifold.memory', 'ReadMemory', { identifier }),
    );
  }

  async function searchMemories(
    query: string,
    opts: SearchMemoriesOptions = {},
  ): Promise<SearchResult> {
    await ensureConnected();

    const pageSize = opts.pageSize ?? 10;
    const payload: Record<string, unknown> = { query, pageSize };

    if (opts.mode) payload.mode = opts.mode;
    if (opts.page !== undefined) payload.page = opts.page;
    if (opts.folder) payload.folder = opts.folder;

    return normalizeSearchResponse(
      await mcp.gatewayExecute('maenifold.memory', 'SearchMemories', payload),
      pageSize,
    );
  }

  async function recentActivity(
    opts: RecentActivityOptions = {},
  ): Promise<RecentResult> {
    await ensureConnected();

    const payload: Record<string, unknown> = {};
    if (opts.filter) payload.filter = opts.filter;
    if (opts.limit !== undefined) payload.limit = opts.limit;
    if (opts.timespan) payload.timespan = opts.timespan;
    if (opts.includeContent !== undefined) payload.includeContent = opts.includeContent;

    return normalizeRecentResponse(
      await mcp.gatewayExecute('maenifold.system', 'RecentActivity', payload),
    );
  }

  async function listMemories(opts: ListMemoriesOptions = {}): Promise<string> {
    await ensureConnected();

    const payload: Record<string, unknown> = {};
    if (opts.path) payload.path = opts.path;

    return unwrapToolData<string>(await mcp.gatewayExecute('maenifold.memory', 'ListMemories', payload));
  }

  async function writeMemory(options: WriteMemoryOptions): Promise<string> {
    await ensureConnected();
    const payload: Record<string, unknown> = {
      title: options.title,
      content: options.content,
    };
    if (options.folder?.trim()) payload.folder = options.folder.trim();
    return normalizeMemoryWriteResponse(await mcp.gatewayExecute('maenifold.memory', 'WriteMemory', payload));
  }

  async function editMemory(options: EditMemoryOptions): Promise<string> {
    await ensureConnected();
    return normalizeMemoryWriteResponse(
      await mcp.gatewayExecute('maenifold.memory', 'EditMemory', {
        identifier: options.identifier,
        operation: 'find_replace',
        findText: options.findText,
        expectedCount: 1,
        content: options.content,
        checksum: options.checksum,
      }),
      options.identifier,
    );
  }

  async function deleteMemory(identifier: string): Promise<void> {
    await ensureConnected();
    await mcp.gatewayExecute('maenifold.memory', 'DeleteMemory', { identifier, confirm: true });
  }

  async function moveMemory(options: MoveMemoryOptions): Promise<string> {
    await ensureConnected();
    const response = await mcp.gatewayExecute('maenifold.memory', 'MoveMemory', {
      source: options.source,
      destination: options.destination,
    });
    const responseText = unwrapToolData<unknown>(response);
    const arrowMatch = typeof responseText === 'string'
      ? responseText.match(/→\s*(memory:\/\/[^\s]+)/)
      : null;
    return arrowMatch?.[1] ?? options.destination;
  }

  async function buildContext(
    conceptName: string,
    opts: BuildContextOptions = {},
  ): Promise<BuildContextResult> {
    await ensureConnected();

    const payload: Record<string, unknown> = { conceptName };
    if (opts.depth !== undefined) payload.depth = opts.depth;
    if (opts.maxEntities !== undefined) payload.maxEntities = opts.maxEntities;
    if (opts.includeContent !== undefined) payload.includeContent = opts.includeContent;

    return normalizeBuildContextResponse(
      await mcp.gatewayExecute('maenifold.graph', 'BuildContext', payload),
    );
  }

  async function findSimilarConcepts(
    conceptName: string,
    maxResults = 10,
  ): Promise<SimilarConceptResult> {
    await ensureConnected();
    return normalizeSimilarConceptsResponse(
      await mcp.gatewayExecute('maenifold.graph', 'FindSimilarConcepts', { conceptName, maxResults }),
    );
  }

  return {
    readMemory,
    searchMemories,
    recentActivity,
    listMemories,
    writeMemory,
    editMemory,
    deleteMemory,
    moveMemory,
    buildContext,
    findSimilarConcepts,
  };
}
