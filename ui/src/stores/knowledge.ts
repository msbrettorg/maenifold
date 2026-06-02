import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { executeGateway, executeGatewayText, readGatewayResource } from '../lib/mcp/gateway';

const MEMORY_DOMAIN = 'maenifold.memory';
const DEFAULT_QUERY = 'maenifold';
const DEFAULT_CONCEPT = 'maenifold';
const STATS_URI = 'graph://stats';
const GRAPH_RELATION_LIMIT = 28;
const SEARCH_PAGE_SIZE = 32;
const ROOT_TREE_PATH = '';

export interface GraphStats {
  node_count?: number;
  edge_count?: number;
  community_count?: number;
  [key: string]: unknown;
}

export interface MemorySearchResult {
  title?: string;
  path?: string;
  decayWeight?: number;
  fusedScore?: number;
  snippet?: string;
  [key: string]: unknown;
}

interface SearchMemoriesResponse {
  results?: MemorySearchResult[];
  totalCount?: number;
  [key: string]: unknown;
}

export interface ContextRelation {
  name?: string;
  coOccurrenceCount?: number;
  decayWeight?: number;
  weightedScore?: number;
  communityId?: number | null;
  sourceFileCount?: number;
  files?: string[];
  [key: string]: unknown;
}

interface ConceptNeighborsResponse {
  concept?: string;
  communityId?: number | null;
  limit?: number;
  cursor?: string;
  nextCursor?: string | null;
  totalCount?: number;
  neighbors?: ContextRelation[];
  [key: string]: unknown;
}

export interface MemoryFile {
  uri?: string;
  title?: string;
  location?: string;
  modified?: string;
  checksum?: string;
  content?: string;
  [key: string]: unknown;
}

export interface MemoryTreeNode {
  id: string;
  kind: 'folder' | 'file';
  name: string;
  path: string;
  uri?: string;
  fileCount?: number;
  loaded?: boolean;
  loading?: boolean;
  children: MemoryTreeNode[];
}

interface DirectoryListing {
  folders: Array<{ name: string; path: string; fileCount: number }>;
  files: Array<{ name: string; path: string; uri: string }>;
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function cleanQuery(value: string): string {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : DEFAULT_QUERY;
}

function cleanConcept(value: string): string {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : DEFAULT_CONCEPT;
}

function relationName(relation: ContextRelation): string {
  return relation.name ?? 'unknown-concept';
}

function createRootTree(): MemoryTreeNode {
  return {
    id: 'folder:',
    kind: 'folder',
    name: 'memory://',
    path: ROOT_TREE_PATH,
    children: [],
    loaded: false,
  };
}

function joinMemoryPath(parent: string, child: string): string {
  const cleanChild = child.replace(/^\/+|\/+$/g, '');
  return parent ? `${parent}/${cleanChild}` : cleanChild;
}

function uriFromFilePath(filePath: string): string {
  return `memory://${filePath.replace(/\.md$/i, '')}`;
}

function parseListMemories(text: string, folderPath: string): DirectoryListing {
  const listing: DirectoryListing = { folders: [], files: [] };
  let section: 'folders' | 'files' | null = null;

  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;
    if (line === '### Folders') {
      section = 'folders';
      continue;
    }
    if (line === '### Files') {
      section = 'files';
      continue;
    }
    if (line.startsWith('#') || line.includes(':')) continue;

    const cleanLine = line.replace(/^[^A-Za-z0-9._/-]+/, '').trim();
    if (section === 'folders') {
      const match = cleanLine.match(/^(.+\/)\s+\((\d+)\s+files\)$/);
      if (!match) continue;
      const name = match[1].replace(/\/$/, '');
      listing.folders.push({
        name,
        path: joinMemoryPath(folderPath, name),
        fileCount: Number(match[2]),
      });
    }

    if (section === 'files') {
      const match = cleanLine.match(/^(.+?\.md)\s+\(([^)]+)\)$/);
      if (!match) continue;
      const fileName = match[1];
      const path = joinMemoryPath(folderPath, fileName);
      listing.files.push({
        name: fileName.replace(/\.md$/i, ''),
        path,
        uri: uriFromFilePath(path),
      });
    }
  }

  return listing;
}

function findTreeNode(root: MemoryTreeNode, path: string): MemoryTreeNode | null {
  if (root.path === path) return root;
  for (const child of root.children) {
    if (child.kind === 'folder') {
      const found = findTreeNode(child, path);
      if (found) return found;
    }
  }
  return null;
}

