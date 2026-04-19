<script setup lang="ts">
import { computed } from 'vue';
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

defineProps<{
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

function formatStatus(status: string): string {
  return status
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function isRevision(status: string): boolean {
  return /revis/i.test(status);
}

function revisionTarget(status: string): number | null {
  const match = status.match(/revis(?:e|es|ed|ion)?[^\d]*(\d+)/i);
  return match ? Number(match[1]) : null;
}

function isActive(status: string): boolean {
  return /active|running|in[- ]?progress/i.test(status);
}

function statusClass(status: string): string {
  if (isRevision(status)) return 'trace-status--revision';
  if (isActive(status)) return 'trace-status--active';
  if (/complete|done/i.test(status)) return 'trace-status--completed';
  if (/stuck|abandon|fail|error/i.test(status)) return 'trace-status--warning';
  return 'trace-status--pending';
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
      <span class="trace-count">{{ trace.length }} thoughts</span>
    </div>

    <ol class="trace-list">
      <li v-for="(thought, index) in trace" :key="`${thought.thought_number}-${index}`">
        <details class="trace-card" :open="index < 3">
          <summary class="trace-summary">
            <div class="trace-summary__copy">
              <span class="trace-number">Thought {{ thought.thought_number }}</span>
              <span v-if="isRevision(thought.status)" class="trace-revision">
                Revises {{ revisionTarget(thought.status) ? `#${revisionTarget(thought.status)}` : 'an earlier thought' }}
              </span>
            </div>
            <span :class="['trace-status', statusClass(thought.status)]">
              <span v-if="isActive(thought.status)" class="trace-status__dot" aria-hidden="true" />
              {{ formatStatus(thought.status) }}
            </span>
          </summary>

          <div class="trace-body">
            <div class="trace-content" v-html="renderMarkdown(thought.text)" />
          </div>
        </details>
      </li>
    </ol>
  </section>
</template>
