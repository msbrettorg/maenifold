<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core';
</script>

<template>
  <div class="pipeline-node-ports">
    <Handle type="target" :position="Position.Left" />
    <Handle type="source" :position="Position.Right" />
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
</style>
