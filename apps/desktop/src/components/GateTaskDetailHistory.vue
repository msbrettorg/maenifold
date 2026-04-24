<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import type { GateTaskHistoryEntry } from '../lib/gate';
import { formatStageLabel, gateStageFamily } from '../lib/gate-display';

const props = defineProps<{
  history: GateTaskHistoryEntry[];
  historyLoading: boolean;
  historyError: string;
}>();

const historySummaryId = computed(
  () => `gate-detail-history-summary-${props.history.length > 0 ? 'loaded' : 'empty'}`,
);

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
    : value.toLocaleString([], {
        month: 'short',
        day: 'numeric',
        hour: 'numeric',
        minute: '2-digit',
      });
}

function stageAccentClass(family: string): string {
  if (family === 'swe') return 'text-brand';
  if (family === 'red-team') return 'text-warning';
  if (family === 'blue-team' || family === 'done') return 'text-success';
  if (family === 'failed') return 'text-danger';
  return 'text-text-secondary';
}
</script>

<template>
  <section
    :aria-labelledby="historySummaryId"
    class="flex flex-col gap-3 p-3.5 border border-border-subtle rounded bg-surface-container"
  >
    <header class="flex items-center justify-between gap-3">
      <h3
        :id="historySummaryId"
        class="text-text-secondary font-mono text-xs font-medium leading-tight uppercase tracking-wide"
      >
        Gate history
      </h3>
      <span
        v-if="history.length"
        class="inline-flex items-center justify-center min-w-6 px-1.5 py-0.5 rounded bg-bg-elevated text-text-primary text-xs font-mono"
      >{{ history.length }}</span>
    </header>

    <p
      v-if="historyLoading"
      class="p-3 border border-border-subtle rounded bg-surface-container text-text-secondary text-sm"
    >Loading task history…</p>
    <p
      v-else-if="historyError"
      class="p-3 border border-border-subtle rounded bg-surface-container text-danger text-sm"
    >{{ historyError }}</p>
    <p
      v-else-if="!history.length"
      class="p-3 border border-border-subtle rounded bg-surface-container text-text-secondary text-sm"
    >No recorded gate history yet.</p>

    <Table v-else>
      <TableHeader>
        <TableRow>
          <TableHead class="w-[14%]">Stage</TableHead>
          <TableHead class="w-[18%]">Event</TableHead>
          <TableHead class="w-[14%]">Agent</TableHead>
          <TableHead class="w-[14%]">Model</TableHead>
          <TableHead class="w-[14%]">Time</TableHead>
          <TableHead>Notes</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow
          v-for="(entry, index) in history"
          :key="`${entry.timestamp ?? 'unknown'}-${entry.stage}-${entry.event}-${index}`"
          class="align-top"
        >
          <TableCell
            :class="['font-medium text-xs whitespace-nowrap', stageAccentClass(gateStageFamily(entry.stage))]"
          >
            {{ formatStageLabel(entry.stage) }}
          </TableCell>
          <TableCell class="text-text-primary text-sm">
            {{ entry.event }}
          </TableCell>
          <TableCell class="font-mono text-sm text-text-primary">
            {{ entry.agent || '—' }}
          </TableCell>
          <TableCell class="font-mono text-sm text-text-primary">
            {{ entry.model || '—' }}
          </TableCell>
          <TableCell class="text-text-secondary text-xs whitespace-nowrap">
            <time :datetime="entry.timestamp ?? undefined">
              {{ formatTimestamp(entry.timestamp) }}
            </time>
          </TableCell>
          <TableCell class="text-text-primary text-sm [overflow-wrap:anywhere]">
            <div
              v-if="entry.notes"
              class="space-y-2"
              v-html="renderMarkdown(entry.notes)"
            />
            <span v-else class="text-text-secondary">—</span>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
  </section>
</template>
