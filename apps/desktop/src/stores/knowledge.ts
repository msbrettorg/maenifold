import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useMemoryApi } from '../composables/useMemoryApi';
import type { CommunityGraph, ConceptGraph } from '../lib/graph-types';
import {
  createFolderNode,
  extractConceptCandidates,
  findFolderNode,
  parseListMemoriesOutput,
  type KnowledgeConceptNode,
  type KnowledgeFileEntry,
  type KnowledgeFolderNode,
} from '../lib/knowledge-helpers';
import { readStorageValue, writeStorageValue } from '../lib/local-storage';
import type { MemoryFile, MemorySearchItem, SimilarConceptItem } from '../lib/workflow-types';
import { createKnowledgeGraphActions } from './knowledge-graph-actions';

export type {
  KnowledgeConceptNode,
  KnowledgeFileEntry,
  KnowledgeFolderNode,
} from '../lib/knowledge-helpers';

export interface KnowledgeTabCacheEntry {
  title: string;
  loading: boolean;
  error: string | null;
  content?: MemoryFile;
}
export type KnowledgeSubtab = 'memory' | 'graph';
const STORAGE_NAMESPACE = 'knowledge';
const ACTIVE_SUBTAB_STORAGE_KEY = 'activeSubtab';
export const useKnowledgeStore = defineStore('knowledge', () => {
  const memoryApi = useMemoryApi();

  const folders = ref<KnowledgeFolderNode[]>([]);
  const rootFiles = ref<KnowledgeFileEntry[]>([]);
  const activeSubtab = ref<KnowledgeSubtab>(
    readStorageValue<KnowledgeSubtab>(STORAGE_NAMESPACE, ACTIVE_SUBTAB_STORAGE_KEY) === 'graph' ? 'graph' : 'memory',
  );

  const searchResults = ref<MemorySearchItem[]>([]);
  const openTabs = ref<string[]>([]);
  const activeTabUri = ref<string | null>(null);
  const tabCache = ref<Map<string, KnowledgeTabCacheEntry>>(new Map());
  const activeTabEntry = computed(() => (
    activeTabUri.value ? (tabCache.value.get(activeTabUri.value) ?? null) : null
  ));
  const activeNote = computed(() => activeTabEntry.value?.content ?? null);

  const foldersLoading = ref(false);
  const foldersError = ref<string | null>(null);

  const searchLoading = ref(false);
  const searchError = ref<string | null>(null);
  const searchQuery = ref('');

  const noteLoading = computed(() => activeTabEntry.value?.loading ?? false);
  const noteError = computed(() => activeTabEntry.value?.error ?? null);

  const graphLoading = ref(false);
  const graphError = ref<string | null>(null);
  const communityGraph = ref<CommunityGraph | null>(null);
  const conceptGraph = ref<ConceptGraph | null>(null);
  const conceptNodes = ref<KnowledgeConceptNode[]>([]);
  const activeCommunityId = ref<number | null>(null);
  const selectedConcept = ref<string | null>(null);
  const conceptContextLoading = ref(false);
  const conceptContextError = ref<string | null>(null);

  const relatedConcepts = ref<SimilarConceptItem[]>([]);
  const relatedConceptsSource = ref<string | null>(null);
  const relatedConceptsLoading = ref(false);
  const relatedConceptsError = ref<string | null>(null);

  function titleFromUri(uri: string): string {
    return decodeURIComponent(uri.replace(/^memory:\/\//, '').split('/').at(-1) ?? uri);
  }

  function setTabCache(uri: string, patch: Partial<KnowledgeTabCacheEntry>): void {
    const next = new Map(tabCache.value);
    const previous = next.get(uri) ?? {
      title: titleFromUri(uri),
      loading: false,
      error: null,
    };
    next.set(uri, { ...previous, ...patch });
    tabCache.value = next;
  }

  function setActiveSubtab(subtab: KnowledgeSubtab): void {
    activeSubtab.value = subtab;
    writeStorageValue(STORAGE_NAMESPACE, ACTIVE_SUBTAB_STORAGE_KEY, subtab);
  }
  async function loadRootFolders(): Promise<void> {
    foldersLoading.value = true;
    foldersError.value = null;

    try {
      const raw = await memoryApi.listMemories();
      const parsed = parseListMemoriesOutput(raw, '');
      folders.value = parsed.folders.map((folder) => createFolderNode(folder));
      rootFiles.value = parsed.files;
    } catch (errorValue) {
      foldersError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      folders.value = [];
      rootFiles.value = [];
    } finally {
      foldersLoading.value = false;
    }
  }

  async function loadFolder(path: string): Promise<void> {
    const folder = findFolderNode(folders.value, path);
    if (!folder || folder.loading) return;

    folder.loading = true;
    folder.error = null;

    try {
      const raw = await memoryApi.listMemories({ path });
      const parsed = parseListMemoriesOutput(raw, path);
      folder.children = parsed.folders.map((child) => createFolderNode(child));
      folder.files = parsed.files;
      folder.loaded = true;
    } catch (errorValue) {
      folder.error = errorValue instanceof Error ? errorValue.message : String(errorValue);
    } finally {
      folder.loading = false;
    }
  }

  async function setFolderOpen(path: string, open: boolean): Promise<void> {
    const folder = findFolderNode(folders.value, path);
    if (!folder) return;

    folder.open = open;
    if (open && !folder.loaded) {
      await loadFolder(path);
    }
  }

  async function search(query: string): Promise<void> {
    const normalized = query.trim();
    searchQuery.value = query;
    searchError.value = null;

    if (!normalized) {
      searchResults.value = [];
      return;
    }

    searchLoading.value = true;
    try {
      const result = await memoryApi.searchMemories(normalized);
      searchResults.value = result.results;
    } catch (errorValue) {
      searchError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      searchResults.value = [];
    } finally {
      searchLoading.value = false;
    }
  }

  async function loadRelatedConceptsForNote(note: MemoryFile | null): Promise<void> {
    relatedConceptsSource.value = null;
    relatedConceptsError.value = null;
    relatedConcepts.value = [];

    const [seedConcept] = extractConceptCandidates(note);
    if (!seedConcept) return;

    relatedConceptsSource.value = seedConcept;
    relatedConceptsLoading.value = true;
    try {
      const result = await memoryApi.findSimilarConcepts(seedConcept, 8);
      const filtered = result.results.filter(
        (item) => item.name.toLowerCase() !== seedConcept.toLowerCase(),
      );
      relatedConcepts.value = filtered;
    } catch (errorValue) {
      relatedConceptsError.value =
        errorValue instanceof Error ? errorValue.message : String(errorValue);
      relatedConcepts.value = [];
    } finally {
      relatedConceptsLoading.value = false;
    }
  }

  async function loadTab(uri: string): Promise<void> {
    const current = tabCache.value.get(uri);
    if (current?.loading) return;
    if (current?.content) {
      if (activeTabUri.value === uri) {
        await loadRelatedConceptsForNote(current.content);
      }
      return;
    }
    setTabCache(uri, { loading: true, error: null });
    try {
      const note = await memoryApi.readMemory(uri);
      setTabCache(uri, {
        title: note.title || titleFromUri(note.uri),
        loading: false,
        error: null,
        content: note,
      });
      if (activeTabUri.value === uri) {
        await loadRelatedConceptsForNote(note);
      }
    } catch (errorValue) {
      setTabCache(uri, {
        loading: false,
        error: errorValue instanceof Error ? errorValue.message : String(errorValue),
      });
      if (activeTabUri.value === uri) {
        relatedConcepts.value = [];
        relatedConceptsSource.value = null;
        relatedConceptsError.value = null;
      }
    }
  }

  function setActiveTab(uri: string): void {
    if (!openTabs.value.includes(uri)) {
      openTabs.value = [...openTabs.value, uri];
    }
    activeTabUri.value = uri;
    void loadTab(uri);
  }

  function openInNewTab(uri: string): void {
    setActiveTab(uri);
  }

  function openNote(uri: string): void {
    if (!activeTabUri.value) {
      openInNewTab(uri);
      return;
    }
    if (openTabs.value.includes(uri)) {
      setActiveTab(uri);
      return;
    }
    openTabs.value = openTabs.value.map((tabUri) => (
      tabUri === activeTabUri.value ? uri : tabUri
    ));
    activeTabUri.value = uri;
    void loadTab(uri);
  }

  function closeTab(uri: string): void {
    const closingIndex = openTabs.value.indexOf(uri);
    openTabs.value = openTabs.value.filter((tabUri) => tabUri !== uri);
    if (activeTabUri.value !== uri) return;

    const nextIndex = Math.max(0, closingIndex - 1);
    const nextUri = openTabs.value[nextIndex] ?? null;
    activeTabUri.value = nextUri;
    if (nextUri) {
      void loadTab(nextUri);
      void loadRelatedConceptsForNote(tabCache.value.get(nextUri)?.content ?? null);
      return;
    }
    relatedConcepts.value = [];
    relatedConceptsSource.value = null;
    relatedConceptsError.value = null;
  }

  const graphActions = createKnowledgeGraphActions(
    {
      graphLoading,
      graphError,
      communityGraph,
      conceptGraph,
      conceptNodes,
      activeCommunityId,
      selectedConcept,
      conceptContextLoading,
      conceptContextError,
      activeTabUri,
    },
    memoryApi,
    openNote,
  );

  return {
    folders, rootFiles, searchQuery, searchResults,
    activeSubtab, openTabs, activeTabUri, tabCache, activeNote,
    foldersLoading, foldersError, searchLoading, searchError,
    noteLoading, noteError, graphLoading, graphError,
    communityGraph, conceptGraph, conceptNodes, activeCommunityId,
    selectedConcept, conceptContextLoading, conceptContextError,
    relatedConcepts, relatedConceptsSource,
    relatedConceptsLoading, relatedConceptsError,
    loadRootFolders, loadFolder, setFolderOpen, search,
    openNote, openInNewTab, closeTab, setActiveTab, setActiveSubtab,
    ...graphActions,
  };
});
