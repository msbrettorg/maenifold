<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { Tabs } from '@/components/ui/tabs';
import KnowledgeGraphView from './KnowledgeGraphView.vue';
import KnowledgeMemoryView from './KnowledgeMemoryView.vue';
import KnowledgeSubtabToggle from './KnowledgeSubtabToggle.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { useAppShellStore } from '../stores/app-shell';
import { useKnowledgeStore, type KnowledgeSubtab } from '../stores/knowledge';
import { useProvenanceStore } from '../stores/provenance';

const store = useKnowledgeStore();
const appShell = useAppShellStore();
const provenance = useProvenanceStore();
const { navigateTo } = useProvenanceNavigation();
const { activeSubtab } = storeToRefs(store);
const memoryView = ref<{ focusSearchInput: () => void; openNewNoteDialog: () => void } | null>(null);

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

function selectSubtab(value: string | number): void {
  if (value === 'memory' || value === 'graph') {
    store.setActiveSubtab(value as KnowledgeSubtab);
  }
}

async function activateMemorySubtab(): Promise<void> {
  if (store.activeSubtab !== 'memory') {
    store.setActiveSubtab('memory');
    await nextTick();
  }
}

async function focusMemorySearch(): Promise<void> {
  await activateMemorySubtab();
  memoryView.value?.focusSearchInput();
}

async function openNewNoteDialog(): Promise<void> {
  await activateMemorySubtab();
  memoryView.value?.openNewNoteDialog();
}

function handleKnowledgeShortcut(event: KeyboardEvent): void {
  if (appShell.activeTab !== 'knowledge' || !(event.metaKey || event.ctrlKey)) {
    return;
  }

  const key = event.key.toLocaleLowerCase();
  if (key === 'f') {
    event.preventDefault();
    void focusMemorySearch();
  } else if (key === 'n') {
    event.preventDefault();
    void openNewNoteDialog();
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKnowledgeShortcut);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKnowledgeShortcut);
});
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

      <div class="min-h-0 flex-1">
        <KnowledgeMemoryView v-if="activeSubtab === 'memory'" ref="memoryView" />
        <div v-else-if="activeSubtab === 'graph'" class="h-full min-h-0">
          <KnowledgeGraphView />
        </div>
      </div>
    </Tabs>
  </section>
</template>
