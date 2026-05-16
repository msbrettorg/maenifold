<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { FileText, FolderTree, RefreshCw, Search, X } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useKnowledgeStore, type KnowledgeFileEntry, type KnowledgeFolderNode } from '../stores/knowledge';
import KnowledgeRowActions from './KnowledgeRowActions.vue';
import KnowledgeVaultNode from './KnowledgeVaultNode.vue';
type TreeSortMode = 'name-asc' | 'name-desc' | 'modified-newest';
type SearchSortMode = 'relevance' | 'name' | 'modified';
type SortableEntry = { name?: string; title?: string; modifiedAt?: string };
const store = useKnowledgeStore();
const {
  folders,
  rootFiles,
  searchQuery,
  searchResults,
  activeTabUri,
  foldersLoading,
  foldersError,
  searchLoading,
  searchError,
} = storeToRefs(store);
const query = ref(store.searchQuery);
const searchInput = ref<{ $el?: HTMLInputElement } | null>(null);
const treeQuery = ref('');
const mode = ref<'tree' | 'search'>('tree');
const treeSortMode = ref<TreeSortMode>('name-asc');
const searchSortMode = ref<SearchSortMode>('relevance');
const hasSubmittedSearch = ref(false);
const showSearchResults = computed(() => mode.value === 'search');
const normalizedTreeQuery = computed(() => treeQuery.value.trim().toLocaleLowerCase());
const isTreeFilterActive = computed(() => normalizedTreeQuery.value.length > 0);
const visibleRootFiles = computed(() => filterFiles(rootFiles.value, normalizedTreeQuery.value));
const visibleFolders = computed(() => filterFolders(folders.value, normalizedTreeQuery.value));
const currentSortMode = computed({
  get: (): TreeSortMode | SearchSortMode => (mode.value === 'tree' ? treeSortMode.value : searchSortMode.value),
  set: (value) => {
    if (mode.value === 'tree') {
      treeSortMode.value = value as TreeSortMode;
      return;
    }
    searchSortMode.value = value as SearchSortMode;
  },
});
const emptySearchMessage = computed(() => (
  hasSubmittedSearch.value && searchQuery.value.trim()
    ? 'No results'
    : 'Search the vault to show matching notes.'
));
let treeFilterTimer: number | null = null;

function entryMatches(values: string[], normalizedQuery: string): boolean {
  return values.some((value) => value.toLocaleLowerCase().includes(normalizedQuery));
}

function fileMatches(file: KnowledgeFileEntry, normalizedQuery: string): boolean {
  return entryMatches([file.name, file.path, file.uri], normalizedQuery);
}

function folderMatches(folder: KnowledgeFolderNode, normalizedQuery: string): boolean {
  return entryMatches([folder.name, folder.path], normalizedQuery);
}

function filterFiles(files: KnowledgeFileEntry[], normalizedQuery: string): KnowledgeFileEntry[] {
  if (!normalizedQuery) return files;
  return files.filter((file) => fileMatches(file, normalizedQuery));
}

function filterFolders(
  folderNodes: KnowledgeFolderNode[],
  normalizedQuery: string,
): KnowledgeFolderNode[] {
  if (!normalizedQuery) return folderNodes;

  return folderNodes
    .map((folder) => {
      const filteredFiles = filterFiles(folder.files, normalizedQuery);
      const filteredChildren = filterFolders(folder.children, normalizedQuery);
      const matches = folderMatches(folder, normalizedQuery);
      if (!matches && filteredFiles.length === 0 && filteredChildren.length === 0) {
        return null;
      }
      return { ...folder, files: filteredFiles, children: filteredChildren };
    })
    .filter((folder): folder is KnowledgeFolderNode => folder !== null);
}

function getLabel(entry: SortableEntry): string {
  return entry.title ?? entry.name ?? '';
}

