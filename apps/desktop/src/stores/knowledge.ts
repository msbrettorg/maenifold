import { defineStore } from 'pinia';
import { ref } from 'vue';
import { useMemoryApi } from '../composables/useMemoryApi';
import { getDaemonUrl } from '../lib/daemon-url';
import type { CommunityGraph, ConceptGraph } from '../lib/graph-types';
import type { MemoryFile, MemorySearchItem, SimilarConceptItem } from '../lib/workflow-types';

export interface KnowledgeFileEntry {
  name: string;
  path: string;
  uri: string;
}

export interface KnowledgeFolderNode {
  name: string;
  path: string;
  fileCount: number;
  open: boolean;
  loaded: boolean;
  loading: boolean;
  error: string | null;
  children: KnowledgeFolderNode[];
  files: KnowledgeFileEntry[];
}

export interface KnowledgeConceptNode {
  name: string;
  occurrences: number;
  edgeCount: number;
}

interface ParsedListing {
  folders: Array<{ name: string; path: string; fileCount: number }>;
  files: KnowledgeFileEntry[];
}

function trimPath(value: string): string {
  return value.trim().replace(/^\/+|\/+$/g, '');
}

function joinPath(base: string, segment: string): string {
  const normalizedBase = trimPath(base);
  const normalizedSegment = trimPath(segment);
  if (!normalizedBase) {
    return normalizedSegment;
  }
  if (!normalizedSegment) {
    return normalizedBase;
  }
  return `${normalizedBase}/${normalizedSegment}`;
}

function buildMemoryUri(path: string, fileName: string): string {
  const stem = fileName.replace(/\.md$/i, '');
  const targetPath = joinPath(path, stem);
  return `memory://${targetPath}`;
}

function parseListMemoriesOutput(output: string, currentPath: string): ParsedListing {
  const folders: ParsedListing['folders'] = [];
  const files: ParsedListing['files'] = [];
  const lines = output.split('\n');
  let section: 'folders' | 'files' | null = null;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }

    if (trimmed === '### Folders') {
      section = 'folders';
      continue;
    }

    if (trimmed === '### Files') {
      section = 'files';
      continue;
    }

    if (section === 'folders') {
      const match = trimmed.match(/^📁\s+(.+?)\/\s+\((\d+)\s+files\)$/);
      if (!match) {
        continue;
      }
      const folderName = match[1] ?? '';
      const fileCount = Number(match[2] ?? 0);
      folders.push({
        name: folderName,
        path: joinPath(currentPath, folderName),
        fileCount: Number.isFinite(fileCount) ? fileCount : 0,
      });
      continue;
    }

    if (section === 'files') {
      const match = trimmed.match(/^📄\s+(.+?)\s+\([^)]+\)$/);
      if (!match) {
        continue;
      }
      const fileName = (match[1] ?? '').trim();
      files.push({
        name: fileName,
        path: joinPath(currentPath, fileName),
        uri: buildMemoryUri(currentPath, fileName),
      });
    }
  }

  return { folders, files };
}

function createFolderNode(
  input: ParsedListing['folders'][number],
  open = false,
): KnowledgeFolderNode {
  return {
    name: input.name,
    path: input.path,
    fileCount: input.fileCount,
    open,
    loaded: false,
    loading: false,
    error: null,
    children: [],
    files: [],
  };
}

function findFolderNode(
  folders: KnowledgeFolderNode[],
  targetPath: string,
): KnowledgeFolderNode | null {
  for (const folder of folders) {
    if (folder.path === targetPath) {
      return folder;
    }
    const nested = findFolderNode(folder.children, targetPath);
    if (nested) {
      return nested;
    }
  }
  return null;
}

function normalizeConceptName(rawValue: string): string {
  const normalized = rawValue.trim();
  const aliasIndex = normalized.indexOf('|');
  if (aliasIndex > 0) {
    return normalized.slice(0, aliasIndex).trim();
  }
  return normalized;
}

