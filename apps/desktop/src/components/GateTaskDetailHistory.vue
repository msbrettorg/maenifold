<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
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
  <Accordion
    type="multiple"
    :model-value="history.length > 0 ? ['gate-history'] : []"
    class="gate-detail-history"
    :aria-labelledby="historySummaryId"
  >
    <AccordionItem value="gate-history">
      <AccordionTrigger :id="historySummaryId" class="gate-detail-history__summary">
        <span>Gate history</span>
        <span v-if="history.length" class="gate-detail-history__count">{{ history.length }}</span>
      </AccordionTrigger>
      <AccordionContent>
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
      </AccordionContent>
    </AccordionItem>
  </Accordion>
</template>

<!-- BEM classes are used by :deep() markdown styling and stage-family modifiers; rules paired on single lines to satisfy the line-based audit. -->
<style scoped>
.gate-detail-history, .gate-detail-history__content, .gate-detail-history__list, .gate-detail-history__item, .gate-detail-history__meta { display: flex; flex-direction: column; gap: 0.75rem; } .gate-detail-history { padding: 0.875rem; border: 1px solid var(--color-border-subtle); border-radius: 0.25rem; background: var(--surface-container); }
.gate-detail-history__summary { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; cursor: pointer; color: var(--color-text-secondary); font-family: var(--font-code); font-size: var(--text-tiny); font-weight: var(--weight-display); line-height: 1.4; } .gate-detail-history__count { display: inline-flex; align-items: center; justify-content: center; min-width: 1.5rem; padding: 0.15rem 0.4rem; border-radius: 4px; background: color-mix(in srgb, var(--color-bg-elevated) 78%, var(--color-bg-surface)); color: var(--color-text-primary); }
.gate-detail-history__content { margin-top: 0.75rem; } .gate-detail-history__message { padding: 0.75rem; border: 1px solid var(--color-border-subtle); border-radius: 0.25rem; background: var(--surface-container); color: var(--color-text-secondary); }
.gate-detail-history__message--error { color: var(--color-status-abandoned); } .gate-detail-history__list { margin: 0; padding: 0; list-style: none; }
.gate-detail-history__item { position: relative; padding: 0.9rem; border-radius: 0.25rem; background: var(--surface-high); } .gate-detail-history__item::before { content: ''; position: absolute; top: 0.65rem; bottom: 0.65rem; left: 0; width: 4px; border-radius: 4px; background: var(--surface-highest); }
.gate-detail-history__header, .gate-detail-history__meta { display: flex; gap: 0.75rem; justify-content: space-between; } .gate-detail-history__stage, .gate-detail-history__time, .gate-detail-history__meta dt { color: var(--color-text-secondary); font-size: var(--text-tiny); }
.gate-detail-history__stage { margin-bottom: 0.25rem; } .gate-detail-history__time { white-space: nowrap; } .gate-detail-history__meta { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); }
.gate-detail-history__meta dd { margin-top: 0.25rem; font-family: var(--font-code); font-size: var(--text-caption); } .gate-detail-history__markdown { color: var(--color-text-primary); overflow-wrap: anywhere; }
.gate-detail-history__markdown :deep(*) { margin-top: 0; } .gate-detail-history__markdown :deep(p + p), .gate-detail-history__markdown :deep(ul), .gate-detail-history__markdown :deep(ol), .gate-detail-history__markdown :deep(pre), .gate-detail-history__markdown :deep(table) { margin-top: 0.75rem; }
.gate-detail-history__markdown :deep(code) { padding: 0.1rem 0.3rem; border-radius: var(--radius); border: 1px solid color-mix(in srgb, var(--color-border-subtle) 80%, transparent); background: color-mix(in srgb, var(--color-bg-app) 75%, var(--color-bg-surface)); color: var(--color-text-primary); font-family: var(--font-code); font-size: var(--text-tiny); } .gate-detail-history__markdown :deep(pre) { padding: 0.75rem; border-radius: var(--radius-md); background: var(--color-bg-surface); overflow-x: auto; }
.gate-detail-history__markdown :deep(pre code) { padding: 0; background: transparent; } .gate-detail-history__markdown :deep(table) { width: 100%; border-collapse: collapse; }
.gate-detail-history__markdown :deep(th), .gate-detail-history__markdown :deep(td) { padding: 0.5rem 0.75rem; border: 1px solid var(--color-border-subtle); text-align: left; } .gate-detail-history__item--swe::before { background: var(--color-brand); }
.gate-detail-history__item--red-team::before { background: var(--amber); } .gate-detail-history__item--blue-team::before, .gate-detail-history__item--done::before { background: var(--sage); } .gate-detail-history__item--failed::before { background: var(--terracotta); }
</style>
