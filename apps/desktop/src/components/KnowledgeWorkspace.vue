<script setup lang="ts">
import { computed, onMounted } from 'vue';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import KnowledgeGraphPane from './KnowledgeGraphPane.vue';
import KnowledgeLeafPane from './KnowledgeLeafPane.vue';
import KnowledgeTabStrip from './KnowledgeTabStrip.vue';
import KnowledgeVaultBrowser from './KnowledgeVaultBrowser.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { useKnowledgeStore } from '../stores/knowledge';
import { useProvenanceStore } from '../stores/provenance';

const store = useKnowledgeStore();
const provenance = useProvenanceStore();
const { navigateTo } = useProvenanceNavigation();

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

    <ResizablePanelGroup
      direction="horizontal"
      auto-save-id="knowledge-workspace"
      class="min-h-0 flex-1"
    >
      <ResizablePanel id="knowledge-browser" :default-size="20" :min-size="15" :order="1">
        <div class="h-full w-full min-w-0 min-h-0">
          <KnowledgeVaultBrowser />
        </div>
      </ResizablePanel>

      <ResizableHandle
        class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
      />

      <ResizablePanel id="knowledge-content" :default-size="50" :min-size="30" :order="2">
        <div class="flex h-full w-full min-w-0 min-h-0 flex-col">
          <KnowledgeTabStrip />
          <KnowledgeLeafPane />
        </div>
      </ResizablePanel>

      <ResizableHandle
        class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
      />

      <ResizablePanel
        id="knowledge-graph"
        :default-size="30"
        :min-size="15"
        :order="3"
        collapsible
      >
        <div class="h-full w-full min-w-0 min-h-0">
          <KnowledgeGraphPane />
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  </section>
</template>
