<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import type { LogEntry } from '../lib/pipeline-types';
import { getPipelineNodeDisplayLabel, getViewNodeDisplayLabel } from '../lib/pipeline-types';
import { buildNodeOutputPreview } from '../composables/pipeline-view-diff';

const props = defineProps<{ collapsed?: boolean }>();

const store = usePipelineStore();
const { currentView, designMode, eventLog, localDefinition } = storeToRefs(store);

const logContainer = ref<HTMLElement | null>(null);

const localNodesById = computed(() => new Map(localDefinition.value.nodes.map((node) => [node.id, node])));
const viewNodesById = computed(() => new Map((currentView.value?.nodes ?? []).map((node) => [node.nodeId, node])));
const displayNodeId = (nodeId: string) => {
  const viewNode = viewNodesById.value.get(nodeId);
  if (viewNode) {
    return getViewNodeDisplayLabel(viewNode);
  }

  const localNode = localNodesById.value.get(nodeId);
  return localNode ? getPipelineNodeDisplayLabel(localNode) : nodeId;
};

const statusLine = computed(() => {
  if (designMode.value) {
    return `Design mode — ${localDefinition.value.nodes.length} nodes, ${localDefinition.value.edges.length} edges`;
  }
  if (!currentView.value) return 'No pipeline loaded';
  const activeList = currentView.value.active.length > 0
    ? currentView.value.active.map(displayNodeId).join(', ')
    : 'none';
  return `● ${currentView.value.status} — ${currentView.value.completed.length}/${currentView.value.nodes.length} completed — Active: ${activeList}`;
});

const formattedLog = computed(() => eventLog.value.map((entry) => formatLogEntry(entry)));

watch(() => eventLog.value.length, async (length, previousLength) => {
  if (length === previousLength) return;
  await nextTick();
  if (logContainer.value) {
    logContainer.value.scrollTop = logContainer.value.scrollHeight;
  }
});

function iconFor(status: string): string {
  switch (status) {
    case 'active': return '▶';
    case 'completed': return '✅';
    case 'abandoned': return '❌';
    case 'stuck': return '🚫';
    case 'ready': return '🟡';
    case 'pending': return '⏳';
    default: return '•';
  }
}

function formatLogEntry(entry: LogEntry) {
  const timestamp = new Date(entry.timestamp).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
  const preview = entry.preview ?? buildNodeOutputPreview(viewNodesById.value.get(entry.nodeId));
  if (entry.from === 'loaded') {
    const base = `[${timestamp}] ↺ ${displayNodeId(entry.nodeId)} loaded as ${entry.to}`;
    if (entry.to === 'completed' && preview) {
      return `${base} — output: ${preview}`;
    }
    if ((entry.to === 'abandoned' || entry.to === 'stuck') && preview) {
      return `${base} — details: ${preview}`;
    }
    return base;
  }
  if (entry.to === 'completed' && preview) {
    return `[${timestamp}] ✅ ${displayNodeId(entry.nodeId)} completed — output: ${preview}`;
  }
  if (entry.to === 'abandoned' && preview) {
    return `[${timestamp}] ❌ ${displayNodeId(entry.nodeId)} abandoned — details: ${preview}`;
  }
  if (entry.to === 'stuck' && preview) {
    return `[${timestamp}] 🚫 ${displayNodeId(entry.nodeId)} stuck — details: ${preview}`;
  }
  return `[${timestamp}] ${iconFor(entry.to)} ${displayNodeId(entry.nodeId)}: ${entry.from} → ${entry.to}`;
}
</script>

<template>
  <section class="runner-panel" :class="{ 'runner-panel--collapsed': props.collapsed }">
    <div class="status-line">{{ statusLine }}</div>

    <div v-show="!props.collapsed" ref="logContainer" class="tab-panel log-panel" aria-live="polite" aria-label="Event log">
      <p v-if="formattedLog.length === 0" class="empty-state">No events yet.</p>
      <pre v-else class="log-lines"><code>{{ formattedLog.join('\n') }}</code></pre>
    </div>
  </section>
</template>

<style scoped>
.runner-panel {
  display: flex;
  flex-direction: column;
  min-height: 260px;
  border-top: 1px solid var(--border);
  background: var(--background);
}
.runner-panel--collapsed {
  min-height: 0;
}
.status-line {
  padding: 0.75rem 1rem;
  color: var(--muted-foreground);
  font-size: var(--text-base);
  border-bottom: 1px solid var(--border);
}
.tab-panel {
  flex: 1;
  min-height: 0;
  padding: 1rem;
  overflow: auto;
}
.log-panel {
  background: var(--muted);
}
.log-lines {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-code);
  font-size: var(--text-sm);
  color: var(--foreground);
}
.empty-state {
  color: var(--muted-foreground);
}
</style>
