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
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
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
  <section class="flex flex-col gap-3" aria-label="Thinking trace">
    <ProvenanceBreadcrumbs
      current-kind="thinking"
      :current-id="currentThinkingSessionId"
      :current-label="currentThinkingLabel"
      :links="provenanceLinks"
      @navigate="navigateTo"
    />

    <Card class="bg-bg-surface border-border-default">
      <CardHeader class="flex flex-row items-center justify-between gap-3 pb-3">
        <div class="flex items-center gap-2 min-w-0">
          <h3 class="m-0 text-sm font-medium text-text-primary truncate">
            Thinking trace
          </h3>
          <Badge variant="secondary" class="shrink-0">
            {{ props.trace.length }} thoughts
          </Badge>
        </div>
        <Button
          variant="ghost"
          size="sm"
          class="shrink-0"
          @click="toggleAll"
        >
          {{ allExpanded ? 'Collapse all' : 'Expand all' }}
        </Button>
      </CardHeader>

      <CardContent class="pt-0">
        <ScrollArea class="max-h-[60vh] pr-3">
          <Accordion
            v-model="openThoughtIds"
            type="multiple"
            class="list-none m-0 p-0 flex flex-col gap-2"
            as="ol"
          >
            <AccordionItem
              v-for="(thought, index) in props.trace"
              :key="`${thought.thought_number}-${index}`"
              :value="thoughtKey(thought, index)"
              class="border border-border-default rounded-md bg-bg-elevated overflow-hidden"
              as="li"
            >
              <AccordionTrigger
                class="px-3 py-2 hover:no-underline flex items-start justify-between gap-3"
              >
                <div class="flex flex-col gap-1 min-w-0 text-left">
                  <span class="font-mono text-xs text-text-primary">
                    Thought {{ thought.thought_number }}
                  </span>
                  <span
                    v-if="thought.is_revision"
                    class="text-text-muted text-xs"
                  >
                    Revises
                    {{ thought.revises_thought ? `#${thought.revises_thought}` : 'an earlier thought' }}
                  </span>
                </div>
              </AccordionTrigger>
              <AccordionContent>
                <div class="px-3 pb-3 border-t border-border-default">
                  <div
                    class="text-text-primary text-sm leading-relaxed pt-3 [overflow-wrap:anywhere] [&_p]:m-0 [&_p+p]:mt-2 [&_pre]:mt-2 [&_pre]:p-2 [&_pre]:rounded [&_pre]:bg-bg-code [&_pre]:overflow-x-auto [&_code]:font-mono [&_code]:text-xs [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded [&_code]:bg-bg-code [&_pre_code]:bg-transparent [&_pre_code]:p-0"
                    v-html="renderMarkdown(thought.text)"
                  />
                </div>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        </ScrollArea>
      </CardContent>
    </Card>
  </section>
</template>
