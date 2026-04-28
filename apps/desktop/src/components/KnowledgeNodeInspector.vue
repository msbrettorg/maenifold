<script setup lang="ts">
import { computed } from 'vue';
import { storeToRefs } from 'pinia';
import { CircleDot, FileText, GitBranch, Loader2, Network } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useKnowledgeStore } from '../stores/knowledge';

const store = useKnowledgeStore();
const {
  activeCommunityId,
  conceptGraph,
  selectedConcept,
  selectedConceptContext,
  conceptContextLoading,
  conceptContextError,
} = storeToRefs(store);

const selectedNode = computed(() => {
  if (!selectedConcept.value) return null;
  return conceptGraph.value?.nodes.find((node) => node.name === selectedConcept.value) ?? null;
});

const communityId = computed(() => (
  selectedConceptContext.value?.communityId ?? conceptGraph.value?.communityId ?? activeCommunityId.value
));

const memberFiles = computed(() => {
  const files = [
    ...(selectedConceptContext.value?.directRelations.flatMap((relation) => relation.files) ?? []),
    ...(selectedConceptContext.value?.expandedRelations.flatMap((relation) => relation.files) ?? []),
  ].filter((file) => file.startsWith('memory://'));

  return [...new Set(files)].sort((left, right) => left.localeCompare(right));
});

const visibleFiles = computed(() => memberFiles.value.slice(0, 12));

const neighbors = computed(() => {
  const selected = selectedConcept.value;
  if (!selected || !conceptGraph.value) return [];

  return conceptGraph.value.edges
    .flatMap((edge) => {
      if (edge.source === selected) {
        return [{ name: edge.target, weight: edge.weight }];
      }
      if (edge.target === selected) {
        return [{ name: edge.source, weight: edge.weight }];
      }
      return [];
    })
    .sort((left, right) => right.weight - left.weight);
});

function shortFileName(uri: string): string {
  return decodeURIComponent(uri.replace(/^memory:\/\//, '').split('/').at(-1) ?? uri);
}
</script>

<template>
  <aside class="flex h-full min-h-0 w-full flex-col border-l border-border-subtle bg-bg-sidebar" aria-label="Node inspector">
    <div class="border-b border-border-subtle px-4 py-3">
      <div class="flex items-center gap-2">
        <CircleDot class="size-4 text-text-secondary" aria-hidden="true" />
        <p class="text-sm font-semibold text-text-primary">Node inspector</p>
      </div>
      <p class="mt-1 text-xs text-text-secondary">
        Click a concept node to inspect graph bonds. Double-click to open its strongest note.
      </p>
    </div>

    <div v-if="!selectedConcept" class="p-4 text-sm text-text-secondary">
      No concept selected.
    </div>

    <ScrollArea v-else class="min-h-0 flex-1">
      <div class="grid gap-4 p-4">
        <section class="grid gap-2">
          <p class="text-xs font-medium uppercase tracking-wide text-text-tertiary">Concept</p>
          <h2 class="break-words text-lg font-semibold text-text-primary">{{ selectedConcept }}</h2>
          <div class="flex flex-wrap gap-2">
            <Badge variant="secondary">{{ selectedNode?.occurrences ?? 0 }} files</Badge>
            <Badge variant="outline">Community {{ communityId ?? 'unknown' }}</Badge>
          </div>
        </section>

        <section class="grid gap-2 rounded-lg border border-border-subtle bg-bg-surface p-3">
          <div class="flex items-center gap-2">
            <FileText class="size-4 text-text-secondary" aria-hidden="true" />
            <p class="text-sm font-medium text-text-primary">Member files</p>
            <span class="ml-auto text-xs text-text-tertiary">{{ memberFiles.length }}</span>
          </div>
          <div v-if="conceptContextLoading" class="flex items-center gap-2 text-sm text-text-secondary">
            <Loader2 class="size-4 animate-spin" aria-hidden="true" />
            Loading file bonds…
          </div>
          <p v-else-if="conceptContextError" class="text-sm text-status-error">{{ conceptContextError }}</p>
          <ul v-else-if="visibleFiles.length > 0" class="grid gap-1">
            <li v-for="uri in visibleFiles" :key="uri" class="truncate text-sm text-text-secondary" :title="uri">
              {{ shortFileName(uri) }}
            </li>
          </ul>
          <p v-else class="text-sm text-text-secondary">No member files surfaced for this node.</p>
          <p v-if="memberFiles.length > visibleFiles.length" class="text-xs text-text-tertiary">
            +{{ memberFiles.length - visibleFiles.length }} more files
          </p>
        </section>

        <section class="grid gap-2 rounded-lg border border-border-subtle bg-bg-surface p-3">
          <div class="flex items-center gap-2">
            <Network class="size-4 text-text-secondary" aria-hidden="true" />
            <p class="text-sm font-medium text-text-primary">Neighbors</p>
            <span class="ml-auto text-xs text-text-tertiary">{{ neighbors.length }}</span>
          </div>
          <ul v-if="neighbors.length > 0" class="grid gap-2">
            <li v-for="neighbor in neighbors.slice(0, 16)" :key="neighbor.name" class="grid gap-1">
              <div class="flex items-center gap-2">
                <GitBranch class="size-3 text-text-tertiary" aria-hidden="true" />
                <span class="min-w-0 flex-1 truncate text-sm text-text-primary">{{ neighbor.name }}</span>
                <span class="text-xs tabular-nums text-text-tertiary">{{ neighbor.weight }}</span>
              </div>
            </li>
          </ul>
          <p v-else class="text-sm text-text-secondary">No weighted neighbors in this view.</p>
        </section>
      </div>
    </ScrollArea>
  </aside>
</template>
