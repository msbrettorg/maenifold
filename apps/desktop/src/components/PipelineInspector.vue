<script setup lang="ts">
import DesignNodeConfigPanel from './DesignNodeConfigPanel.vue';
import PipelineInspectorServer from './PipelineInspectorServer.vue';
import { usePipelineStore } from '../stores/pipeline';

defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const store = usePipelineStore();
</script>

<template>
  <section class="pipeline-inspector" role="region" aria-label="Pipeline inspector">
    <DesignNodeConfigPanel
      v-if="store.designMode"
      :selected-node-id="selectedNodeId"
      :selected-edge="selectedEdge"
    />
    <PipelineInspectorServer
      v-else
      :selected-node-id="selectedNodeId"
      :selected-edge="selectedEdge"
    />
  </section>
</template>

<style scoped>
.pipeline-inspector {
  width: 100%;
  height: 100%;
  min-height: 0;
}
</style>
