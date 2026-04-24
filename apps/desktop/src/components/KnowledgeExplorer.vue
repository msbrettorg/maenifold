<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Search, X, BookOpenText, Network } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import KnowledgeContent from './KnowledgeContent.vue';
import KnowledgeGraphPanel from './KnowledgeGraphPanel.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import KnowledgeSidebar from './KnowledgeSidebar.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { useKnowledgeStore } from '../stores/knowledge';
import { useProvenanceStore } from '../stores/provenance';

const store = useKnowledgeStore();
const provenance = useProvenanceStore();
const { navigateTo } = useProvenanceNavigation();
const query = ref(store.searchQuery);
const mode = ref<'notes' | 'graph'>('notes');
const currentKnowledgeLineageId = computed(() => store.activeUri);
const currentKnowledgeTitle = computed(() => store.activeNote?.title);
const activeNoteLineageLinks = computed(() =>
  provenance.buildLineageLinks({
    gate: store.activeNote?.lineageGateTaskId,
    pipeline: store.activeNote?.lineagePipelineRunId,
    thinking: store.activeNote?.lineageThinkingSessionId,
    knowledge: store.activeUri,
    parentWorkflowId: store.activeNote?.parentWorkflowId,
  }, {
    knowledge: currentKnowledgeTitle.value,
  }));
const provenanceLinks = computed(() =>
  activeNoteLineageLinks.value
    .filter((link) => link.kind !== 'knowledge' && link.id !== currentKnowledgeLineageId.value));

async function handleSearch() {
  await store.search(query.value);
}

async function clearSearch() {
  query.value = '';
  await store.search('');
}

onMounted(async () => {
  await store.loadRootFolders();
});
</script>

<template>
  <section class="flex h-full min-h-0 flex-1 flex-col bg-bg-app" aria-label="Knowledge explorer">
    <div class="px-4 pt-3">
      <ProvenanceBreadcrumbs
        current-kind="knowledge"
        :current-id="currentKnowledgeLineageId"
        :current-label="store.activeNote?.title"
        :links="provenanceLinks"
        @navigate="navigateTo"
      />
    </div>
    <header class="flex flex-wrap items-center gap-2 border-b border-border-subtle px-4 py-3">
      <Tabs
        :model-value="mode"
        @update:model-value="mode = ($event as 'notes' | 'graph')"
      >
        <TabsList variant="line" class="bg-transparent p-0">
          <TabsTrigger value="notes" class="h-8 rounded-md px-2 py-1">
            <BookOpenText class="size-4" aria-hidden="true" />
            Notes
          </TabsTrigger>
          <TabsTrigger value="graph" class="h-8 rounded-md px-2 py-1">
            <Network class="size-4" aria-hidden="true" />
            Graph
          </TabsTrigger>
        </TabsList>
      </Tabs>

      <div class="relative min-w-[16rem] flex-1">
        <Search class="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-text-tertiary" aria-hidden="true" />
        <Input
          v-model="query"
          class="h-9 border-border-subtle bg-bg-surface pl-8 text-text-primary placeholder:text-text-secondary"
          placeholder="Search notes..."
          @keydown.enter.prevent="handleSearch"
        />
      </div>
      <Button type="button" size="sm" class="h-9" @click="handleSearch">Search</Button>
      <Button
        v-if="query.trim().length > 0"
        type="button"
        variant="ghost"
        size="sm"
        class="h-9 text-text-secondary hover:text-text-primary"
        @click="clearSearch"
      >
        <X class="size-4" aria-hidden="true" />
        Clear
      </Button>
    </header>

    <div v-if="mode === 'notes'" class="flex min-h-0 flex-1">
      <div class="min-h-0 w-[20rem] shrink-0 border-r border-border-subtle">
        <KnowledgeSidebar />
      </div>
      <KnowledgeContent />
    </div>

    <div v-else class="flex min-h-0 flex-1">
      <KnowledgeGraphPanel />
      <div class="min-h-0 w-[32rem] shrink-0 border-l border-border-subtle">
        <KnowledgeContent />
      </div>
    </div>
  </section>
</template>
