<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { Tabs } from '@/components/ui/tabs';
import KnowledgeSubtabToggle from './KnowledgeSubtabToggle.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { useKnowledgeStore, type KnowledgeSubtab } from '../stores/knowledge';
import { useProvenanceStore } from '../stores/provenance';

const store = useKnowledgeStore();
const provenance = useProvenanceStore();
const { navigateTo } = useProvenanceNavigation();
const { activeSubtab } = storeToRefs(store);

const currentKnowledgeTitle = computed(() => store.activeNote?.title);
const provenanceLinks = computed(() => (
  provenance.buildLineageLinks({
    gate: store.activeNote?.lineageGateTaskId,
    pipeline: store.activeNote?.lineagePipelineRunId,
    thinking: store.activeNote?.lineageThinkingSessionId,
    knowledge: store.activeTabUri,
    parentWorkflowId: store.activeNote?.parentWorkflowId,
  }, {
    knowledge: currentKnowledgeTitle.value,
  }).filter((link) => link.kind !== 'knowledge' && link.id !== store.activeTabUri)
));

onMounted(() => {
  void store.loadRootFolders();
});

function selectSubtab(value: string | number): void {
  if (value === 'memory' || value === 'graph') {
    store.setActiveSubtab(value as KnowledgeSubtab);
  }
}
</script>

<template>
  <section class="flex h-full min-h-0 flex-1 flex-col bg-bg-app" aria-label="Knowledge workspace">
    <div class="border-b border-border-subtle px-4 py-3">
      <ProvenanceBreadcrumbs
        current-kind="knowledge"
        :current-id="store.activeTabUri"
        :current-label="store.activeNote?.title"
        :links="provenanceLinks"
        @navigate="navigateTo"
      />
    </div>

    <Tabs
      :model-value="activeSubtab"
      class="min-h-0 flex-1"
      @update:model-value="selectSubtab"
    >
      <div class="border-b border-border-subtle px-4">
        <KnowledgeSubtabToggle />
      </div>

      <div class="min-h-0 flex-1 p-4 text-sm text-text-secondary">
        <div v-if="activeSubtab === 'memory'">
          Memory subtab — vault + viewer coming in T-242.13.2
        </div>
        <div v-else-if="activeSubtab === 'graph'">
          Graph subtab — full-canvas graph coming in T-242.13.4
        </div>
      </div>
    </Tabs>
  </section>
</template>
