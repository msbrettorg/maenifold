<script setup lang="ts">
import { Loader2, PanelRightClose, PanelRightOpen, Sparkles } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { useKnowledgeStore } from '../stores/knowledge';

const store = useKnowledgeStore();
const props = withDefaults(defineProps<{
  rail?: boolean;
  openInMemory?: boolean;
}>(), {
  rail: false,
  openInMemory: false,
});
const collapsed = defineModel<boolean>('collapsed', { default: false });
const {
  relatedConcepts,
  relatedConceptsSource,
  relatedConceptsLoading,
  relatedConceptsError,
  conceptContextLoading,
  conceptContextError,
} = storeToRefs(store);

function openRelatedConcept(conceptName: string): void {
  if (props.openInMemory) {
    store.setActiveSubtab('memory');
  }
  void store.openConcept(conceptName);
}
</script>

<template>
  <aside
    :class="rail
      ? 'flex h-full min-h-0 w-full flex-col border-l border-border-subtle bg-bg-sidebar'
      : 'flex min-h-0 h-64 shrink-0 flex-col border-t border-border-subtle bg-bg-sidebar'"
    aria-label="Related concepts"
  >
    <div
      :class="collapsed
        ? 'flex items-center justify-center border-b border-border-subtle px-1 py-2'
        : 'flex items-center justify-between gap-2 border-b border-border-subtle px-3 py-2'"
    >
      <div v-if="!collapsed" class="flex min-w-0 items-center gap-2">
        <Sparkles class="size-4 text-text-secondary" aria-hidden="true" />
        <p class="text-sm font-semibold text-text-primary">Related concepts</p>
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        :title="collapsed ? 'Expand related concepts' : 'Collapse related concepts'"
        :aria-label="collapsed ? 'Expand related concepts' : 'Collapse related concepts'"
        @click="collapsed = !collapsed"
      >
        <PanelRightOpen v-if="collapsed" class="size-4" />
        <PanelRightClose v-else class="size-4" />
      </Button>
    </div>
    <div v-if="!collapsed" class="border-b border-border-subtle px-3 py-2 text-xs text-text-tertiary">
      {{ relatedConceptsSource ? `Seed: ${relatedConceptsSource}` : 'Open a note to load nearby concepts.' }}
    </div>
    <ScrollArea v-if="!collapsed" class="min-h-0 flex-1 px-2 py-2">
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
            @click="openRelatedConcept(concept.name)"
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
    <Alert
      v-if="!collapsed && conceptContextError"
      variant="destructive"
      class="m-2 border-border-subtle bg-bg-surface"
    >
      <AlertTitle>Concept navigation failed</AlertTitle>
      <AlertDescription>{{ conceptContextError }}</AlertDescription>
    </Alert>
    <div v-else-if="!collapsed && conceptContextLoading" class="px-3 pb-3 text-xs text-text-secondary">
      Opening concept context…
    </div>
  </aside>
</template>
