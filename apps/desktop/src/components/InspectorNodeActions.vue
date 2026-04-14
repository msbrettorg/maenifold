<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { usePipelineStore } from '../stores/pipeline';
import type { ViewNode } from '../lib/pipeline-types';
import { useInspectorActions } from '../composables/useInspectorActions';
import PromotionDialog from './PromotionDialog.vue';

const props = defineProps<{
  node: ViewNode;
  nodeId: string;
}>();

const store = usePipelineStore();
const conclusionText = ref('');
const forkedSessionId = ref<string | null>(null);
const showPromotionDialog = ref(false);
const promotionBusy = ref(false);

const { busyAction, runBusyAction, completeNode, restartNode, rewindNode, abandonNode } = useInspectorActions({
  store,
  selectedNodeId: computed<string | null>(() => props.nodeId),
  selectedViewNode: computed<ViewNode | null>(() => props.node),
  conclusionText,
});

const canPromote = computed(() => (
  props.node.status === 'completed'
  && typeof props.node.output === 'string'
  && props.node.output.trim().length > 0
  && typeof store.currentView?.pipelineId === 'string'
  && store.currentView.pipelineId.length > 0
));
const promoteButtonTitle = computed(() => (
  promotionBusy.value
    ? 'Promotion in progress…'
    : 'Save this completed output to memory'
));

watch(() => props.nodeId, () => {
  forkedSessionId.value = null;
  showPromotionDialog.value = false;
  promotionBusy.value = false;
});

function extractForkedSessionId(result: unknown): string | null {
  if (typeof result === 'string') {
    return result.trim() || null;
  }

  if (!result || typeof result !== 'object') {
    return null;
  }

  const record = result as Record<string, unknown>;
  for (const key of ['sessionId', 'pipelineId', 'newSessionId', 'forkedSessionId']) {
    const value = record[key];
    if (typeof value === 'string' && value.trim().length > 0) {
      return value;
    }
  }

  return null;
}

async function forkNode(): Promise<void> {
  if (!props.node.sessionId || !props.nodeId) return;

  const requestedSessionIdInput = window.prompt('Optional name for the forked pipeline session:', '');
  if (requestedSessionIdInput === null) return;
  const requestedSessionId = requestedSessionIdInput.trim();

  forkedSessionId.value = null;

  const result = await runBusyAction('fork', () =>
    store.forkPipeline(
      props.node.sessionId,
      props.nodeId,
      requestedSessionId || undefined,
    ),
  );

  forkedSessionId.value = extractForkedSessionId(result);
}

function openPromotionDialog(): void {
  if (!canPromote.value || promotionBusy.value) {
    return;
  }

  showPromotionDialog.value = true;
}
</script>

<template>
  <label v-if="node.status === 'active'" class="field">
    <span>Completion note</span>
    <textarea v-model="conclusionText" class="field-textarea" rows="4" placeholder="Optional conclusion for this node" />
  </label>
  <div class="actions">
    <button
      v-if="canPromote"
      :class="['btn', 'btn-primary', { 'btn--busy': promotionBusy }]"
      :disabled="promotionBusy"
      :title="promoteButtonTitle"
      type="button"
      @click="openPromotionDialog"
    >
      {{ promotionBusy ? 'Promoting…' : 'Promote to Memory' }}
    </button>
    <button
      :class="['btn', 'btn-ghost', { 'btn--busy': busyAction === 'fork' }]"
      :disabled="busyAction !== null"
      type="button"
      @click="forkNode"
    >
      {{ busyAction === 'fork' ? 'Forking...' : 'Fork from here' }}
    </button>
    <button
      v-if="['abandoned', 'stuck'].includes(node.status)"
      :class="['btn', 'btn-primary', { 'btn--busy': busyAction === 'restart' }]"
      :disabled="busyAction !== null"
      type="button"
      @click="restartNode"
    >
      {{ busyAction === 'restart' ? 'Restart...' : 'Restart' }}
    </button>
    <button
      v-if="['completed', 'abandoned', 'stuck'].includes(node.status)"
      :class="['btn', 'btn-ghost', { 'btn--busy': busyAction === 'rewind' }]"
      :disabled="busyAction !== null"
      type="button"
      @click="rewindNode"
    >
      {{ busyAction === 'rewind' ? 'Rewind...' : 'Rewind' }}
    </button>
    <button
      v-if="node.status === 'active'"
      :class="['btn', 'btn-ghost', { 'btn--busy': busyAction === 'complete' }]"
      :disabled="busyAction !== null"
      type="button"
      @click="completeNode"
    >
      {{ busyAction === 'complete' ? 'Complete...' : 'Complete' }}
    </button>
    <button
      v-if="node.status === 'active'"
      :class="['btn', 'btn-danger', { 'btn--busy': busyAction === 'abandon' }]"
      :disabled="busyAction !== null"
      type="button"
      @click="abandonNode"
    >
      {{ busyAction === 'abandon' ? 'Abandon...' : 'Abandon' }}
    </button>
  </div>
  <div v-if="forkedSessionId" class="fork-result" role="status">
    <span>Forked session</span>
    <strong class="mono">{{ forkedSessionId }}</strong>
  </div>
  <PromotionDialog
    v-if="showPromotionDialog && canPromote && store.currentView?.pipelineId"
    :node="node"
    :pipeline-id="store.currentView.pipelineId"
    @busy-change="promotionBusy = $event"
    @close="showPromotionDialog = false"
  />
</template>

<style scoped>
.field { display: flex; flex-direction: column; gap: 0.375rem; color: var(--text-secondary); font-size: var(--text-sm); }
.actions { display: flex; flex-wrap: wrap; gap: 0.75rem; }
.fork-result { display: flex; flex-direction: column; gap: 0.25rem; padding: 0.75rem; border: 1px solid var(--border-default); border-radius: 8px; background: var(--bg-main); }
.fork-result span { color: var(--text-secondary); font-size: var(--text-sm); }
.mono { font-family: var(--font-code); word-break: break-all; }
</style>
