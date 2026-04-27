<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Search } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { useKnowledgeStore } from '../stores/knowledge';

const store = useKnowledgeStore();
const {
  graphLoading,
  conceptNodes,
  activeCommunityId,
  selectedConcept,
} = storeToRefs(store);
const conceptFilter = ref('');

const filteredConcepts = computed(() => {
  const query = conceptFilter.value.trim().toLowerCase();
  if (!query) return conceptNodes.value;
  return conceptNodes.value.filter((item) => item.name.toLowerCase().includes(query));
});

watch(activeCommunityId, () => {
  conceptFilter.value = '';
});
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col" aria-label="Concept list">
    <div class="border-b border-border-subtle px-3 py-2">
      <div class="relative">
        <Search class="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-text-tertiary" aria-hidden="true" />
        <Input
          v-model="conceptFilter"
          :disabled="activeCommunityId === null"
          class="h-8 border-border-subtle bg-bg-surface pl-8 text-sm text-text-primary placeholder:text-text-secondary disabled:opacity-60"
          placeholder="Filter concepts"
        />
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1 px-2 py-2">
      <div class="grid gap-1">
        <template v-if="graphLoading && activeCommunityId !== null">
          <Skeleton class="h-8 rounded-lg" />
          <Skeleton class="h-8 rounded-lg" />
          <Skeleton class="h-8 rounded-lg" />
        </template>
        <template v-else-if="activeCommunityId !== null">
          <Button
            v-for="concept in filteredConcepts"
            :key="concept.name"
            type="button"
            variant="ghost"
            class="h-8 justify-start gap-2 rounded-lg px-2 text-left"
            :class="selectedConcept === concept.name ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
            @click="store.openConcept(concept.name)"
          >
            <span class="truncate text-sm">{{ concept.name }}</span>
            <span class="ml-auto text-xs text-text-tertiary">{{ concept.edgeCount }}</span>
          </Button>
          <p v-if="filteredConcepts.length === 0" class="px-2 py-3 text-sm text-text-secondary">
            No concepts match this filter.
          </p>
        </template>
        <p v-else class="px-2 py-3 text-sm text-text-secondary">
          Select a community cluster in the graph to browse concepts.
        </p>
      </div>
    </ScrollArea>
  </section>
</template>
