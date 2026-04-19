<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import { Button } from '@/components/ui/button';
import GateTaskDetailHistory from './GateTaskDetailHistory.vue';
import GateTaskDetailMeta from './GateTaskDetailMeta.vue';
import GateTaskDetailStages from './GateTaskDetailStages.vue';
import type { GateTask, GateTaskHistoryEntry } from '../lib/gate';

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
  const pipeline = props.task.pipeline?.trim() || 'unbound';
  return props.task.location === 'done' ? `${pipeline} done` : pipeline;
});

const detailTitleId = computed(() => (props.task ? `gate-task-detail-title-${props.task.taskId}` : 'gate-task-detail-title-empty'));

function renderMarkdown(content: string | null) {
  if (!content) return '';
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}

</script>

<template>
  <aside class="gate-detail" role="article" :aria-labelledby="detailTitleId">
    <div class="gate-detail__header">
      <div class="gate-detail__header-copy">
        <p class="gate-detail__eyebrow">Task detail</p>
        <h3 :id="detailTitleId">{{ task ? `${task.taskId} ${task.title}` : 'Select a task' }}</h3>
        <GateTaskDetailMeta v-if="task" :task="task" :process-label="processLabel" />
      </div>
      <Button v-if="task" variant="ghost" @click="$emit('close')">Clear</Button>
    </div>

    <template v-if="task">
      <div class="gate-detail__content">
        <div class="gate-detail__body-scroll">
          <section class="gate-detail__section">
            <p class="gate-detail__section-label">Spec</p>
            <p v-if="bodyLoading" class="gate-detail__message">Loading task detail…</p>
            <p v-else-if="bodyError" class="gate-detail__message gate-detail__message--error">{{ bodyError }}</p>
            <p v-else-if="!body.trim()" class="gate-detail__message">No task body content found.</p>
            <div v-else class="gate-detail__markdown gate-detail__body" v-html="renderMarkdown(body)" />
          </section>

          <GateTaskDetailStages :task="task" />
          <p v-if="task.issue" class="gate-detail__issue">{{ task.issue }}</p>
        </div>

        <GateTaskDetailHistory
          :history="history"
          :history-loading="historyLoading"
          :history-error="historyError"
        />
      </div>
    </template>

    <p v-else class="gate-detail__message">
      Select a task from the table to inspect its gate history, path, and gate metadata.
    </p>
  </aside>
</template>

<style scoped>
.gate-detail,
.gate-detail__header-copy,
.gate-detail__content,
.gate-detail__body-scroll {
  display: flex;
  flex-direction: column;
}

.gate-detail {
  gap: 1rem;
  flex: 3;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  padding: 1rem;
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-card);
  border-radius: 8px;
}

.gate-detail__header,
.gate-detail__section {
  gap: 0.75rem;
}

.gate-detail__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.gate-detail__header-copy {
  min-width: 0;
  gap: 0.5rem;
}

.gate-detail__eyebrow,
.gate-detail__section-label,
.gate-detail__message {
  color: var(--muted-foreground);
}

.gate-detail__eyebrow {
  font-size: var(--text-tiny);
}

.gate-detail__content {
  gap: 1rem;
  min-height: 0;
}

.gate-detail__body-scroll {
  gap: 1rem;
  min-height: 0;
  max-height: min(52vh, 34rem);
  padding-right: 0.25rem;
  overflow: auto;
}

.gate-detail__section,
.gate-detail__section-label {
  display: flex;
  flex-direction: column;
}

.gate-detail__section-label {
  font-size: var(--text-tiny);
}

.gate-detail__issue,
.gate-detail__message {
  padding: 0.75rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: var(--surface-container);
}

.gate-detail__message--error,
.gate-detail__issue {
  color: var(--color-status-abandoned);
}

.gate-detail__markdown {
  color: var(--foreground);
  overflow-wrap: anywhere;
}

.gate-detail__body {
  padding: 0.9rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: var(--surface-container);
}

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
  border: 1px solid color-mix(in srgb, var(--border) 80%, transparent);
  background: color-mix(in srgb, var(--background) 75%, var(--muted));
  color: var(--foreground);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-detail__markdown :deep(pre) {
  padding: 0.75rem;
  border-radius: var(--radius-md);
  background: var(--muted);
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
  border: 1px solid var(--border);
  text-align: left;
}

@media (max-width: 960px) {
  .gate-detail {
    min-width: 0;
  }
}
</style>
