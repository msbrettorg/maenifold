import type {
  BuildContextResult,
  MemoryFile,
  MemorySearchMode,
  RecentActivityFilter,
  RecentResult,
  SearchResult,
  SimilarConceptResult,
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

interface ListMemoriesOptions {
  path?: string;
}

interface BuildContextOptions {
  depth?: number;
  maxEntities?: number;
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

function extractLineageValue(record: Record<string, unknown>, ...keys: string[]): string | null {
  for (const key of keys) {
    const value = toNonEmptyString(record[key]);
    if (value) {
      return value;
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

  const metadata = isRecord(data.metadata)
    ? data.metadata
    : isRecord(data.frontmatter)
      ? data.frontmatter
      : null;

  const metadataRecord = metadata ?? {};

  return {
    uri,
    title: toNonEmptyString(data.title) ?? uri,
    content,
    created: typeof data.created === 'string' ? data.created : null,
    modified: typeof data.modified === 'string' ? data.modified : null,
    lineageGateTaskId: extractLineageValue(data, 'lineageGateTaskId', 'lineage_gate_task_id')
      ?? extractLineageValue(metadataRecord, 'lineageGateTaskId', 'lineage_gate_task_id'),
    lineagePipelineRunId: extractLineageValue(data, 'lineagePipelineRunId', 'lineage_pipeline_run_id')
      ?? extractLineageValue(metadataRecord, 'lineagePipelineRunId', 'lineage_pipeline_run_id'),
    lineageThinkingSessionId: extractLineageValue(data, 'lineageThinkingSessionId', 'lineage_thinking_session_id')
      ?? extractLineageValue(metadataRecord, 'lineageThinkingSessionId', 'lineage_thinking_session_id'),
    lineageMemoryNoteId: extractLineageValue(data, 'lineageMemoryNoteId', 'lineage_memory_note_id')
      ?? extractLineageValue(metadataRecord, 'lineageMemoryNoteId', 'lineage_memory_note_id'),
    parentWorkflowId: extractLineageValue(data, 'parentWorkflowId', 'parent_workflow_id')
      ?? extractLineageValue(metadataRecord, 'parentWorkflowId', 'parent_workflow_id'),
    ...(metadata ? { metadata } : {}),
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

function normalizeBuildContextResponse(value: unknown): BuildContextResult {
  const data = unwrapToolData<unknown>(value);
  const record = isRecord(data) ? data : {};
  const directRelations = Array.isArray(record.DirectRelations)
    ? record.DirectRelations
    : Array.isArray(record.directRelations)
      ? record.directRelations
      : [];
  const expandedRelations = Array.isArray(record.ExpandedRelations)
    ? record.ExpandedRelations
    : Array.isArray(record.expandedRelations)
      ? record.expandedRelations
      : [];

  const normalizeRelation = (entry: unknown) => {
    if (!isRecord(entry)) {
      return null;
    }

    const name = toNonEmptyString(entry.Name) ?? toNonEmptyString(entry.name);
    if (!name) {
      return null;
    }

    const files = Array.isArray(entry.Files)
      ? entry.Files
      : Array.isArray(entry.files)
        ? entry.files
        : [];

    return {
      name,
      coOccurrenceCount:
        typeof entry.CoOccurrenceCount === 'number'
          ? entry.CoOccurrenceCount
          : typeof entry.coOccurrenceCount === 'number'
            ? entry.coOccurrenceCount
            : 0,
      files: files.flatMap((file) => (typeof file === 'string' ? [file] : [])),
      weightedScore:
        typeof entry.WeightedScore === 'number'
          ? entry.WeightedScore
          : typeof entry.weightedScore === 'number'
            ? entry.weightedScore
            : null,
      communityId:
        typeof entry.CommunityId === 'number'
          ? entry.CommunityId
          : typeof entry.communityId === 'number'
            ? entry.communityId
            : null,
    };
  };

  const conceptName = toNonEmptyString(record.ConceptName) ?? toNonEmptyString(record.conceptName) ?? '';
  if (!conceptName) {
    throw new Error('BuildContext did not include a concept name.');
  }

  return {
    conceptName,
    depth:
      typeof record.Depth === 'number'
        ? record.Depth
        : typeof record.depth === 'number'
          ? record.depth
          : 0,
    communityId:
      typeof record.CommunityId === 'number'
        ? record.CommunityId
        : typeof record.communityId === 'number'
          ? record.communityId
          : null,
    directRelations: directRelations.flatMap((entry) => {
      const relation = normalizeRelation(entry);
      return relation ? [relation] : [];
    }),
    expandedRelations: expandedRelations.flatMap((entry) => {
      const relation = normalizeRelation(entry);
      return relation ? [relation] : [];
    }),
  };
}

function normalizeSimilarConceptsResponse(value: unknown): SimilarConceptResult {
  const data = unwrapToolData<unknown>(value);
  const record = isRecord(data) ? data : {};
  const conceptName = toNonEmptyString(record.conceptName) ?? toNonEmptyString(record.ConceptName) ?? '';
  if (!conceptName) {
    throw new Error('FindSimilarConcepts did not include a concept name.');
  }

  const collection = Array.isArray(record.results)
    ? record.results
    : Array.isArray(record.Results)
      ? record.Results
      : [];

  return {
    conceptName,
    totalCount:
      typeof record.totalCount === 'number'
        ? record.totalCount
        : typeof record.TotalCount === 'number'
          ? record.TotalCount
          : collection.length,
    results: collection.flatMap((entry) => {
      if (!isRecord(entry)) {
        return [];
      }
      const name = toNonEmptyString(entry.name) ?? toNonEmptyString(entry.Name);
      if (!name) {
        return [];
      }
      return [{
        name,
        similarity:
          typeof entry.similarity === 'number'
            ? entry.similarity
            : typeof entry.Similarity === 'number'
              ? entry.Similarity
              : 0,
        weightedSimilarity:
          typeof entry.weightedSimilarity === 'number'
            ? entry.weightedSimilarity
            : typeof entry.WeightedSimilarity === 'number'
              ? entry.WeightedSimilarity
              : null,
      }];
    }),
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

  async function listMemories(opts: ListMemoriesOptions = {}): Promise<string> {
    await ensureConnected();

    const payload: Record<string, unknown> = {};
    if (opts.path) {
      payload.path = opts.path;
    }

    return unwrapToolData<string>(await mcp.callTool('ListMemories', payload));
  }

  async function buildContext(
    conceptName: string,
    opts: BuildContextOptions = {},
  ): Promise<BuildContextResult> {
    await ensureConnected();

    const payload: Record<string, unknown> = { conceptName };
    if (opts.depth !== undefined) {
      payload.depth = opts.depth;
    }
    if (opts.maxEntities !== undefined) {
      payload.maxEntities = opts.maxEntities;
    }
    if (opts.includeContent !== undefined) {
      payload.includeContent = opts.includeContent;
    }

    return normalizeBuildContextResponse(
      await mcp.callTool('BuildContext', payload),
    );
  }

  async function findSimilarConcepts(
    conceptName: string,
    maxResults = 10,
  ): Promise<SimilarConceptResult> {
    await ensureConnected();
    return normalizeSimilarConceptsResponse(
      await mcp.callTool('FindSimilarConcepts', { conceptName, maxResults }),
    );
  }

  return {
    readMemory,
    searchMemories,
    recentActivity,
    listMemories,
    buildContext,
    findSimilarConcepts,
  };
}