function getModifiedTime(entry: SortableEntry): number {
  return Date.parse(entry.modifiedAt ?? '') || 0;
}

function compareByName(left: SortableEntry, right: SortableEntry, descending = false): number {
  const order = getLabel(left).localeCompare(getLabel(right));
  return descending ? -order : order;
}

function compareByModified(left: SortableEntry, right: SortableEntry): number {
  return getModifiedTime(right) - getModifiedTime(left) || compareByName(left, right);
}

const sortedRootFiles = computed(() => {
  const files = [...visibleRootFiles.value];
  if (treeSortMode.value === 'modified-newest') return files.sort(compareByModified);
  return files.sort((left, right) => compareByName(left, right, treeSortMode.value === 'name-desc'));
});

const sortedFolders = computed(() => {
  const foldersList = [...visibleFolders.value];
  if (treeSortMode.value === 'modified-newest') return foldersList.sort(compareByModified);
  return foldersList.sort((left, right) => compareByName(left, right, treeSortMode.value === 'name-desc'));
});

const sortedSearchResults = computed(() => {
  const results = [...searchResults.value];
  if (searchSortMode.value === 'name') return results.sort((left, right) => compareByName(left, right));
  if (searchSortMode.value === 'modified') return results.sort(compareByModified);
  return results.sort((left, right) => (right.score ?? 0) - (left.score ?? 0));
});

watch(query, (nextQuery) => {
  if (treeFilterTimer) {
    window.clearTimeout(treeFilterTimer);
  }
  treeFilterTimer = window.setTimeout(() => {
    treeQuery.value = nextQuery;
    treeFilterTimer = null;
  }, 75);
});

onBeforeUnmount(() => {
  if (treeFilterTimer) {
    window.clearTimeout(treeFilterTimer);
  }
});

async function runSearch(): Promise<void> {
  await store.search(query.value);
  hasSubmittedSearch.value = Boolean(query.value.trim());
  mode.value = hasSubmittedSearch.value ? 'search' : 'tree';
}

async function clearSearch(): Promise<void> {
  query.value = '';
  await store.search('');
  hasSubmittedSearch.value = false;
  mode.value = 'tree';
}

function openUri(uri: string, event: MouseEvent): void {
  if (event.metaKey || event.ctrlKey) {
    store.openInNewTab(uri);
    return;
  }
  store.openNote(uri);
}

function focusSearchInput(): void {
  searchInput.value?.$el?.focus();
}

defineExpose({ focusSearchInput });
</script>

