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
    class="gate-detail-history flex flex-col gap-3 p-3.5 border border-border-subtle rounded bg-surface-container"
    :aria-labelledby="historySummaryId"
  >
    <AccordionItem value="gate-history">
      <AccordionTrigger
        :id="historySummaryId"
        class="flex items-center justify-between gap-3 cursor-pointer text-text-secondary font-mono text-xs font-medium leading-tight"
      >
        <span>Gate history</span>
        <span
          v-if="history.length"
          class="inline-flex items-center justify-center min-w-6 px-1.5 py-0.5 rounded bg-bg-elevated text-text-primary"
        >{{ history.length }}</span>
      </AccordionTrigger>
      <AccordionContent>
        <div class="flex flex-col gap-3 mt-3">
          <p
            v-if="historyLoading"
            class="p-3 border border-border-subtle rounded bg-surface-container text-text-secondary"
          >Loading task history…</p>
          <p
            v-else-if="historyError"
            class="p-3 border border-border-subtle rounded bg-surface-container text-danger"
          >{{ historyError }}</p>
          <p
            v-else-if="!history.length"
            class="p-3 border border-border-subtle rounded bg-surface-container text-text-secondary"
          >No recorded gate history yet.</p>
          <ol v-else class="m-0 p-0 list-none flex flex-col gap-3">
            <li
              v-for="(entry, index) in history"
              :key="`${entry.timestamp ?? 'unknown'}-${entry.stage}-${entry.event}-${index}`"
              :class="[
                'gate-detail-history__item relative p-3.5 rounded bg-surface-high flex flex-col gap-3',
                `gate-detail-history__item--${gateStageFamily(entry.stage)}`,
              ]"
            >
              <div class="flex justify-between gap-3">
                <div>
                  <p class="mb-1 text-text-secondary text-xs">{{ formatStageLabel(entry.stage) }}</p>
                  <h4>{{ entry.event }}</h4>
                </div>
                <time
                  class="whitespace-nowrap text-text-secondary text-xs"
                  :datetime="entry.timestamp ?? undefined"
                >{{ formatTimestamp(entry.timestamp) }}</time>
              </div>
              <dl class="grid grid-cols-2 gap-3">
                <div>
                  <dt class="text-text-secondary text-xs">Agent</dt>
                  <dd class="mt-1 font-mono text-sm">{{ entry.agent || '—' }}</dd>
                </div>
                <div>
                  <dt class="text-text-secondary text-xs">Model</dt>
                  <dd class="mt-1 font-mono text-sm">{{ entry.model || '—' }}</dd>
                </div>
              </dl>
              <div
                v-if="entry.notes"
                class="gate-detail-history__markdown text-text-primary [overflow-wrap:anywhere]"
                v-html="renderMarkdown(entry.notes)"
              />
            </li>
          </ol>
        </div>
      </AccordionContent>
    </AccordionItem>
  </Accordion>
</template>

<!-- audit-ok: max-style-rules — remaining rules are stage-family ::before pseudo-element accents and :deep() rules styling sanitized markdown injected via v-html. Pseudo-element content/positioning and cross-scope :deep() targets cannot be expressed as Tailwind utilities. -->
<style scoped>
.gate-detail-history__item::before {
  content: '';
  position: absolute;
  top: 0.65rem;
  bottom: 0.65rem;
  left: 0;
  width: 4px;
  border-radius: 4px;
  background: var(--surface-highest);
}

.gate-detail-history__item--swe::before {
  background: var(--color-brand);
}

.gate-detail-history__item--red-team::before {
  background: var(--amber);
}

.gate-detail-history__item--blue-team::before,
.gate-detail-history__item--done::before {
  background: var(--sage);
}

.gate-detail-history__item--failed::before {
  background: var(--terracotta);
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
  border-radius: var(--radius);
  border: 1px solid color-mix(in srgb, var(--color-border-subtle) 80%, transparent);
  background: color-mix(in srgb, var(--color-bg-app) 75%, var(--color-bg-surface));
  color: var(--color-text-primary);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-detail-history__markdown :deep(pre) {
  padding: 0.75rem;
  border-radius: var(--radius-md);
  background: var(--color-bg-surface);
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
  border: 1px solid var(--color-border-subtle);
  text-align: left;
}
</style>
