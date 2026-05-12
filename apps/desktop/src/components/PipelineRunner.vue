<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import type { Component } from 'vue';
import { Play, CheckCircle, XCircle, AlertTriangle, Pause, Clock } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { ScrollArea } from '@/components/ui/scroll-area';
import { usePipelineStore } from '../stores/pipeline';
import type { LogEntry } from '../lib/pipeline-types';
import { getPipelineNodeDisplayLabel, getViewNodeDisplayLabel } from '../lib/pipeline-types';
import { buildNodeOutputPreview, buildAbandonedNodePreview } from '../composables/pipeline-view-diff';
import PipelineAcpSessionInspectors from './PipelineAcpSessionInspectors.vue';
import PipelineRunHistoryPanel from './PipelineRunHistoryPanel.vue';

const props = defineProps<{ collapsed?: boolean }>();

const store = usePipelineStore();
const { currentView, currentPipelineId, designMode, eventLog, localDefinition, pipelines } = storeToRefs(store);

const currentRuns = computed(() => {
  if (!currentPipelineId.value) return [];
  return pipelines.value.find((p) => p.pipelineId === currentPipelineId.value)?.runs ?? [];
});

const logAnchor = ref<HTMLElement | null>(null);

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
  return `${currentView.value.status} — ${currentView.value.completed.length}/${currentView.value.nodes.length} completed — Active: ${activeList}`;
});

const collapsedSummaryLine = computed(() => {
  if (designMode.value) {
    return `Design mode — ${localDefinition.value.nodes.length} nodes, ${localDefinition.value.edges.length} edges`;
  }
  if (!currentView.value) return 'No pipeline loaded';
  const activeNodeId = currentView.value.active[0] ?? 'none';
  return `${currentView.value.status} — ${currentView.value.completed.length}/${currentView.value.nodes.length} completed — Active: ${activeNodeId}`;
});

const formattedLog = computed(() => eventLog.value.map((entry) => formatLogEntry(entry)));

watch(() => eventLog.value.length, async (length, previousLength) => {
  if (length === previousLength) return;
  await nextTick();
  logAnchor.value?.scrollIntoView({ block: 'end' });
});

function iconFor(status: string): Component {
  switch (status) {
    case 'active': return Play;
    case 'completed': return CheckCircle;
    case 'failed':
    case 'cancelled': return XCircle;
    case 'abandoned':
    case 'stuck': return AlertTriangle;
    case 'paused': return Pause;
    default: return Clock;
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case 'active': return 'active';
    case 'completed': return 'completed';
    case 'failed': return 'failed';
    case 'cancelled': return 'cancelled';
    case 'abandoned': return 'abandoned';
    case 'stuck': return 'stuck';
    case 'paused': return 'paused';
    default: return status;
  }
}

function formatLogEntry(entry: LogEntry) {
  const timestamp = new Date(entry.timestamp).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
  const viewNodeForEntry = viewNodesById.value.get(entry.nodeId);
  const preview = entry.preview
    ?? (entry.to === 'abandoned' || entry.to === 'stuck'
      ? buildAbandonedNodePreview(viewNodeForEntry)
      : buildNodeOutputPreview(viewNodeForEntry));
  if (entry.from === 'loaded') {
    const base = `[${timestamp}] ${displayNodeId(entry.nodeId)} loaded as ${entry.to}`;
    if (entry.to === 'completed' && preview) {
      return `${base} — output: ${preview}`;
    }
    if ((entry.to === 'abandoned' || entry.to === 'stuck') && preview) {
      return `${base} — details: ${preview}`;
    }
    return base;
  }
  if (entry.to === 'completed' && preview) {
    return `[${timestamp}] ${displayNodeId(entry.nodeId)} completed — output: ${preview}`;
  }
  if (entry.to === 'abandoned' && preview) {
    return `[${timestamp}] ${displayNodeId(entry.nodeId)} abandoned: ${preview}`;
  }
  if (entry.to === 'stuck' && preview) {
    return `[${timestamp}] ${displayNodeId(entry.nodeId)} stuck: ${preview}`;
  }
  return `[${timestamp}] ${statusLabel(entry.to)} ${displayNodeId(entry.nodeId)}: ${entry.from} → ${entry.to}`;
}
</script>

<template>
  <section
    class="flex flex-col bg-surface"
    :class="props.collapsed ? 'min-h-0' : 'min-h-[260px]'"
  >
    <div class="flex items-center gap-2 bg-surface-container px-4 py-3 text-base text-text-secondary">
      <component
        :is="iconFor(currentView?.status ?? 'pending')"
        class="size-3.5"
        :stroke-width="1.5"
      />
      <span>{{ props.collapsed ? collapsedSummaryLine : statusLine }}</span>
    </div>

    <PipelineAcpSessionInspectors v-show="!props.collapsed" />
    <PipelineRunHistoryPanel
      v-show="!props.collapsed"
      :runs="currentRuns"
    />
    <ScrollArea
      v-show="!props.collapsed"
      class="flex-1 min-h-0 bg-surface-container"
      aria-live="polite"
      aria-label="Event log"
    >
      <div class="min-h-full p-4">
        <p v-if="formattedLog.length === 0" class="text-text-secondary">No events yet.</p>
        <pre v-else class="m-0 whitespace-pre-wrap break-words font-mono text-sm text-text-primary"><code>{{ formattedLog.join('\n') }}</code></pre>
        <div ref="logAnchor" />
      </div>
    </ScrollArea>
  </section>
</template>
