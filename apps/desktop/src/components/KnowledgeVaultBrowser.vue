<script setup lang="ts">
import { computed, ref } from 'vue';
import { FileText, FolderTree, RefreshCw, Search, X } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useKnowledgeStore } from '../stores/knowledge';
import KnowledgeRowActions from './KnowledgeRowActions.vue';
import KnowledgeVaultNode from './KnowledgeVaultNode.vue';

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
const mode = ref<'tree' | 'search'>('tree');
const hasSubmittedSearch = ref(false);
const showSearchResults = computed(() => mode.value === 'search');
const emptySearchMessage = computed(() => (
  hasSubmittedSearch.value && searchQuery.value.trim()
    ? 'No results'
    : 'Search the vault to show matching notes.'
));

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
          v-model="query"
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
            <div
              v-for="file in rootFiles"
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
            <KnowledgeVaultNode v-for="folder in folders" :key="folder.path" :node="folder" :active-tab-uri="activeTabUri" />
            <p v-if="!rootFiles.length && !folders.length" class="px-2 py-3 text-body-sm text-text-secondary">
              No memory folders were found.
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
          <template v-else-if="searchResults.length > 0">
            <Button
              v-for="result in searchResults"
              :key="result.uri"
              type="button"
              variant="ghost"
              class="h-auto justify-start rounded-md px-2 py-2 text-left"
              :class="activeTabUri === result.uri ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
              @click="openUri(result.uri, $event)"
            >
              <div class="grid min-w-0 gap-1">
                <span class="truncate text-body-sm font-medium">{{ result.title }}</span>
                <span class="line-clamp-2 text-tiny text-text-tertiary">{{ result.snippet || result.uri }}</span>
              </div>
            </Button>
          </template>
          <p v-else class="px-2 py-3 text-body-sm text-text-secondary">{{ emptySearchMessage }}</p>
        </template>
      </div>
    </ScrollArea>
  </aside>
</template>
