import { computed, ref } from 'vue';
import { unwrapToolData } from './pipeline-mcp-transport';
import { getSharedMcpClient } from './usePipelineMcp';

const SEARCH_PAGE_SIZE = 5;
const WIKILINK_PATTERN = /\[\[([^[\]]+)\]\]/g;

export interface MemoryPromotionSource {
  pipelineId: string;
  nodeId: string;
  output: string;
}

export interface NormalizedMemorySearchItem {
  uri: string;
  title: string;
  snippet: string;
  score: number;
}

export interface NormalizedMemorySearchResult {
  results: NormalizedMemorySearchItem[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface NormalizedMemoryDocument {
  uri: string;
  title: string;
  content: string;
  checksum: string | null;
  created: string | null;
  modified: string | null;
}

interface SearchMemoriesPayload extends Record<string, unknown> {
  query: string;
  mode: 'Hybrid';
  pageSize: number;
}

interface WriteMemoryPayload extends Record<string, unknown> {
  title: string;
  content: string;
  folder?: string;
  tags?: string[];
}

interface EditMemoryPayload extends Record<string, unknown> {
  identifier: string;
  operation: 'append' | 'replace_section';
  content: string;
  sectionName?: string;
  checksum?: string;
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

function normalizeSearchItem(value: unknown): NormalizedMemorySearchItem | null {
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
    score: typeof value.score === 'number'
      ? value.score
      : typeof value.rank === 'number'
        ? value.rank
        : 0,
  };
}

function normalizeSearchResponse(value: unknown): NormalizedMemorySearchResult {
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
    .filter((entry): entry is NormalizedMemorySearchItem => entry !== null)
    .sort((left, right) => right.score - left.score);

  return {
    results,
    totalCount: typeof record.totalCount === 'number' ? record.totalCount : results.length,
    page: typeof record.page === 'number' ? record.page : 1,
    pageSize: typeof record.pageSize === 'number' ? record.pageSize : SEARCH_PAGE_SIZE,
  };
}

function normalizeMemoryDocument(value: unknown): NormalizedMemoryDocument {
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
    checksum: toNonEmptyString(data.checksum),
    created: typeof data.created === 'string' ? data.created : null,
    modified: typeof data.modified === 'string' ? data.modified : null,
  };
}

function normalizeWriteResult(value: unknown, fallbackUri: string | null = null): string {
  const data = unwrapToolData<unknown>(value);
  const uri = extractUri(data) ?? fallbackUri;
  if (!uri) {
    throw new Error('Memory write completed without returning a memory URI.');
  }
  return uri;
}

function normalizeConceptName(value: string): string {
  return value
    .trim()
    .replace(/\[\[|\]\]/g, '')
    .replace(/[_/]+/g, '-')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
    .toLowerCase();
}

export function normalizeWikiLinkInput(input: string): string[] {
  const explicitMatches = Array.from(input.matchAll(WIKILINK_PATTERN)).map((match) => match[1] ?? '');
  const fallbackTokens = explicitMatches.length > 0
    ? []
    : input
      .split(/[\n,;]/)
      .map((token) => token.trim())
      .filter(Boolean);

  const sourceTokens = explicitMatches.length > 0 ? explicitMatches : fallbackTokens;
  const deduped = new Set<string>();

  for (const token of sourceTokens) {
    const normalized = normalizeConceptName(token);
    if (normalized) {
      deduped.add(`[[${normalized}]]`);
    }
  }

  return [...deduped];
}

export function hasWikiLinkConnection(input: string): boolean {
  return normalizeWikiLinkInput(input).length > 0;
}

export function buildPromotedContentBlock(source: MemoryPromotionSource, wikiLinkInput: string, promotedAt: string): string {
  const wikiLinks = normalizeWikiLinkInput(wikiLinkInput);
  if (wikiLinks.length === 0) {
    throw new Error('At least one connection is required.');
  }

  return [
    `Connections: ${wikiLinks.join(' ')}`,
    '',
    source.output,
    '',
    `Source: Pipeline \`${source.pipelineId}\` · Node: \`${source.nodeId}\` · Promoted: \`${promotedAt}\``,
  ].join('\n');
}

export function buildPromotionBlock(source: MemoryPromotionSource, wikiLinkInput: string, promotedAt: string): string {
  return [
    '## Promoted Content',
    '',
    buildPromotedContentBlock(source, wikiLinkInput, promotedAt),
  ].join('\n');
}

export function buildPromotionPreview(source: MemoryPromotionSource, wikiLinkInput: string, promotedAt: string): string {
  return buildPromotionBlock(source, wikiLinkInput, promotedAt);
}

export function isStaleMemoryEditError(errorValue: unknown): boolean {
  const message = errorValue instanceof Error ? errorValue.message : String(errorValue);
  return /checksum|stale|modified/i.test(message);
}

export function useMemoryPromotion(baseUrl?: string) {
  const mcp = getSharedMcpClient(baseUrl);
  const searching = ref(false);
  const writing = ref(false);
  const error = ref<string | null>(null);
  const writtenUri = ref<string | null>(null);
  const busy = computed(() => searching.value || writing.value);

  async function ensureConnected() {
    if (mcp.connected.value) {
      return;
    }

    await mcp.connect();
  }

  async function searchMemories(query: string): Promise<NormalizedMemorySearchResult> {
    error.value = null;
    writtenUri.value = null;
    searching.value = true;

    try {
      await ensureConnected();
      const result = await mcp.callTool<SearchMemoriesPayload>('SearchMemories', {
        query,
        mode: 'Hybrid',
        pageSize: SEARCH_PAGE_SIZE,
      });
      return normalizeSearchResponse(result);
    } catch (errorValue) {
      error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      throw errorValue;
    } finally {
      searching.value = false;
    }
  }

  async function readMemory(identifier: string, includeChecksum = false): Promise<NormalizedMemoryDocument> {
    error.value = null;

    try {
      await ensureConnected();
      const result = await mcp.callTool('ReadMemory', {
        identifier,
        includeChecksum,
      });
      return normalizeMemoryDocument(result);
    } catch (errorValue) {
      error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      throw errorValue;
    }
  }

  async function writeMemory(source: MemoryPromotionSource, options: {
    title: string;
    wikiLinkInput: string;
    folder?: string;
    tags?: string[];
  }): Promise<string> {
    error.value = null;
    writtenUri.value = null;
    writing.value = true;

    try {
      const content = buildPromotionBlock(source, options.wikiLinkInput, new Date().toISOString());
      await ensureConnected();
      const payload: WriteMemoryPayload = {
        title: options.title.trim(),
        content,
      };

      if (options.folder?.trim()) {
        payload.folder = options.folder.trim();
      }

      if (options.tags && options.tags.length > 0) {
        payload.tags = options.tags;
      }

      const result = await mcp.callTool('WriteMemory', payload);
      const uri = normalizeWriteResult(result);
      writtenUri.value = uri;
      return uri;
    } catch (errorValue) {
      error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      throw errorValue;
    } finally {
      writing.value = false;
    }
  }

  async function appendMemory(source: MemoryPromotionSource, options: {
    identifier: string;
    wikiLinkInput: string;
  }): Promise<string> {
    error.value = null;
    writtenUri.value = null;
    writing.value = true;

    try {
      const block = buildPromotionBlock(source, options.wikiLinkInput, new Date().toISOString());
      await ensureConnected();
      const payload: EditMemoryPayload = {
        identifier: options.identifier,
        operation: 'append',
        content: `\n\n${block}`,
      };
      const result = await mcp.callTool('EditMemory', payload);
      const uri = normalizeWriteResult(result, options.identifier);
      writtenUri.value = uri;
      return uri;
    } catch (errorValue) {
      error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      throw errorValue;
    } finally {
      writing.value = false;
    }
  }

  async function updateMemory(source: MemoryPromotionSource, options: {
    identifier: string;
    wikiLinkInput: string;
    checksum: string | null;
  }): Promise<string> {
    error.value = null;
    writtenUri.value = null;
    writing.value = true;

    try {
      const checksum = toNonEmptyString(options.checksum);
      if (!checksum) {
        throw new Error('A checksum is required before updating this note.');
      }

      const content = buildPromotedContentBlock(source, options.wikiLinkInput, new Date().toISOString());
      await ensureConnected();
      const payload: EditMemoryPayload = {
        identifier: options.identifier,
        operation: 'replace_section',
        sectionName: 'Promoted Content',
        content,
        checksum,
      };
      const result = await mcp.callTool('EditMemory', payload);
      const uri = normalizeWriteResult(result, options.identifier);
      writtenUri.value = uri;
      return uri;
    } catch (errorValue) {
      error.value = isStaleMemoryEditError(errorValue)
        ? 'File was modified — please retry.'
        : errorValue instanceof Error
          ? errorValue.message
          : String(errorValue);
      throw errorValue;
    } finally {
      writing.value = false;
    }
  }

  function resetResultState() {
    error.value = null;
    writtenUri.value = null;
  }

  return {
    searching,
    writing,
    busy,
    error,
    writtenUri,
    searchMemories,
    readMemory,
    writeMemory,
    appendMemory,
    updateMemory,
    resetResultState,
  };
}
