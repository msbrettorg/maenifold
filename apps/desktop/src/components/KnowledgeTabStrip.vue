<script setup lang="ts">
import { FileText, X } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useKnowledgeStore } from '../stores/knowledge';

const store = useKnowledgeStore();
const { openTabs, activeTabUri, tabCache } = storeToRefs(store);

function tabTitle(uri: string): string {
  return tabCache.value.get(uri)?.title ?? uri.replace(/^memory:\/\//, '').split('/').at(-1) ?? uri;
}

function closeTab(uri: string, event: MouseEvent): void {
  event.stopPropagation();
  store.closeTab(uri);
}
</script>

<template>
  <div class="flex h-11 shrink-0 items-center border-b border-border-subtle bg-bg-sidebar" aria-label="Open knowledge tabs">
    <ScrollArea class="min-w-0 flex-1">
      <div class="flex min-w-max items-center gap-1 px-2 py-1">
        <div
          v-for="uri in openTabs"
          :key="uri"
          class="inline-flex h-9 max-w-64 items-center rounded-md"
          :class="activeTabUri === uri ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
        >
          <Button
            type="button"
            variant="ghost"
            class="h-9 min-w-0 flex-1 justify-start gap-2 rounded-r-none px-2 text-current hover:bg-transparent hover:text-current"
            @click="store.setActiveTab(uri)"
          >
            <FileText class="size-4 shrink-0" aria-hidden="true" />
            <span class="truncate text-body-sm font-medium">{{ tabTitle(uri) }}</span>
          </Button>
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            class="mr-1 h-6 w-6 text-text-tertiary hover:bg-bg-surface-hover hover:text-text-primary"
            aria-label="Close tab"
            @click="closeTab(uri, $event)"
          >
            <X class="size-3" aria-hidden="true" />
          </Button>
        </div>
        <p v-if="openTabs.length === 0" class="px-2 text-body-sm text-text-tertiary">
          No notes open
        </p>
      </div>
    </ScrollArea>
  </div>
</template>
