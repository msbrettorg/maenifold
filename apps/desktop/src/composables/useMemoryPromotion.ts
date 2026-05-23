import { computed, ref } from 'vue';
import { getSharedMcpClient } from './useSharedMcpClient';
import {
  SEARCH_PAGE_SIZE,
  buildPromotedContentBlock,
  buildPromotionBlock,
  isStaleMemoryEditError,
  normalizeMemoryDocument,
  normalizeSearchResponse,
  normalizeWriteResult,
  toNonEmptyString,
  type MemoryPromotionSource,
  type NormalizedMemoryDocument,
  type NormalizedMemorySearchResult,
} from './useMemoryPromotion-helpers';

export type {
  MemoryPromotionSource,
  NormalizedMemoryDocument,
  NormalizedMemorySearchItem,
  NormalizedMemorySearchResult,
} from './useMemoryPromotion-helpers';
export {
  buildPromotedContentBlock,
  buildPromotionBlock,
  buildPromotionPreview,
  hasWikiLinkConnection,
  isStaleMemoryEditError,
  normalizeWikiLinkInput,
} from './useMemoryPromotion-helpers';

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
      const result = await mcp.callTool('ReadMemory', { identifier, includeChecksum });
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
