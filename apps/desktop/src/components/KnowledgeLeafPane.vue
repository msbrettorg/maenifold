<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { BookOpenText, Clock, FileText, X } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { useMemoryApi } from '../composables/useMemoryApi';
import { useKnowledgeStore } from '../stores/knowledge';
import type { RecentActivityItem } from '../lib/workflow-types';
import EmptyState from './EmptyState.vue';
import KnowledgeNoteRenderer from './KnowledgeNoteRenderer.vue';

const store = useKnowledgeStore();
const memoryApi = useMemoryApi();
const { activeTabUri, activeNote, noteLoading, noteError } = storeToRefs(store);
const recentNotes = ref<RecentActivityItem[]>([]);
const recentLoading = ref(false);
const recentError = ref<string | null>(null);
const noteErrorTitle = computed(() => (
  /not\s*found|no such|missing|404/i.test(noteError.value ?? '')
    ? 'Note not found'
    : 'Could not open note'
));

async function loadRecentNotes(): Promise<void> {
  recentLoading.value = true;
  recentError.value = null;
  try {
    const result = await memoryApi.recentActivity({ filter: 'memory', limit: 5 });
    recentNotes.value = result.items;
  } catch (errorValue) {
    recentError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
    recentNotes.value = [];
  } finally {
    recentLoading.value = false;
  }
}

onMounted(() => {
  void loadRecentNotes();
});

function closeActiveTab(): void {
  if (!activeTabUri.value) return;
  store.closeTab(activeTabUri.value);
}

</script>

<template>
  <section class="flex min-h-0 min-w-0 flex-1 flex-col bg-bg-app" aria-label="Knowledge leaf">
    <header v-if="activeNote" class="border-b border-border-subtle px-5 py-3">
      <p class="text-tiny uppercase tracking-[0.04em] text-text-tertiary">Selected note</p>
      <h2 class="truncate text-title font-semibold text-text-primary">{{ activeNote.title }}</h2>
      <p v-if="activeTabUri" class="truncate text-body-sm text-text-secondary">{{ activeTabUri }}</p>
    </header>

    <ScrollArea class="min-h-0 flex-1">
      <div class="mx-auto grid min-w-0 max-w-5xl gap-3 p-5">
        <template v-if="noteLoading">
          <Skeleton class="h-5 w-3/5 rounded-md" />
          <Skeleton class="h-4 w-full rounded-md" />
          <Skeleton class="h-4 w-5/6 rounded-md" />
          <Skeleton class="h-4 w-11/12 rounded-md" />
          <Skeleton class="h-48 w-full rounded-md" />
        </template>

        <Alert v-else-if="noteError" variant="destructive" class="border-border-subtle bg-bg-surface">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <AlertTitle>{{ noteErrorTitle }}</AlertTitle>
              <AlertDescription>{{ noteError }}</AlertDescription>
            </div>
            <Button
              v-if="activeTabUri"
              type="button"
              size="icon-sm"
              variant="ghost"
              class="h-7 w-7 shrink-0 text-text-tertiary hover:bg-bg-surface-hover hover:text-text-primary"
              aria-label="Close tab"
              @click="closeActiveTab"
            >
              <X class="size-4" aria-hidden="true" />
            </Button>
          </div>
        </Alert>

        <div v-else-if="!activeNote" class="grid min-h-[24rem] place-items-center">
          <div class="grid w-full max-w-2xl gap-4">
            <EmptyState :icon="BookOpenText" title="Open a note" description="Choose a note from the vault or reopen a recent note." />
            <div class="rounded-[8px] border border-border-card bg-bg-surface p-3">
              <div class="mb-2 inline-flex items-center gap-2 text-body-sm font-semibold text-text-primary">
                <Clock class="size-4 text-text-secondary" aria-hidden="true" />
                Recent notes
              </div>
              <Alert v-if="recentError" variant="destructive" class="border-border-subtle bg-bg-app">
                <AlertDescription>{{ recentError }}</AlertDescription>
              </Alert>
              <div v-else-if="recentLoading" class="grid gap-2">
                <Skeleton v-for="index in 3" :key="index" class="h-9 rounded-md" />
              </div>
              <div v-else-if="recentNotes.length > 0" class="grid gap-1">
                <Button
                  v-for="note in recentNotes"
                  :key="note.uri"
                  type="button"
                  variant="ghost"
                  class="h-auto justify-start rounded-md px-2 py-2 text-left text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary"
                  @click="store.openInNewTab(note.uri)"
                >
                  <div class="grid min-w-0 gap-1">
                    <span class="truncate text-body-sm font-medium">{{ note.title }}</span>
                    <span class="truncate text-tiny text-text-tertiary">{{ note.uri }}</span>
                  </div>
                </Button>
              </div>
              <p v-else class="text-body-sm text-text-secondary">No recent notes were reported.</p>
            </div>
          </div>
        </div>

        <div v-else-if="!activeNote.content?.trim()" class="flex min-h-[18rem] items-center justify-center">
          <EmptyState :icon="FileText" title="Empty note" description="This note has no content yet." />
        </div>

        <article v-else class="bg-bg-surface min-w-0 max-w-full rounded-[8px] p-4">
          <KnowledgeNoteRenderer :content="activeNote.content" />
        </article>
      </div>
    </ScrollArea>
  </section>
</template>
