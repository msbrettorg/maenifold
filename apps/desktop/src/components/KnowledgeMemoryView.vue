<script setup lang="ts">
import { onMounted } from 'vue';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { useKnowledgeStore } from '../stores/knowledge';
import KnowledgeLeafPane from './KnowledgeLeafPane.vue';
import KnowledgeTabStrip from './KnowledgeTabStrip.vue';
import KnowledgeVaultBrowser from './KnowledgeVaultBrowser.vue';

const store = useKnowledgeStore();

onMounted(() => {
  void store.loadRootFolders();
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
        :default-size="20"
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
        :default-size="80"
        :min-size="30"
        :order="2"
      >
        <div class="flex h-full w-full min-h-0 min-w-0 flex-col bg-bg-app">
          <KnowledgeTabStrip />
          <KnowledgeLeafPane />
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  </section>
</template>
