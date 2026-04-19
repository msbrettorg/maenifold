<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import type { SequentialThinkingViewNode } from '../lib/pipeline-types';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { usePipelineStore } from '../stores/pipeline';
import { useProvenanceStore } from '../stores/provenance';
import { useSemanticZoomStore } from '../stores/semantic-zoom';
import './ThinkingTraceView.css';

const props = defineProps<{
  trace: NonNullable<SequentialThinkingViewNode['thinking_trace']>;
}>();

const pipelineStore = usePipelineStore();
const provenance = useProvenanceStore();
const zoomStore = useSemanticZoomStore();
const { currentView } = storeToRefs(pipelineStore);
const { navigateTo } = useProvenanceNavigation();

const focusedNode = computed(() =>
  currentView.value?.nodes.find((node) => node.nodeId === zoomStore.focusedNodeId) ?? null);

const currentThinkingSessionId = computed(() =>
  focusedNode.value?.provenance?.detail.thinkingSessionId
  ?? focusedNode.value?.thinkingSessionId
  ?? null);

const currentThinkingLabel = computed(() =>
  currentThinkingSessionId.value
    ? `Session ${currentThinkingSessionId.value.slice(0, 8)}…`
    : 'Trace');

function firstNonEmptyId(...values: Array<string | null | undefined>): string | null {
  for (const value of values) {
    if (typeof value !== 'string') {
      continue;
    }
    const trimmed = value.trim();
    if (trimmed.length > 0) {
      return trimmed;
    }
  }
  return null;
}

const focusedNodeProvenanceDetail = computed(() => focusedNode.value?.provenance?.detail ?? null);
const thinkingLineageLinks = computed(() =>
  provenance.buildLineageLinks({
    gate: firstNonEmptyId(
      focusedNodeProvenanceDetail.value?.lineageGateTaskId,
      focusedNodeProvenanceDetail.value?.lineage_gate_task_id,
    ),
    pipeline: firstNonEmptyId(
      focusedNodeProvenanceDetail.value?.lineagePipelineRunId,
      focusedNodeProvenanceDetail.value?.lineage_pipeline_run_id,
      focusedNodeProvenanceDetail.value?.parentWorkflowId,
      focusedNodeProvenanceDetail.value?.parent_workflow_id,
    ),
    thinking: firstNonEmptyId(
      currentThinkingSessionId.value,
      focusedNodeProvenanceDetail.value?.lineageThinkingSessionId,
      focusedNodeProvenanceDetail.value?.lineage_thinking_session_id,
    ),
    knowledge: firstNonEmptyId(
      focusedNodeProvenanceDetail.value?.lineageMemoryNoteId,
      focusedNodeProvenanceDetail.value?.lineage_memory_note_id,
    ),
    parentWorkflowId: firstNonEmptyId(
      focusedNodeProvenanceDetail.value?.parentWorkflowId,
      focusedNodeProvenanceDetail.value?.parent_workflow_id,
    ),
  }, {
    thinking: currentThinkingLabel.value,
  }));
const provenanceLinks = computed(() =>
  thinkingLineageLinks.value
    .filter((link) => link.kind !== 'thinking' && link.id !== currentThinkingSessionId.value));

function renderMarkdown(content: string): string {
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}

function thoughtKey(thought: NonNullable<SequentialThinkingViewNode['thinking_trace']>[number], index: number): string {
  return `${thought.thought_number}-${index}`;
}

const openThoughts = ref<Record<string, boolean>>({});

watch(
  () => props.trace,
  (trace) => {
    const next: Record<string, boolean> = {};
    trace.forEach((thought, index) => {
      const key = thoughtKey(thought, index);
      next[key] = openThoughts.value[key] ?? true;
    });
    openThoughts.value = next;
  },
  { immediate: true },
);

function isThoughtOpen(thought: NonNullable<SequentialThinkingViewNode['thinking_trace']>[number], index: number): boolean {
  return openThoughts.value[thoughtKey(thought, index)] ?? true;
}

function setThoughtOpen(
  thought: NonNullable<SequentialThinkingViewNode['thinking_trace']>[number],
  index: number,
  open: boolean,
): void {
  openThoughts.value[thoughtKey(thought, index)] = open;
}

function onThoughtToggle(
  thought: NonNullable<SequentialThinkingViewNode['thinking_trace']>[number],
  index: number,
  event: Event,
): void {
  const target = event.target;
  if (target instanceof HTMLDetailsElement) {
    setThoughtOpen(thought, index, target.open);
  }
}

const allExpanded = computed(() =>
  props.trace.length > 0 && props.trace.every((thought, index) => isThoughtOpen(thought, index)));

function toggleAll(): void {
  const expand = !allExpanded.value;
  const next: Record<string, boolean> = {};
  props.trace.forEach((thought, index) => {
    next[thoughtKey(thought, index)] = expand;
  });
  openThoughts.value = next;
}
</script>

<template>
  <section class="trace-panel" aria-label="Thinking trace">
    <ProvenanceBreadcrumbs
      current-kind="thinking"
      :current-id="currentThinkingSessionId"
      :current-label="currentThinkingLabel"
      :links="provenanceLinks"
      @navigate="navigateTo"
    />
    <div class="trace-header">
      <h3>Thinking trace</h3>
      <span class="trace-count">{{ props.trace.length }} thoughts</span>
      <button type="button" class="trace-toggle" @click="toggleAll">
        {{ allExpanded ? 'Collapse all' : 'Expand all' }}
      </button>
    </div>

    <ol class="trace-list">
      <li v-for="(thought, index) in props.trace" :key="`${thought.thought_number}-${index}`">
        <details
          class="trace-card"
          :open="isThoughtOpen(thought, index)"
          @toggle="onThoughtToggle(thought, index, $event)"
        >
          <summary class="trace-summary">
            <div class="trace-summary__copy">
              <span class="trace-number">Thought {{ thought.thought_number }}</span>
              <span v-if="thought.is_revision" class="trace-revision">
                Revises {{ thought.revises_thought ? `#${thought.revises_thought}` : 'an earlier thought' }}
              </span>
            </div>
          </summary>

          <div class="trace-body">
            <div class="trace-content" v-html="renderMarkdown(thought.text)" />
          </div>
        </details>
      </li>
    </ol>
  </section>
</template>