function extractConceptCandidates(note: MemoryFile | null): string[] {
  if (!note) {
    return [];
  }

  const fromWikiLinks = [...note.content.matchAll(/\[\[([^\]]+)\]\]/g)]
    .map((match) => normalizeConceptName(match[1] ?? ''))
    .filter((value) => value.length > 0);

  if (fromWikiLinks.length > 0) {
    return fromWikiLinks;
  }

  const fromTitle = note.title.trim();
  if (fromTitle.length > 0 && !fromTitle.startsWith('memory://')) {
    return [fromTitle];
  }

  const uriPath = note.uri.replace(/^memory:\/\//, '');
  const uriStem = uriPath.split('/').at(-1)?.trim() ?? '';
  if (!uriStem) {
    return [];
  }

  return [uriStem.replace(/[-_]+/g, ' ')];
}

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
    if (!folder || folder.loading) {
      return;
    }

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
    if (!folder) {
      return;
    }

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
    if (!seedConcept) {
      return;
    }

    relatedConceptsSource.value = seedConcept;
    relatedConceptsLoading.value = true;
    try {
      const result = await memoryApi.findSimilarConcepts(seedConcept, 8);
      const filtered = result.results.filter(
        (item) => item.name.toLowerCase() !== seedConcept.toLowerCase(),
      );
      relatedConcepts.value = filtered;
    } catch (errorValue) {
      relatedConceptsError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
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

  async function loadGraphCommunities(): Promise<void> {
    graphLoading.value = true;
    graphError.value = null;

    try {
      const response = await fetch(getDaemonUrl('/api/graph/communities'));
      if (!response.ok) {
        throw new Error((await response.text()) || `Failed to load graph communities (${response.status}).`);
      }

      communityGraph.value = await response.json() as CommunityGraph;
    } catch (errorValue) {
      graphError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      communityGraph.value = null;
    } finally {
      graphLoading.value = false;
    }
  }

  async function loadGraphConcepts(communityId: number): Promise<void> {
    graphLoading.value = true;
    graphError.value = null;

    try {
      const response = await fetch(getDaemonUrl(`/api/graph/communities/${communityId}/concepts`));
      if (!response.ok) {
        throw new Error((await response.text()) || `Failed to load concepts (${response.status}).`);
      }

      const graph = await response.json() as ConceptGraph;
      const edgeCounts = new Map<string, number>();
      for (const edge of graph.edges) {
        edgeCounts.set(edge.source, (edgeCounts.get(edge.source) ?? 0) + 1);
        edgeCounts.set(edge.target, (edgeCounts.get(edge.target) ?? 0) + 1);
      }

      conceptGraph.value = graph;
      conceptNodes.value = graph.nodes
        .map((node) => ({
          name: node.name,
          occurrences: node.occurrences,
          edgeCount: edgeCounts.get(node.name) ?? 0,
        }))
        .sort((left, right) => {
          if (right.edgeCount !== left.edgeCount) {
            return right.edgeCount - left.edgeCount;
          }
          return left.name.localeCompare(right.name);
        });
      activeCommunityId.value = communityId;
    } catch (errorValue) {
      graphError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      conceptGraph.value = null;
      conceptNodes.value = [];
    } finally {
      graphLoading.value = false;
    }
  }

  async function focusCommunity(communityId: number): Promise<void> {
    await loadGraphConcepts(communityId);
  }

  function showCommunityGraph(): void {
    activeCommunityId.value = null;
    conceptGraph.value = null;
    conceptNodes.value = [];
    selectedConcept.value = null;
    conceptContextError.value = null;
  }

  function findBestAnchorUri(conceptName: string, files: string[]): string | null {
    const memoryUris = files.filter((file) => file.startsWith('memory://'));
    if (memoryUris.length === 0) {
      return null;
    }

    const conceptSlug = conceptName.toLowerCase().replace(/\s+/g, '-');
    const anchored = memoryUris.find((uri) => uri.toLowerCase().includes(conceptSlug));
    return anchored ?? memoryUris[0] ?? null;
  }

  async function openConcept(conceptName: string): Promise<void> {
    const normalized = normalizeConceptName(conceptName);
    if (!normalized) {
      return;
    }

    selectedConcept.value = normalized;
    conceptContextLoading.value = true;
    conceptContextError.value = null;

    try {
      const context = await memoryApi.buildContext(normalized, {
        depth: 1,
        maxEntities: 30,
      });

      if (typeof context.communityId === 'number') {
        activeCommunityId.value = context.communityId;
      }

      const files = [
        ...context.directRelations.flatMap((relation) => relation.files),
        ...context.expandedRelations.flatMap((relation) => relation.files),
      ];

      const exactResults = await memoryApi.searchMemories(normalized, { mode: 'FullText' });
      const exactMatch = exactResults.results.find(
        (result) => result.title?.toLowerCase() === normalized.toLowerCase(),
      );
      if (exactMatch?.uri) {
        await openNote(exactMatch.uri);
        return;
      }

      const anchorUri = findBestAnchorUri(normalized, files);
      if (anchorUri) {
        await openNote(anchorUri);
        return;
      }

      const fallbackSearch = await memoryApi.searchMemories(normalized, { pageSize: 1 });
      const firstResult = fallbackSearch.results[0];
      if (firstResult) {
        await openNote(firstResult.uri);
        return;
      }

      activeNote.value = null;
      activeUri.value = null;
      conceptContextError.value = `No anchor note found for ${normalized}`;
    } catch (errorValue) {
      conceptContextError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
    } finally {
      conceptContextLoading.value = false;
    }
  }

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
    loadGraphCommunities,
    focusCommunity,
    showCommunityGraph,
    openConcept,
  };
});
