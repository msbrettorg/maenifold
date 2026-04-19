<script setup lang="ts">
import { Search, RefreshCw, FileText } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { useKnowledgeStore } from '../stores/knowledge';
import KnowledgeSidebarNode from './KnowledgeSidebarNode.vue';

const store = useKnowledgeStore();
const {
  folders,
  rootFiles,
  searchResults,
  activeUri,
  foldersLoading,
  foldersError,
  searchLoading,
  searchError,
  searchQuery,
} = storeToRefs(store);
</script>

<template>
  <aside class="flex h-full min-w-0 flex-col border-r border-border-subtle bg-bg-sidebar" aria-label="Knowledge sidebar">
    <div class="flex items-center justify-between gap-2 border-b border-border-subtle px-3 py-3">
      <div class="inline-flex items-center gap-2">
        <Search class="size-4 text-text-secondary" aria-hidden="true" />
        <h2 class="text-[var(--text-body-sm)] font-semibold text-text-primary">Notes</h2>
      </div>
      <Button
        type="button"
        size="icon-sm"
        variant="ghost"
        class="text-text-secondary hover:text-text-primary"
        title="Refresh folders"
        @click="store.loadRootFolders()"
      >
        <RefreshCw class="size-4" aria-hidden="true" />
      </Button>
    </div>

    <ScrollArea class="min-h-0 flex-1 px-2 py-2">
      <div class="grid gap-2">
        <Alert v-if="foldersError" variant="destructive" class="border-border-subtle bg-bg-surface">
          <AlertTitle>Could not load folders</AlertTitle>
          <AlertDescription>{{ foldersError }}</AlertDescription>
        </Alert>

        <template v-else-if="foldersLoading">
          <Skeleton class="h-8 rounded-[var(--radius-8)]" />
          <Skeleton class="h-8 rounded-[var(--radius-8)]" />
          <Skeleton class="h-8 rounded-[var(--radius-8)]" />
          <Skeleton class="h-8 rounded-[var(--radius-8)]" />
        </template>

        <template v-else>
          <Button
            v-for="file in rootFiles"
            :key="file.uri"
            type="button"
            variant="ghost"
            class="h-8 justify-start gap-2 rounded-[var(--radius-8)] px-2"
            :class="activeUri === file.uri ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
            @click="store.openNote(file.uri)"
          >
            <FileText class="size-4 shrink-0" aria-hidden="true" />
            <span class="truncate text-[var(--text-body-sm)] font-medium">{{ file.name }}</span>
          </Button>

          <KnowledgeSidebarNode
            v-for="folder in folders"
            :key="folder.path"
            :node="folder"
            :active-uri="activeUri"
            @open-folder="({ path, open }) => store.setFolderOpen(path, open)"
            @open-note="store.openNote"
          />

          <p v-if="!rootFiles.length && !folders.length" class="px-2 py-3 text-[var(--text-body-sm)] text-text-secondary">
            No memory folders were found.
          </p>
        </template>
      </div>

      <div
        v-if="searchQuery.trim().length > 0 || searchLoading || searchError || searchResults.length > 0"
        class="mt-4 grid gap-2 border-t border-border-subtle pt-3"
      >
        <h3 class="px-2 text-[var(--text-tiny)] uppercase tracking-[0.04em] text-text-tertiary">Search results</h3>

        <Alert v-if="searchError" variant="destructive" class="border-border-subtle bg-bg-surface">
          <AlertDescription>{{ searchError }}</AlertDescription>
        </Alert>

        <template v-else-if="searchLoading">
          <Skeleton class="h-9 rounded-[var(--radius-8)]" />
          <Skeleton class="h-9 rounded-[var(--radius-8)]" />
        </template>

        <template v-else-if="searchResults.length > 0">
          <Button
            v-for="result in searchResults"
            :key="result.uri"
            type="button"
            variant="ghost"
            class="h-auto justify-start rounded-[var(--radius-8)] px-2 py-2 text-left"
            :class="activeUri === result.uri ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
            @click="store.openNote(result.uri)"
          >
            <div class="grid min-w-0 gap-1">
              <span class="truncate text-[var(--text-body-sm)] font-medium">{{ result.title }}</span>
              <span class="line-clamp-2 text-[var(--text-tiny)] text-text-tertiary">{{ result.snippet || result.uri }}</span>
            </div>
          </Button>
        </template>

        <p v-else class="px-2 text-[var(--text-body-sm)] text-text-secondary">No notes matched this query.</p>
      </div>
    </ScrollArea>
  </aside>
</template>
