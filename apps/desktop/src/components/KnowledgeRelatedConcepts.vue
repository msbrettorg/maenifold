<script setup lang="ts">
import { Loader2, Sparkles } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { useKnowledgeStore } from '../stores/knowledge';

const store = useKnowledgeStore();
const {
  relatedConcepts,
  relatedConceptsSource,
  relatedConceptsLoading,
  relatedConceptsError,
  conceptContextLoading,
  conceptContextError,
} = storeToRefs(store);
</script>

<template>
  <aside class="flex min-h-0 h-64 shrink-0 flex-col border-t border-border-subtle bg-bg-sidebar" aria-label="Related concepts">
    <div class="flex items-center gap-2 border-b border-border-subtle px-3 py-2">
      <Sparkles class="size-4 text-text-secondary" aria-hidden="true" />
      <p class="text-sm font-semibold text-text-primary">Related concepts</p>
    </div>
    <div class="border-b border-border-subtle px-3 py-2 text-xs text-text-tertiary">
      {{ relatedConceptsSource ? `Seed: ${relatedConceptsSource}` : 'Open a note to load nearby concepts.' }}
    </div>
    <ScrollArea class="min-h-0 flex-1 px-2 py-2">
      <div class="grid gap-2">
        <Alert v-if="relatedConceptsError" variant="destructive" class="border-border-subtle bg-bg-surface">
          <AlertTitle>Could not load related concepts</AlertTitle>
          <AlertDescription>{{ relatedConceptsError }}</AlertDescription>
        </Alert>
        <template v-else-if="relatedConceptsLoading">
          <div class="inline-flex items-center gap-2 px-2 py-1 text-sm text-text-secondary">
            <Loader2 class="size-4 animate-spin" aria-hidden="true" />
            Loading related concepts…
          </div>
          <Skeleton class="h-8 rounded-lg" />
          <Skeleton class="h-8 rounded-lg" />
        </template>
        <template v-else-if="relatedConcepts.length > 0">
          <Button
            v-for="concept in relatedConcepts"
            :key="concept.name"
            type="button"
            variant="ghost"
            class="h-auto justify-start rounded-lg px-2 py-2 text-left"
            @click="store.openConcept(concept.name)"
          >
            <div class="grid min-w-0 gap-1">
              <span class="truncate text-sm text-text-primary">{{ concept.name }}</span>
              <span class="text-xs text-text-tertiary">
                Similarity {{ concept.similarity.toFixed(3) }}
              </span>
            </div>
          </Button>
        </template>
        <p v-else class="px-2 py-3 text-sm text-text-secondary">
          No related concepts available for the selected note.
        </p>
      </div>
    </ScrollArea>
    <Alert v-if="conceptContextError" variant="destructive" class="m-2 border-border-subtle bg-bg-surface">
      <AlertTitle>Concept navigation failed</AlertTitle>
      <AlertDescription>{{ conceptContextError }}</AlertDescription>
    </Alert>
    <div v-else-if="conceptContextLoading" class="px-3 pb-3 text-xs text-text-secondary">
      Opening concept context…
    </div>
  </aside>
</template>
