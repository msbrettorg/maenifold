<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import type { GateTaskHistoryEntry } from '../lib/gate';
import { formatStageLabel, gateStageFamily } from '../lib/gate-display';

const props = defineProps<{
  history: GateTaskHistoryEntry[];
  historyLoading: boolean;
  historyError: string;
}>();

const historySummaryId = computed(() => `gate-detail-history-summary-${props.history.length > 0 ? 'loaded' : 'empty'}`);

function renderMarkdown(content: string | null) {
  if (!content) return '';
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}

function formatTimestamp(timestamp: string | null) {
  if (!timestamp) return 'Unknown time';
  const value = new Date(timestamp);
  return Number.isNaN(value.getTime())
    ? timestamp
    : value.toLocaleString([], { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' });
}
</script>

<template>
  <details class="gate-detail-history" :open="history.length > 0" :aria-labelledby="historySummaryId">
    <summary :id="historySummaryId" class="gate-detail-history__summary">
      <span>Gate history</span>
      <span v-if="history.length" class="gate-detail-history__count">{{ history.length }}</span>
    </summary>
    <div class="gate-detail-history__content">
      <p v-if="historyLoading" class="gate-detail-history__message">Loading task history…</p>
      <p v-else-if="historyError" class="gate-detail-history__message gate-detail-history__message--error">{{ historyError }}</p>
      <p v-else-if="!history.length" class="gate-detail-history__message">No recorded gate history yet.</p>
      <ol v-else class="gate-detail-history__list">
        <li
          v-for="(entry, index) in history"
          :key="`${entry.timestamp ?? 'unknown'}-${entry.stage}-${entry.event}-${index}`"
          :class="['gate-detail-history__item', `gate-detail-history__item--${gateStageFamily(entry.stage)}`]"
        >
          <div class="gate-detail-history__header">
            <div>
              <p class="gate-detail-history__stage">{{ formatStageLabel(entry.stage) }}</p>
              <h4>{{ entry.event }}</h4>
            </div>
            <time class="gate-detail-history__time" :datetime="entry.timestamp ?? undefined">{{ formatTimestamp(entry.timestamp) }}</time>
          </div>
          <dl class="gate-detail-history__meta">
            <div>
              <dt>Agent</dt>
              <dd>{{ entry.agent || '—' }}</dd>
            </div>
            <div>
              <dt>Model</dt>
              <dd>{{ entry.model || '—' }}</dd>
            </div>
          </dl>
          <div v-if="entry.notes" class="gate-detail-history__markdown" v-html="renderMarkdown(entry.notes)" />
        </li>
      </ol>
    </div>
  </details>
</template>

<style scoped>
.gate-detail-history,
.gate-detail-history__content,
.gate-detail-history__list,
.gate-detail-history__item,
.gate-detail-history__meta {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.gate-detail-history {
  padding: 0.875rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: #201f1f;
}

.gate-detail-history__summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  cursor: pointer;
  color: var(--muted-foreground);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  font-weight: var(--weight-display);
  line-height: 1.4;
}

.gate-detail-history__count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5rem;
  padding: 0.15rem 0.4rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--card) 78%, var(--muted));
  color: var(--foreground);
}

.gate-detail-history__content {
  margin-top: 0.75rem;
}

.gate-detail-history__message {
  padding: 0.75rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
  background: #201f1f;
  color: var(--muted-foreground);
}

.gate-detail-history__message--error {
  color: var(--color-status-abandoned);
}

.gate-detail-history__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.gate-detail-history__item {
  position: relative;
  padding: 0.9rem;
  border-radius: 0.25rem;
  background: #2a2a2a;
}

.gate-detail-history__item::before {
  content: '';
  position: absolute;
  top: 0.65rem;
  bottom: 0.65rem;
  left: 0;
  width: 4px;
  border-radius: 999px;
  background: #353534;
}

.gate-detail-history__header,
.gate-detail-history__meta {
  display: flex;
  gap: 0.75rem;
  justify-content: space-between;
}

.gate-detail-history__stage,
.gate-detail-history__time,
.gate-detail-history__meta dt {
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
}

.gate-detail-history__stage {
  margin-bottom: 0.25rem;
}

.gate-detail-history__time {
  white-space: nowrap;
}

.gate-detail-history__meta {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.gate-detail-history__meta dd {
  margin-top: 0.25rem;
  font-family: var(--font-code);
  font-size: var(--text-caption);
}

.gate-detail-history__markdown {
  color: var(--foreground);
  overflow-wrap: anywhere;
}

.gate-detail-history__markdown :deep(*) {
  margin-top: 0;
}

.gate-detail-history__markdown :deep(p + p),
.gate-detail-history__markdown :deep(ul),
.gate-detail-history__markdown :deep(ol),
.gate-detail-history__markdown :deep(pre),
.gate-detail-history__markdown :deep(table) {
  margin-top: 0.75rem;
}

.gate-detail-history__markdown :deep(code) {
  padding: 0.1rem 0.3rem;
  border-radius: 6px;
  border: 1px solid color-mix(in srgb, var(--border) 80%, transparent);
  background: color-mix(in srgb, var(--background) 75%, var(--muted));
  color: var(--foreground);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-detail-history__markdown :deep(pre) {
  padding: 0.75rem;
  border-radius: var(--radius-md);
  background: var(--muted);
  overflow-x: auto;
}

.gate-detail-history__markdown :deep(pre code) {
  padding: 0;
  background: transparent;
}

.gate-detail-history__markdown :deep(table) {
  width: 100%;
  border-collapse: collapse;
}

.gate-detail-history__markdown :deep(th),
.gate-detail-history__markdown :deep(td) {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border);
  text-align: left;
}

.gate-detail-history__item--swe::before { background: #4d8eff; }
.gate-detail-history__item--red-team::before { background: #f59e0b; }
.gate-detail-history__item--blue-team::before,
.gate-detail-history__item--done::before { background: #4edea3; }
.gate-detail-history__item--failed::before { background: #ffb4ab; }
</style>
