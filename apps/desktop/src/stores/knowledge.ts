import { defineStore } from 'pinia';
import { ref } from 'vue';
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
import type { MemoryFile, MemorySearchItem, SimilarConceptItem } from '../lib/workflow-types';
import { createKnowledgeGraphActions } from './knowledge-graph-actions';

export type {
  KnowledgeConceptNode,
  KnowledgeFileEntry,
  KnowledgeFolderNode,
} from '../lib/knowledge-helpers';

export const useKnowledgeStore = defineStore('knowledge', () => {
  const memoryApi = useMemoryApi();

  const folders = ref<KnowledgeFolderNode[]>([]);
  const rootFiles = ref<KnowledgeFileEntry[]>([]);

  const searchResults = ref<MemorySearchItem[]>([]);
  const activeUri = ref<string | null>(null);
  const activeNote = ref<MemoryFile | null>(null);

  const foldersLoading = ref(false);
  const foldersError = ref<string | null>(null);

  const searchLoading = ref(false);
  const searchError = ref<string | null>(null);
  const searchQuery = ref('');

  const noteLoading = ref(false);
  const noteError = ref<string | null>(null);

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

  async function openNote(identifier: string): Promise<void> {
    noteLoading.value = true;
    noteError.value = null;
    activeUri.value = identifier;

    try {
      activeNote.value = await memoryApi.readMemory(identifier);
      activeUri.value = activeNote.value.uri;
      await loadRelatedConceptsForNote(activeNote.value);
    } catch (errorValue) {
      noteError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      activeNote.value = null;
      relatedConcepts.value = [];
      relatedConceptsSource.value = null;
      relatedConceptsError.value = null;
    } finally {
      noteLoading.value = false;
    }
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
      activeNote,
      activeUri,
    },
    memoryApi,
    openNote,
  );

  return {
    folders,
    rootFiles,
    searchQuery,
    searchResults,
    activeUri,
    activeNote,
    foldersLoading,
    foldersError,
    searchLoading,
    searchError,
    noteLoading,
    noteError,
    graphLoading,
    graphError,
    communityGraph,
    conceptGraph,
    conceptNodes,
    activeCommunityId,
    selectedConcept,
    conceptContextLoading,
    conceptContextError,
    relatedConcepts,
    relatedConceptsSource,
    relatedConceptsLoading,
    relatedConceptsError,
    loadRootFolders,
    loadFolder,
    setFolderOpen,
    search,
    openNote,
    ...graphActions,
  };
});
