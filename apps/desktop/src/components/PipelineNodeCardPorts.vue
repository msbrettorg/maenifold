<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core';
import { computed, inject, ref } from 'vue';

const props = defineProps<{
  direction?: 'LR' | 'TB';
}>();

const injectedDirection = inject('pipelineLayoutDirection', ref<'LR' | 'TB'>('LR'));
const direction = computed<'LR' | 'TB'>(() => props.direction ?? injectedDirection.value);
</script>

<template>
  <div class="pipeline-node-ports">
    <Handle
      id="in-top"
      type="target"
      :position="Position.Top"
      :class="['pipeline-port', { 'pipeline-port--inactive': direction !== 'TB' }]"
    />
    <Handle
      id="out-bottom"
      type="source"
      :position="Position.Bottom"
      :class="['pipeline-port', { 'pipeline-port--inactive': direction !== 'TB' }]"
    />
    <Handle
      id="in-left"
      type="target"
      :position="Position.Left"
      :class="['pipeline-port', { 'pipeline-port--inactive': direction !== 'LR' }]"
    />
    <Handle
      id="out-right"
      type="source"
      :position="Position.Right"
      :class="['pipeline-port', { 'pipeline-port--inactive': direction !== 'LR' }]"
    />
  </div>
</template>

<style scoped>
.pipeline-node-ports :deep(.vue-flow__handle) {
  width: 12px;
  height: 12px;
  border: 2px solid var(--color-bg-app);
  background: var(--node-accent);
}

.pipeline-node-ports :deep(.vue-flow__handle-left) {
  left: -5px;
}

.pipeline-node-ports :deep(.vue-flow__handle-right) {
  right: -5px;
}

.pipeline-node-ports :deep(.vue-flow__handle-top) {
  top: -5px;
}

.pipeline-node-ports :deep(.vue-flow__handle-bottom) {
  bottom: -5px;
}

.pipeline-node-ports :deep(.pipeline-port--inactive) {
  opacity: 0;
  pointer-events: none;
}
</style>
