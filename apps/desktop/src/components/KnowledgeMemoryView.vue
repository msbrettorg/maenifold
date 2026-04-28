<script setup lang="ts">
import { onMounted, ref, watch } from 'vue';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { useKnowledgeStore } from '../stores/knowledge';
import KnowledgeLeafPane from './KnowledgeLeafPane.vue';
import KnowledgeRelatedConcepts from './KnowledgeRelatedConcepts.vue';
import KnowledgeTabStrip from './KnowledgeTabStrip.vue';
import KnowledgeVaultBrowser from './KnowledgeVaultBrowser.vue';

const store = useKnowledgeStore();
const relatedRailCollapsed = ref(false);
const relatedRailPanel = ref<{ collapse: () => void; expand: () => void } | null>(null);

onMounted(() => {
  void store.loadRootFolders();
});

watch(relatedRailCollapsed, (collapsed) => {
  if (collapsed) {
    relatedRailPanel.value?.collapse();
    return;
  }
  relatedRailPanel.value?.expand();
});
</script>

<template>
  <section class="flex h-full w-full min-h-0 min-w-0 overflow-hidden" aria-label="Memory vault">
    <ResizablePanelGroup
      direction="horizontal"
      auto-save-id="knowledge-memory-layout"
      class="h-full w-full"
    >
      <ResizablePanel
        id="knowledge-memory-vault"
        :default-size="18"
        :min-size="15"
        :max-size="35"
        :order="1"
      >
        <div class="flex h-full w-full min-h-0 min-w-0">
          <KnowledgeVaultBrowser />
        </div>
      </ResizablePanel>

      <ResizableHandle
        class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
      />

      <ResizablePanel
        id="knowledge-memory-leaf"
        :default-size="56"
        :min-size="30"
        :order="2"
      >
        <div class="flex h-full w-full min-h-0 min-w-0 flex-col bg-bg-app">
          <KnowledgeTabStrip />
          <KnowledgeLeafPane />
        </div>
      </ResizablePanel>

      <ResizableHandle
        class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
      />

      <ResizablePanel
        ref="relatedRailPanel"
        id="knowledge-memory-related"
        :default-size="26"
        :min-size="15"
        :max-size="35"
        :collapsed-size="4"
        collapsible
        :order="3"
        @collapse="relatedRailCollapsed = true"
        @expand="relatedRailCollapsed = false"
      >
        <KnowledgeRelatedConcepts
          v-model:collapsed="relatedRailCollapsed"
          rail
          open-in-memory
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  </section>
</template>
