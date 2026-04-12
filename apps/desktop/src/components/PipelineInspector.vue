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
</template>
