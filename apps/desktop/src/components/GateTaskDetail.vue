<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import { Button } from '@/components/ui/button';
import GateTaskDetailHistory from './GateTaskDetailHistory.vue';
import GateTaskDetailMeta from './GateTaskDetailMeta.vue';
import GateTaskDetailStages from './GateTaskDetailStages.vue';
import type { GateTask, GateTaskHistoryEntry } from '../lib/gate';
import { getTaskPipelineLabel, isTaskPipelineDigestStale } from '../lib/gate-display';

const props = defineProps<{
  task: GateTask | null;
  body: string;
  history: GateTaskHistoryEntry[];
  bodyLoading: boolean;
  historyLoading: boolean;
  bodyError: string;
  historyError: string;
}>();

defineEmits<{ close: [] }>();

const processLabel = computed(() => {
  if (!props.task) return '';
  const pipeline = getTaskPipelineLabel(props.task);
  return props.task.location === 'done' ? `${pipeline} done` : pipeline;
});

const pipelineStale = computed(() => Boolean(props.task && isTaskPipelineDigestStale(props.task)));

const detailTitleId = computed(() => (props.task ? `gate-task-detail-title-${props.task.taskId}` : 'gate-task-detail-title-empty'));

function renderMarkdown(content: string | null) {
  if (!content) return '';
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}

</script>

<template>
  <aside
    class="flex flex-col gap-4 flex-[3] min-w-0 min-h-0 overflow-hidden p-4 bg-bg-elevated border border-border-card rounded-lg"
    role="article"
    :aria-labelledby="detailTitleId"
  >
    <div class="flex justify-between items-start gap-3">
      <div class="flex flex-col min-w-0 gap-2">
        <p class="text-text-secondary text-[length:var(--text-tiny)]">Task detail</p>
        <h3 :id="detailTitleId">{{ task ? `${task.taskId} ${task.title}` : 'Select a task' }}</h3>
        <GateTaskDetailMeta v-if="task" :task="task" :process-label="processLabel" :pipeline-stale="pipelineStale" />
      </div>
      <Button v-if="task" variant="ghost" @click="$emit('close')">Clear</Button>
    </div>

    <template v-if="task">
      <div class="flex flex-col gap-4 min-h-0">
        <div class="flex flex-col gap-4 min-h-0 max-h-[min(52vh,34rem)] pr-1 overflow-auto">
          <section class="flex flex-col gap-3">
            <p class="text-text-secondary text-[length:var(--text-tiny)] flex flex-col">Spec</p>
            <p
              v-if="bodyLoading"
              class="p-3 border border-border-subtle rounded bg-surface-container text-text-secondary"
            >Loading task detail…</p>
            <p
              v-else-if="bodyError"
              class="p-3 border border-border-subtle rounded bg-surface-container text-[var(--color-status-abandoned)]"
            >{{ bodyError }}</p>
            <p
              v-else-if="!body.trim()"
              class="p-3 border border-border-subtle rounded bg-surface-container text-text-secondary"
            >No task body content found.</p>
            <div
              v-else
              class="gate-detail__markdown text-text-primary [overflow-wrap:anywhere] p-[0.9rem] border border-border-subtle rounded bg-surface-container"
              v-html="renderMarkdown(body)"
            />
          </section>

          <GateTaskDetailStages :task="task" />
          <p
            v-if="task.issue"
            class="p-3 border border-border-subtle rounded bg-surface-container text-[var(--color-status-abandoned)]"
          >{{ task.issue }}</p>
        </div>

        <GateTaskDetailHistory
          :history="history"
          :history-loading="historyLoading"
          :history-error="historyError"
        />
      </div>
    </template>

    <p
      v-else
      class="p-3 border border-border-subtle rounded bg-surface-container text-text-secondary"
    >
      Select a task from the table to inspect its gate history, path, and gate metadata.
    </p>
  </aside>
</template>

<!-- audit-ok: deep-third-party — :deep() rules style sanitized markdown injected via v-html (paragraph/list/table/code spacing). Cross-scope :deep() targets cannot be expressed as Tailwind utilities. -->
<style scoped>
.gate-detail__markdown :deep(*) {
  margin-top: 0;
}

.gate-detail__markdown :deep(p + p),
.gate-detail__markdown :deep(ul),
.gate-detail__markdown :deep(ol),
.gate-detail__markdown :deep(pre),
.gate-detail__markdown :deep(table) {
  margin-top: 0.75rem;
}

.gate-detail__markdown :deep(code) {
  padding: 0.1rem 0.3rem;
  border-radius: var(--radius);
  border: 1px solid color-mix(in srgb, var(--color-border-subtle) 80%, transparent);
  background: color-mix(in srgb, var(--color-bg-app) 75%, var(--color-bg-surface));
  color: var(--color-text-primary);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-detail__markdown :deep(pre) {
  padding: 0.75rem;
  border-radius: var(--radius-md);
  background: var(--color-bg-surface);
  overflow-x: auto;
}

.gate-detail__markdown :deep(pre code) {
  padding: 0;
  background: transparent;
}

.gate-detail__markdown :deep(table) {
  width: 100%;
  border-collapse: collapse;
}

.gate-detail__markdown :deep(th),
.gate-detail__markdown :deep(td) {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border-subtle);
  text-align: left;
}
</style>
