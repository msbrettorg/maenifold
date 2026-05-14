<script setup lang="ts">
import { Link, Loader2 } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { ref, watch } from 'vue';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { useMemoryApi } from '../composables/useMemoryApi';
import type { MemoryFile, MemorySearchItem } from '../lib/workflow-types';
import { useKnowledgeStore } from '../stores/knowledge';

const props = withDefaults(defineProps<{ rail?: boolean }>(), { rail: false });
const store = useKnowledgeStore();
const memoryApi = useMemoryApi();
const { activeTabUri, activeNote } = storeToRefs(store);

const backlinks = ref<MemorySearchItem[]>([]);
const backlinksLoading = ref(false);
const backlinksError = ref<string | null>(null);
let loadGeneration = 0;

function conceptFromActiveNote(uri: string | null, note: MemoryFile | null): string | null {
  const title = note?.title?.trim();
  if (title) return title;
  if (!uri) return null;
  return decodeURIComponent(uri.replace(/^memory:\/\//, '').split('/').at(-1) ?? uri).trim();
}

function isActiveResult(result: MemorySearchItem, activeUri: string | null): boolean {
  if (!activeUri) return false;
  return result.uri === activeUri || decodeURIComponent(result.uri) === decodeURIComponent(activeUri);
}

async function loadBacklinks(): Promise<void> {
  const generation = ++loadGeneration;
  const currentUri = activeTabUri.value;
  const concept = conceptFromActiveNote(currentUri, activeNote.value);
  backlinks.value = [];
  backlinksError.value = null;
  if (!concept) return;

  backlinksLoading.value = true;
  try {
    const result = await memoryApi.searchMemories(`[[${concept}]]`, {
      mode: 'FullText',
      pageSize: 30,
    });
    const rows = result.results.filter((item) => !isActiveResult(item, currentUri));
    if (generation === loadGeneration) backlinks.value = rows;
  } catch (errorValue) {
    if (generation === loadGeneration) {
      backlinksError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      backlinks.value = [];
    }
  } finally {
    if (generation === loadGeneration) backlinksLoading.value = false;
  }
}

function openBacklink(uri: string): void {
  store.openNote(uri);
}

watch(
  [activeTabUri, () => activeNote.value?.title ?? null, () => activeNote.value?.content ?? null],
  () => { void loadBacklinks(); },
  { immediate: true },
);
</script>

<template>
  <aside
    :class="props.rail
      ? 'flex h-full min-h-0 w-full flex-col border-l border-border-subtle bg-bg-sidebar'
      : 'flex min-h-0 h-64 shrink-0 flex-col border-t border-border-subtle bg-bg-sidebar'"
    aria-label="Linked from"
  >
    <div class="flex items-center gap-2 border-b border-border-subtle px-3 py-2">
      <Link class="size-4 text-text-secondary" aria-hidden="true" />
      <p class="text-sm font-semibold text-text-primary">Linked from</p>
    </div>
    <ScrollArea class="min-h-0 flex-1 px-2 py-2" data-testid="backlinks-scroll-area">
      <div class="grid gap-2">
        <Alert v-if="backlinksError" variant="destructive" class="border-border-subtle bg-bg-surface">
          <AlertTitle>Could not load backlinks</AlertTitle>
          <AlertDescription>{{ backlinksError }}</AlertDescription>
        </Alert>
        <template v-else-if="backlinksLoading">
          <div class="inline-flex items-center gap-2 px-2 py-1 text-sm text-text-secondary">
            <Loader2 class="size-4 animate-spin" aria-hidden="true" />
            Loading backlinks…
          </div>
          <Skeleton class="h-12 rounded-lg" />
          <Skeleton class="h-12 rounded-lg" />
        </template>
        <template v-else-if="backlinks.length > 0">
          <Button
            v-for="backlink in backlinks"
            :key="backlink.uri"
            type="button"
            variant="ghost"
            class="h-auto justify-start rounded-lg px-2 py-2 text-left"
            data-testid="backlink-row"
            @click="openBacklink(backlink.uri)"
          >
            <div class="grid min-w-0 gap-1">
              <span class="truncate text-sm text-text-primary">{{ backlink.title || backlink.uri }}</span>
              <span class="line-clamp-2 text-xs text-text-tertiary">{{ backlink.snippet || 'Contains a link to this note.' }}</span>
            </div>
          </Button>
        </template>
        <p v-else class="px-2 py-3 text-sm text-text-secondary">
          No notes link to this one yet.
        </p>
      </div>
    </ScrollArea>
  </aside>
</template>
