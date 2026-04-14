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
  --pipeline-inspector-surface: #1c1b1b;
  --pipeline-inspector-well: #201f1f;
  --pipeline-inspector-ghost-border: rgba(66, 71, 83, 0.15);
  --pipeline-inspector-ghost-border-focus: rgba(173, 198, 255, 0.4);
  width: 100%;
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  background: var(--pipeline-inspector-surface);
}

.pipeline-inspector :deep(input),
.pipeline-inspector :deep(select),
.pipeline-inspector :deep(textarea) {
  background: var(--pipeline-inspector-surface);
  border: 1px solid var(--pipeline-inspector-ghost-border);
  color: var(--foreground);
}

.pipeline-inspector :deep(input::placeholder),
.pipeline-inspector :deep(textarea::placeholder) {
  color: #707070;
}

.pipeline-inspector :deep(input:focus),
.pipeline-inspector :deep(select:focus),
.pipeline-inspector :deep(textarea:focus) {
  outline: none;
  border-color: var(--pipeline-inspector-ghost-border-focus);
  background: #2a2a2a;
}
</style>
