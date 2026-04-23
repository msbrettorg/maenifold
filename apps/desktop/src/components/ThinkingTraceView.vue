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
  <section class="trace-panel flex flex-col gap-[var(--space-3)]" aria-label="Thinking trace">
    <ProvenanceBreadcrumbs
      current-kind="thinking"
      :current-id="currentThinkingSessionId"
      :current-label="currentThinkingLabel"
      :links="provenanceLinks"
      @navigate="navigateTo"
    />
    <div class="trace-header flex items-start justify-between gap-[var(--space-3)]">
      <h3 class="m-0 text-[length:var(--text-body-sm)] text-[var(--foreground)]">Thinking trace</h3>
      <span class="trace-count text-[var(--muted-foreground)] text-[length:var(--text-tiny)]">{{ props.trace.length }} thoughts</span>
      <Button variant="ghost" size="sm" class="trace-toggle" @click="toggleAll">
        {{ allExpanded ? 'Collapse all' : 'Expand all' }}
      </Button>
    </div>

    <Accordion
      v-model="openThoughtIds"
      type="multiple"
      class="trace-list list-none m-0 p-0 flex flex-col gap-[var(--space-2)]"
      as="ol"
    >
      <AccordionItem
        v-for="(thought, index) in props.trace"
        :key="`${thought.thought_number}-${index}`"
        :value="thoughtKey(thought, index)"
        class="trace-card border border-[var(--border)] rounded-[var(--radius-md)] bg-[var(--muted)] overflow-hidden"
        as="li"
      >
        <AccordionTrigger class="trace-summary list-none cursor-pointer p-[var(--space-3)] flex items-start justify-between gap-[var(--space-3)]">
          <div class="trace-summary__copy flex flex-col gap-[var(--space-1)] min-w-0">
            <span class="trace-number font-[var(--font-code)] text-[length:var(--text-caption)] text-[var(--foreground)]">Thought {{ thought.thought_number }}</span>
            <span
              v-if="thought.is_revision"
              class="trace-revision text-[var(--muted-foreground)] text-[length:var(--text-tiny)]"
            >
              Revises {{ thought.revises_thought ? `#${thought.revises_thought}` : 'an earlier thought' }}
            </span>
          </div>
        </AccordionTrigger>
        <AccordionContent>
          <div class="trace-body px-[var(--space-3)] pb-[var(--space-3)] border-t border-[var(--border)]">
            <div
              class="trace-content text-[var(--foreground)] text-[length:var(--text-caption)] leading-[1.6] [overflow-wrap:anywhere]"
              v-html="renderMarkdown(thought.text)"
            />
          </div>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  </section>
</template>

<!-- audit-ok: deep-third-party (markdown rendered via v-html requires :deep() to style sanitized descendants) -->
<style scoped>
.trace-content :deep(*) { margin-top: 0; }
.trace-content :deep(p:last-child) { margin-bottom: 0; }
.trace-content :deep(code) {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  background: var(--background);
  border-radius: var(--radius-sm);
  padding: 0.1rem 0.3rem;
}
.trace-content :deep(pre) {
  margin: var(--space-2) 0 0;
  padding: var(--space-2);
  background: var(--background);
  border-radius: var(--radius-sm);
  overflow-x: auto;
}
</style>