function conceptFromResult(result: MemorySearchResult): string {
  const title = result.title ?? result.path ?? DEFAULT_CONCEPT;
  return title
    .replace(/^memory:\/\//, '')
    .replace(/\.[a-z0-9]+$/i, '')
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .replace(/[^a-zA-Z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .toLowerCase() || DEFAULT_CONCEPT;
}

export const useKnowledgeStore = defineStore('knowledge', () => {
  const query = ref(DEFAULT_QUERY);
  const selectedConcept = ref(DEFAULT_CONCEPT);
  const stats = ref<GraphStats | null>(null);
  const search = ref<SearchMemoriesResponse | null>(null);
  const selectedMemory = ref<MemoryFile | null>(null);
  const selectedSearchResult = ref<MemorySearchResult | null>(null);
  const neighborPage = ref<ConceptNeighborsResponse | null>(null);
  const memoryTree = ref<MemoryTreeNode>(createRootTree());
  const expandedTreePaths = ref<string[]>([ROOT_TREE_PATH]);
  const bootstrapped = ref(false);
  const loading = ref(false);
  const graphLoading = ref(false);
  const viewerLoading = ref(false);
  const treeLoading = ref(false);
  const error = ref<string | null>(null);

  const searchResults = computed(() => search.value?.results ?? []);
  const relations = computed(() => neighborPage.value?.neighbors ?? []);
  const visibleRelations = computed(() => relations.value.slice(0, GRAPH_RELATION_LIMIT));
  const relationTotal = computed(() => neighborPage.value?.totalCount ?? relations.value.length);
  const graphOverflow = computed(() => Math.max(0, relationTotal.value - visibleRelations.value.length));
  const treeFileCount = computed(() => countTreeFiles(memoryTree.value));

  function countTreeFiles(node: MemoryTreeNode): number {
    if (node.kind === 'file') return 1;
    return node.children.reduce((total, child) => total + countTreeFiles(child), 0);
  }

  async function loadTreeFolder(path: string): Promise<void> {
    const node = findTreeNode(memoryTree.value, path);
    if (!node || node.kind !== 'folder' || node.loaded || node.loading) return;

    node.loading = true;
    try {
      const text = await executeGatewayText(
        MEMORY_DOMAIN,
        'ListMemories',
        path ? { path } : {}
      );
      const listing = parseListMemories(text, path);
      node.children = [
        ...listing.folders.map<MemoryTreeNode>((folder) => ({
          id: `folder:${folder.path}`,
          kind: 'folder',
          name: folder.name,
          path: folder.path,
          fileCount: folder.fileCount,
          children: [],
          loaded: false,
        })),
        ...listing.files.map<MemoryTreeNode>((file) => ({
          id: `file:${file.uri}`,
          kind: 'file',
          name: file.name,
          path: file.path,
          uri: file.uri,
          children: [],
          loaded: true,
        })),
      ];
      node.loaded = true;
      error.value = null;
    } catch (nextError) {
      error.value = toErrorMessage(nextError);
      throw nextError;
    } finally {
      node.loading = false;
    }
  }

  function expandTreePath(path: string): void {
    if (!expandedTreePaths.value.includes(path)) {
      expandedTreePaths.value = [...expandedTreePaths.value, path];
    }
  }

  async function expandFolder(path: string): Promise<void> {
    expandTreePath(path);
    await loadTreeFolder(path);
  }

  async function toggleTreeFolder(path: string): Promise<void> {
    if (expandedTreePaths.value.includes(path)) {
      expandedTreePaths.value = expandedTreePaths.value.filter((entry) => entry !== path);
      return;
    }
    await expandFolder(path);
  }

  async function expandToMemoryUri(uri: string | undefined): Promise<void> {
    const location = uri?.replace(/^memory:\/\//, '');
    if (!location) return;
    const parts = location.split('/').filter(Boolean);
    const folders = parts.slice(0, -1);
    let currentPath = ROOT_TREE_PATH;

    await expandFolder(currentPath);
    for (const folder of folders) {
      currentPath = joinMemoryPath(currentPath, folder);
      await expandFolder(currentPath);
    }
  }

  async function refreshTree(): Promise<void> {
    treeLoading.value = true;
    try {
      memoryTree.value = createRootTree();
      expandedTreePaths.value = [ROOT_TREE_PATH];
      await loadTreeFolder(ROOT_TREE_PATH);
      const uri = selectedMemory.value?.uri ?? selectedSearchResult.value?.path;
      if (uri) await expandToMemoryUri(uri);
    } finally {
      treeLoading.value = false;
    }
  }

  async function refreshStats(): Promise<void> {
    stats.value = await readGatewayResource<GraphStats>(STATS_URI);
  }

  async function loadConcept(conceptName: string): Promise<void> {
    const nextConcept = cleanConcept(conceptName);
    selectedConcept.value = nextConcept;
    graphLoading.value = true;
    try {
      neighborPage.value = await readGatewayResource<ConceptNeighborsResponse>(
        `graph://concept/${encodeURIComponent(nextConcept)}/neighbors?limit=${GRAPH_RELATION_LIMIT}`
      );
      error.value = null;
    } catch (nextError) {
      neighborPage.value = {
        concept: nextConcept,
        totalCount: 0,
        neighbors: [],
      };
      error.value = toErrorMessage(nextError);
    } finally {
      graphLoading.value = false;
    }
  }

  async function readMemoryByIdentifier(
    identifier: string,
    sourceResult?: MemorySearchResult,
    graphConcept?: string
  ): Promise<void> {
    viewerLoading.value = true;
    try {
      const memory = await executeGateway<MemoryFile>(MEMORY_DOMAIN, 'ReadMemory', {
        identifier,
        includeChecksum: true,
      });
      selectedMemory.value = memory;
      selectedSearchResult.value = sourceResult ?? {
        title: memory.title,
        path: memory.uri,
      };
      await expandToMemoryUri(memory.uri);
      const concept = graphConcept ?? selectedConcept.value ?? conceptFromResult(selectedSearchResult.value);
      await loadConcept(concept);
      error.value = null;
    } catch (nextError) {
      error.value = toErrorMessage(nextError);
      throw nextError;
    } finally {
      viewerLoading.value = false;
    }
  }

  async function readMemory(result: MemorySearchResult, graphConcept?: string): Promise<void> {
    const identifier = result.path;
    if (!identifier) return;
    await readMemoryByIdentifier(identifier, result, graphConcept);
  }

  async function runSearch(nextQuery = query.value, openFirst = false): Promise<void> {
    const cleanedQuery = cleanQuery(nextQuery);
    query.value = cleanedQuery;
    loading.value = true;
    try {
      const nextSearch = await executeGateway<SearchMemoriesResponse>(MEMORY_DOMAIN, 'SearchMemories', {
        query: cleanedQuery,
        mode: 'Hybrid',
        page: 1,
        pageSize: SEARCH_PAGE_SIZE,
        semanticWeight: 1,
        textWeight: 1,
        rrfK: 30,
      });
      search.value = nextSearch;
      if (openFirst) {
        const firstResult = nextSearch.results?.find((result) => result.path);
        if (firstResult) await readMemory(firstResult, cleanedQuery);
      }
      error.value = null;
    } catch (nextError) {
      error.value = toErrorMessage(nextError);
      throw nextError;
    } finally {
      loading.value = false;
    }
  }

  async function openConcept(conceptName: string): Promise<void> {
    const nextConcept = cleanConcept(conceptName);
    query.value = nextConcept;
    await Promise.all([runSearch(nextConcept, false), loadConcept(nextConcept)]);
  }

  async function openRelation(relation: ContextRelation): Promise<void> {
    const files = relation.files ?? [];
    if (files[0]) {
      await readMemoryByIdentifier(files[0], {
        title: files[0].replace(/^memory:\/\//, ''),
        path: files[0],
      }, relationName(relation));
      return;
    }
    await openConcept(relationName(relation));
  }

  async function refreshAll(): Promise<void> {
    loading.value = true;
    try {
      await Promise.all([refreshStats(), refreshTree()]);
      await runSearch(query.value, false);
      const firstResult = searchResults.value.find((result) => result.path);
      if (!selectedMemory.value && firstResult) {
        await readMemory(firstResult);
      } else {
        await loadConcept(selectedConcept.value);
      }
      bootstrapped.value = true;
    } finally {
      loading.value = false;
    }
  }

  async function bootstrap(): Promise<void> {
    if (bootstrapped.value) return;
    await refreshAll();
  }

  return {
    query,
    selectedConcept,
    stats,
    search,
    selectedMemory,
    selectedSearchResult,
    neighborPage,
    memoryTree,
    expandedTreePaths,
    bootstrapped,
    loading,
    graphLoading,
    viewerLoading,
    treeLoading,
    error,
    searchResults,
    relations,
    visibleRelations,
    relationTotal,
    graphOverflow,
    treeFileCount,
    refreshStats,
    refreshTree,
    toggleTreeFolder,
    runSearch,
    loadConcept,
    openConcept,
    openRelation,
    readMemory,
    refreshAll,
    bootstrap,
  };
});
