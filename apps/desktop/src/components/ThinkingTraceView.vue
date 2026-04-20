<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Button } from '@/components/ui/button';
import type { SequentialThinkingViewNode } from '../lib/pipeline-types';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { usePipelineStore } from '../stores/pipeline';
import { useProvenanceStore } from '../stores/provenance';
import { useSemanticZoomStore } from '../stores/semantic-zoom';

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

const openThoughtIds = ref<string[]>([]);
let initialized = false;

watch(
  () => props.trace,
  (trace) => {
    if (!initialized) {
      openThoughtIds.value = trace.map((thought, index) => thoughtKey(thought, index));
      initialized = true;
      return;
    }
    const validKeys = new Set(trace.map((thought, index) => thoughtKey(thought, index)));
    const previous = openThoughtIds.value.filter((key) => validKeys.has(key));
    const newKeys = trace
      .map((thought, index) => thoughtKey(thought, index))
      .filter((key) => !openThoughtIds.value.includes(key));
    openThoughtIds.value = [...previous, ...newKeys];
  },
  { immediate: true },
);

function isThoughtOpen(thought: NonNullable<SequentialThinkingViewNode['thinking_trace']>[number], index: number): boolean {
  return openThoughtIds.value.includes(thoughtKey(thought, index));
}

const allExpanded = computed(() =>
  props.trace.length > 0 && props.trace.every((thought, index) => isThoughtOpen(thought, index)));

function toggleAll(): void {
  const expand = !allExpanded.value;
  openThoughtIds.value = expand
    ? props.trace.map((thought, index) => thoughtKey(thought, index))
    : [];
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
      <Button variant="ghost" size="sm" class="trace-toggle" @click="toggleAll">
        {{ allExpanded ? 'Collapse all' : 'Expand all' }}
      </Button>
    </div>

    <Accordion v-model="openThoughtIds" type="multiple" class="trace-list" as="ol">
      <AccordionItem
        v-for="(thought, index) in props.trace"
        :key="`${thought.thought_number}-${index}`"
        :value="thoughtKey(thought, index)"
        class="trace-card"
        as="li"
      >
        <AccordionTrigger class="trace-summary">
          <div class="trace-summary__copy">
            <span class="trace-number">Thought {{ thought.thought_number }}</span>
            <span v-if="thought.is_revision" class="trace-revision">
              Revises {{ thought.revises_thought ? `#${thought.revises_thought}` : 'an earlier thought' }}
            </span>
          </div>
        </AccordionTrigger>
        <AccordionContent>
          <div class="trace-body">
            <div class="trace-content" v-html="renderMarkdown(thought.text)" />
          </div>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  </section>
</template>

<style scoped>
.trace-panel { display: flex; flex-direction: column; gap: var(--space-3); }
.trace-header, .trace-summary { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-3); }
.trace-header h3 { margin: 0; font-size: var(--text-body-sm); color: var(--foreground); }
.trace-count, .trace-revision { color: var(--muted-foreground); font-size: var(--text-tiny); }
.trace-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--space-2); }
.trace-card {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--muted);
  overflow: hidden;
}
.trace-summary {
  list-style: none;
  cursor: pointer;
  padding: var(--space-3);
}
.trace-summary::-webkit-details-marker { display: none; }
.trace-summary__copy { display: flex; flex-direction: column; gap: var(--space-1); min-width: 0; }
.trace-number { font-family: var(--font-code); font-size: var(--text-caption); color: var(--foreground); }
.trace-status {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.2rem 0.55rem;
  border-radius: 4px;
  border: 1px solid transparent;
  font-size: var(--text-tiny);
  font-weight: var(--weight-title);
  white-space: nowrap;
}
.trace-status__dot {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: var(--radius);
  background: currentColor;
}
.trace-status--active {
  color: var(--color-status-active);
  background: color-mix(in srgb, var(--color-status-active) 12%, var(--background));
  border-color: color-mix(in srgb, var(--color-status-active) 30%, transparent);
}
.trace-status--completed {
  color: var(--color-status-completed);
  background: color-mix(in srgb, var(--color-status-completed) 12%, var(--background));
  border-color: color-mix(in srgb, var(--color-status-completed) 30%, transparent);
}
.trace-status--warning {
  color: var(--color-status-stuck);
  background: color-mix(in srgb, var(--color-status-stuck) 12%, var(--background));
  border-color: color-mix(in srgb, var(--color-status-stuck) 30%, transparent);
}
.trace-status--revision {
  color: var(--ring);
  background: color-mix(in srgb, var(--ring) 12%, var(--background));
  border-color: color-mix(in srgb, var(--ring) 30%, transparent);
}
.trace-status--pending {
  color: var(--color-status-pending);
  background: color-mix(in srgb, var(--color-status-pending) 12%, var(--background));
  border-color: color-mix(in srgb, var(--color-status-pending) 30%, transparent);
}
.trace-body {
  padding: 0 var(--space-3) var(--space-3);
  border-top: 1px solid var(--border);
}
.trace-content {
  color: var(--foreground);
  font-size: var(--text-caption);
  line-height: 1.6;
  overflow-wrap: anywhere;
}
.trace-content * { margin-top: 0; }
.trace-content p:last-child { margin-bottom: 0; }
.trace-content code {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  background: var(--background);
  border-radius: var(--radius-sm);
  padding: 0.1rem 0.3rem;
}
.trace-content pre {
  margin: var(--space-2) 0 0;
  padding: var(--space-2);
  background: var(--background);
  border-radius: var(--radius-sm);
  overflow-x: auto;
}
</style>
