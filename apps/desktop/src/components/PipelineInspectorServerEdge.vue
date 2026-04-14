<script setup lang="ts">
defineProps<{
  edgePathLabel: string
  sourceStatus: string
  edgeMappings: [string, string][]
  sourceOutput: string | null
  toolTarget: boolean
  resolvedTargetInput: string
}>()
</script>

<template>
  <section class="panel" role="region" aria-label="Pipeline edge details">
    <h2>Edge Details</h2>
    <div class="summary-row"><span>Path</span><strong>{{ edgePathLabel }}</strong></div>
    <div class="summary-row">
      <span>Source status</span>
      <strong :class="`status-text--${sourceStatus}`">{{ sourceStatus }}</strong>
    </div>
    <div class="read-block">
      <span>Mapping</span>
      <div v-if="edgeMappings.length > 0" class="mapping-list">
        <div v-for="[targetField, sourcePath] in edgeMappings" :key="`${targetField}:${sourcePath}`" class="mapping-row">
          <strong>{{ sourcePath }}</strong>
          <span>→</span>
          <code>{{ targetField }}</code>
        </div>
      </div>
      <pre v-else>No mapping configured</pre>
    </div>
    <div class="read-block">
      <span>Source output</span>
      <pre v-if="sourceOutput">{{ sourceOutput }}</pre>
      <pre v-else>Source output will appear after the source node completes</pre>
    </div>
    <div class="read-block">
      <span>Resolved target input</span>
      <pre v-if="toolTarget">{{ resolvedTargetInput }}</pre>
      <pre v-else>Runtime input mapping applies to tool targets only</pre>
    </div>
  </section>
</template>

<style scoped>
.panel { padding: 1rem; display: flex; flex-direction: column; gap: 0.875rem; background: var(--pipeline-inspector-surface); border-radius: 4px; }
.panel h2 { font-size: var(--text-xl); font-weight: var(--weight-title); }
.summary-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
.summary-row span,.read-block span { color: var(--muted-foreground); }
.status-text--pending { color: var(--color-status-pending); }
.status-text--active { color: var(--color-status-active); }
.status-text--completed { color: var(--color-status-completed); }
.status-text--abandoned { color: var(--color-status-abandoned); }
.status-text--stuck { color: var(--color-status-stuck); }
.status-text--ready,
.status-text--paused { color: var(--color-status-ready); }
.status-text--cancelled { color: var(--color-status-abandoned); }
.read-block { display: flex; flex-direction: column; gap: 0.375rem; color: var(--muted-foreground); font-size: var(--text-sm); }
.read-block pre {
  margin: 0; border: 1px solid var(--pipeline-inspector-ghost-border); border-radius: 4px; padding: 0.75rem;
  background: var(--pipeline-inspector-well); white-space: pre-wrap; word-break: break-word; overflow-wrap: break-word; font: inherit; color: var(--foreground);
}
.mapping-list { display: flex; flex-direction: column; gap: 0.5rem; }
.mapping-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
</style>
