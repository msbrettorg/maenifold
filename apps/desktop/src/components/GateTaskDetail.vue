<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
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
  return props.task.location === 'done' ? `${pipeline.toUpperCase()} done` : pipeline.toUpperCase();
});

function renderMarkdown(content: string | null) {
  if (!content) return '';
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}

function formatTimestamp(timestamp: string | null) {
  if (!timestamp) return 'Unknown time';
  const value = new Date(timestamp);
  return Number.isNaN(value.getTime()) ? timestamp : value.toLocaleString();
}
</script>

<template>
  <aside class="gate-detail panel">
    <div class="gate-detail__header">
      <div>
        <p class="gate-detail__eyebrow">Task detail</p>
        <h3 v-if="task">{{ task.taskId }} {{ task.title }}</h3>
        <h3 v-else>Select a task</h3>
      </div>
      <button v-if="task" type="button" class="btn btn-ghost" @click="$emit('close')">Clear</button>
    </div>

    <template v-if="task">
      <section class="gate-detail__section">
        <p class="gate-detail__section-label">Spec</p>
        <p v-if="bodyLoading" class="gate-detail__message">Loading task detail…</p>
        <p v-else-if="bodyError" class="gate-detail__message gate-detail__message--error">{{ bodyError }}</p>
        <p v-else-if="!body.trim()" class="gate-detail__message">No task body content found.</p>
        <div v-else class="gate-detail__markdown gate-detail__body" v-html="renderMarkdown(body)" />
      </section>

      <section class="gate-detail__section">
        <p class="gate-detail__section-label">Metadata</p>
      <dl class="gate-detail__meta">
        <div>
          <dt>Process</dt>
          <dd>{{ processLabel }}</dd>
        </div>
        <div>
          <dt>State</dt>
          <dd>{{ task.state }}</dd>
        </div>
        <div>
          <dt>Location</dt>
          <dd>{{ task.location }}</dd>
        </div>
        <div>
          <dt>Attempts</dt>
          <dd>{{ task.attempts }}</dd>
        </div>
        <div>
          <dt>Canonical ID</dt>
          <dd>{{ task.canonicalTaskId || '—' }}</dd>
        </div>
        <div class="gate-detail__path">
          <dt>Path</dt>
          <dd>{{ task.path }}</dd>
        </div>
      </dl>
      </section>

      <p v-if="task.issue" class="gate-detail__issue">{{ task.issue }}</p>

      <section class="gate-detail__section">
        <p class="gate-detail__section-label">History</p>
        <p v-if="historyLoading" class="gate-detail__message">Loading task history…</p>
        <p v-else-if="historyError" class="gate-detail__message gate-detail__message--error">{{ historyError }}</p>
        <p v-else-if="!history.length" class="gate-detail__message">No recorded gate history yet.</p>
        <ol v-else class="gate-detail__history">
          <li v-for="(entry, index) in history" :key="`${entry.timestamp ?? 'unknown'}-${entry.stage}-${entry.event}-${index}`" class="gate-detail__history-item">
            <div class="gate-detail__history-header">
              <div>
                <p class="gate-detail__history-stage">{{ entry.stage }}</p>
                <h4>{{ entry.event }}</h4>
              </div>
              <time class="gate-detail__history-time" :datetime="entry.timestamp ?? undefined">{{ formatTimestamp(entry.timestamp) }}</time>
            </div>
            <dl class="gate-detail__history-meta">
              <div>
                <dt>Agent</dt>
                <dd>{{ entry.agent || '—' }}</dd>
              </div>
              <div>
                <dt>Model</dt>
                <dd>{{ entry.model || '—' }}</dd>
              </div>
            </dl>
            <div v-if="entry.notes" class="gate-detail__markdown" v-html="renderMarkdown(entry.notes)" />
          </li>
        </ol>
      </section>
    </template>

    <p v-else class="gate-detail__message">
      Select a task from the table to inspect its gate history, path, and gate metadata.
    </p>
  </aside>
</template>

<style scoped>
.gate-detail,
.gate-detail__meta,
.gate-detail__meta > div {
  display: flex;
  flex-direction: column;
}

.gate-detail {
  gap: 1rem;
  flex: 1;
  min-width: 0;
  overflow: auto;
}

.gate-detail__header,
.gate-detail__section,
.gate-detail__meta,
.gate-detail__history,
.gate-detail__history-item,
.gate-detail__history-meta {
  gap: 0.75rem;
}

.gate-detail__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.gate-detail__eyebrow,
.gate-detail__section-label,
.gate-detail__meta dt,
.gate-detail__message {
  color: var(--text-muted);
}

.gate-detail__eyebrow {
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.gate-detail__section,
.gate-detail__section-label {
  display: flex;
  flex-direction: column;
}

.gate-detail__section-label {
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.gate-detail__meta {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.gate-detail__meta dt {
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.gate-detail__meta dd,
.gate-detail__path dd {
  margin-top: 0.25rem;
  word-break: break-word;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.gate-detail__path {
  grid-column: 1 / -1;
}

.gate-detail__issue,
.gate-detail__message {
  padding: 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}

.gate-detail__message--error,
.gate-detail__issue {
  color: var(--color-status-abandoned);
}

.gate-detail__history {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  list-style: none;
}

.gate-detail__history-item {
  display: flex;
  flex-direction: column;
  padding: 0.9rem;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}

.gate-detail__history-header,
.gate-detail__history-meta {
  display: flex;
  gap: 0.75rem;
  justify-content: space-between;
}

.gate-detail__history-stage,
.gate-detail__history-time,
.gate-detail__history-meta dt {
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.gate-detail__history-stage {
  margin-bottom: 0.25rem;
}

.gate-detail__history-time {
  white-space: nowrap;
}

.gate-detail__history-meta {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.gate-detail__history-meta dd {
  margin-top: 0.25rem;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.gate-detail__markdown {
  color: var(--text-primary);
  overflow-wrap: anywhere;
}

.gate-detail__body {
  padding: 0.9rem;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
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
  border-radius: 6px;
  background: var(--bg-surface);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
}

.gate-detail__markdown :deep(pre) {
  padding: 0.75rem;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
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
  border: 1px solid var(--border-default);
  text-align: left;
}

@media (max-width: 960px) {
  .gate-detail {
    min-width: 0;
  }
}
</style>