<template>
  <aside class="flex h-full min-w-0 flex-col border-r border-border-subtle bg-bg-sidebar" aria-label="Knowledge vault">
    <div class="grid gap-3 border-b border-border-subtle px-3 py-3">
      <div class="flex items-center justify-between gap-2">
        <div class="inline-flex items-center gap-2">
          <FolderTree class="size-4 text-text-secondary" aria-hidden="true" />
          <h2 class="text-body-sm font-semibold text-text-primary">Vault</h2>
        </div>
        <Button type="button" size="icon-sm" variant="ghost" class="text-text-secondary hover:text-text-primary" @click="store.loadRootFolders()">
          <RefreshCw class="size-4" aria-hidden="true" />
        </Button>
      </div>

      <div class="relative">
        <Search class="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-text-tertiary" aria-hidden="true" />
        <Input
          ref="searchInput"
          v-model="query"
          data-testid="knowledge-memory-search"
          class="h-9 border-border-subtle bg-bg-surface pl-8 pr-8 text-text-primary placeholder:text-text-secondary"
          placeholder="Search notes..."
          @keydown.enter.prevent="runSearch"
        />
        <Button
          v-if="query.trim()"
          type="button"
          size="icon-sm"
          variant="ghost"
          class="absolute top-1/2 right-1 h-7 w-7 -translate-y-1/2 text-text-tertiary hover:text-text-primary"
          @click="clearSearch"
        >
          <X class="size-4" aria-hidden="true" />
        </Button>
      </div>

      <Tabs :model-value="mode" @update:model-value="mode = ($event as 'tree' | 'search')">
        <TabsList variant="line" class="w-full bg-transparent p-0">
          <TabsTrigger value="tree" class="h-8 rounded-md px-2 py-1">Tree</TabsTrigger>
          <TabsTrigger value="search" class="h-8 rounded-md px-2 py-1">Search</TabsTrigger>
        </TabsList>
      </Tabs>

      <select
        v-model="currentSortMode"
        data-testid="vault-sort-select"
        class="h-9 rounded-md border border-border-subtle bg-bg-surface px-3 text-body-sm text-text-primary"
      >
        <template v-if="mode === 'tree'">
          <option value="name-asc">Name (A-Z)</option>
          <option value="name-desc">Name (Z-A)</option>
          <option value="modified-newest">Modified (newest)</option>
        </template>
        <template v-else>
          <option value="relevance">Relevance</option>
          <option value="name">Name</option>
          <option value="modified">Modified</option>
        </template>
      </select>
    </div>

    <ScrollArea class="min-h-0 flex-1 px-2 py-2">
      <div class="grid gap-2">
        <template v-if="!showSearchResults">
          <Alert v-if="foldersError" variant="destructive" class="border-border-subtle bg-bg-surface">
            <AlertTitle>Could not load folders</AlertTitle>
            <AlertDescription>{{ foldersError }}</AlertDescription>
          </Alert>
          <template v-else-if="foldersLoading">
            <Skeleton v-for="index in 4" :key="index" class="h-8 rounded-md" />
          </template>
          <template v-else>
            <KnowledgeVaultNode
              v-for="folder in sortedFolders"
              :key="folder.path"
              :node="folder"
              :active-tab-uri="activeTabUri"
              :filter-active="isTreeFilterActive"
            />
            <div
              v-for="file in sortedRootFiles"
              :key="file.uri"
              class="flex h-8 items-center gap-1 rounded-md"
              :class="activeTabUri === file.uri ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
            >
              <Button type="button" variant="ghost" class="min-w-0 flex-1 justify-start gap-2 px-2" @click="openUri(file.uri, $event)">
                <FileText class="size-4 shrink-0" aria-hidden="true" />
                <span class="truncate text-body-sm font-medium">{{ file.name }}</span>
              </Button>
              <KnowledgeRowActions :file="file" />
            </div>
            <p v-if="!sortedRootFiles.length && !sortedFolders.length" class="px-2 py-3 text-body-sm text-text-secondary">
              {{ isTreeFilterActive ? 'No matching notes or folders.' : 'No memory folders were found.' }}
            </p>
          </template>
        </template>

        <template v-else>
          <Alert v-if="searchError" variant="destructive" class="border-border-subtle bg-bg-surface">
            <AlertDescription>{{ searchError }}</AlertDescription>
          </Alert>
          <template v-else-if="searchLoading">
            <Skeleton v-for="index in 3" :key="index" class="h-10 rounded-md" />
          </template>
          <template v-else-if="sortedSearchResults.length > 0">
            <div
              v-for="result in sortedSearchResults"
              :key="result.uri"
              class="grid min-w-0 gap-1"
            >
              <Button
                type="button"
                variant="ghost"
                class="h-auto justify-start rounded-md px-2 py-2 text-left"
                :class="activeTabUri === result.uri ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
                @click="openUri(result.uri, $event)"
              >
                <span class="truncate text-body-sm font-medium">{{ result.title }}</span>
              </Button>
              <span class="px-2 text-tiny text-text-tertiary">{{ result.snippet || result.uri }}</span>
            </div>
          </template>
          <p v-else class="px-2 py-3 text-body-sm text-text-secondary">{{ emptySearchMessage }}</p>
        </template>
      </div>
    </ScrollArea>
  </aside>
</template>
