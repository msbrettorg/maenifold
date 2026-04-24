<script setup lang="ts">
import { BookOpenText } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import KnowledgeNoteRenderer from './KnowledgeNoteRenderer.vue';
import EmptyState from './EmptyState.vue';
import { useKnowledgeStore } from '../stores/knowledge';

const store = useKnowledgeStore();
const { activeUri, activeNote, noteLoading, noteError } = storeToRefs(store);
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col bg-bg-app" aria-label="Knowledge content">
    <header v-if="activeNote" class="border-b border-border-subtle px-5 py-3">
      <p class="text-tiny uppercase tracking-[0.04em] text-text-tertiary">Selected note</p>
      <h2 class="truncate text-title font-semibold text-text-primary">
        {{ activeNote.title }}
      </h2>
      <p v-if="activeUri" class="truncate text-body-sm text-text-secondary">{{ activeUri }}</p>
    </header>

    <ScrollArea class="min-h-0 flex-1">
      <div class="mx-auto grid max-w-5xl gap-3 p-5">
        <template v-if="noteLoading">
          <Skeleton class="h-5 w-3/5 rounded-md" />
          <Skeleton class="h-4 w-full rounded-md" />
          <Skeleton class="h-4 w-5/6 rounded-md" />
          <Skeleton class="h-4 w-11/12 rounded-md" />
          <Skeleton class="h-48 w-full rounded-md" />
        </template>

        <Alert v-else-if="noteError" variant="destructive" class="border-border-subtle bg-bg-surface">
          <AlertTitle>Could not open note</AlertTitle>
          <AlertDescription>{{ noteError }}</AlertDescription>
        </Alert>

        <div
          v-else-if="!activeNote"
          class="flex min-h-[18rem] items-center justify-center"
        >
          <EmptyState :icon="BookOpenText" title="Select a note" description="Choose a note from the tree to read its content." />
        </div>

        <article
          v-else
          class="bg-bg-surface rounded-[8px] p-4"
        >
          <KnowledgeNoteRenderer :content="activeNote.content" />
        </article>
      </div>
    </ScrollArea>
  </section>
</template>
