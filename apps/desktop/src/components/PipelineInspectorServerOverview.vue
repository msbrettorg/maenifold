<script setup lang="ts">
defineProps<{
  pipelineId: string
  status: string
  completedCount: number
  nodeCount: number
  running: boolean
}>()
</script>

<template>
  <section class="panel" role="region" aria-label="Pipeline overview">
    <h2>Pipeline Overview</h2>
    <div class="summary-row"><span>Pipeline ID</span><strong class="mono">{{ pipelineId }}</strong></div>
    <div class="summary-row">
      <span>Status</span>
      <strong :class="['status-text', `status-text--${status}`]">{{ status }}</strong>
    </div>
    <div class="summary-row"><span>Progress</span><strong>{{ completedCount }} / {{ nodeCount }}</strong></div>
    <div class="summary-row"><span>Running</span><strong>{{ running ? 'Yes' : 'No' }}</strong></div>
  </section>
</template>

<style scoped>
.panel { padding: 1rem; display: flex; flex-direction: column; gap: 0.875rem; }
.panel h2 { font-size: var(--text-xl); font-weight: var(--weight-title); }
.summary-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
.summary-row span { color: var(--muted-foreground); }
.mono { font-family: var(--font-code); word-break: break-word; overflow-wrap: break-word; }
</style>
