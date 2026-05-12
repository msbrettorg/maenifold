<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Button } from '@/components/ui/button';
import MarkdownView from './MarkdownView.vue';
import type { GateTaskHistoryEntry } from '../lib/gate';
import { formatStageLabel, gateStageFamily } from '../lib/gate-display';

const props = defineProps<{
  history: GateTaskHistoryEntry[];
  historyLoading: boolean;
  historyError: string;
}>();

const sortDesc = ref(true);
const hideAppend = ref(false);

const historySummaryId = computed(
  () => `gate-detail-history-summary-${props.history.length > 0 ? 'loaded' : 'empty'}`,
);

const filteredHistory = computed(() => {
  const entries = hideAppend.value
    ? props.history.filter((e) => e.event.toLowerCase() !== 'append')
    : props.history.slice();
  return sortDesc.value ? entries.slice().reverse() : entries;
});

const groupedByStage = computed(() => {
  const groups = new Map<string, GateTaskHistoryEntry[]>();
  for (const entry of filteredHistory.value) {
    if (!groups.has(entry.stage)) groups.set(entry.stage, []);
    groups.get(entry.stage)!.push(entry);
  }
  return Array.from(groups.entries()).map(([stage, entries]) => ({ stage, entries }));
});

function formatTimestamp(timestamp: string | null) {
  if (!timestamp) return 'Unknown time';
  const value = new Date(timestamp);
  return Number.isNaN(value.getTime())
    ? timestamp
    : value.toLocaleString([], { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' });
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
      <div class="flex items-center gap-2">
        <span
          v-if="history.length"
          class="inline-flex items-center justify-center min-w-6 px-1.5 py-0.5 rounded bg-bg-elevated text-text-primary text-xs font-mono"
        >{{ history.length }}</span>
        <Button
          v-if="history.length"
          type="button"
          variant="ghost"
          size="sm"
          class="text-xs h-6 px-2"
          :aria-pressed="hideAppend"
          data-testid="gate-history-toggle-append"
          @click="hideAppend = !hideAppend"
        >{{ hideAppend ? 'Show all' : 'Hide appends' }}</Button>
        <Button
          v-if="history.length"
          type="button"
          variant="ghost"
          size="sm"
          class="text-xs h-6 px-2"
          :aria-pressed="!sortDesc"
          data-testid="gate-history-toggle-sort"
          @click="sortDesc = !sortDesc"
        >{{ sortDesc ? 'Newest first' : 'Oldest first' }}</Button>
      </div>
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
      v-else-if="!filteredHistory.length"
      class="p-3 border border-border-subtle rounded bg-surface-container text-text-secondary text-sm"
    >{{ history.length ? 'No matching history entries.' : 'No recorded gate history yet.' }}</p>

    <div v-else class="flex flex-col gap-2">
      <details
        v-for="group in groupedByStage"
        :key="group.stage"
        open
        class="border border-border-subtle rounded"
        :data-testid="`gate-history-stage-${group.stage}`"
      >
        <summary
          :class="['flex items-center gap-2 px-3 py-2 cursor-pointer text-xs font-medium uppercase tracking-wide select-none', stageAccentClass(gateStageFamily(group.stage))]"
        >
          {{ formatStageLabel(group.stage) }}
          <span class="ml-auto text-text-secondary font-mono">{{ group.entries.length }}</span>
        </summary>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead class="w-[18%]">Event</TableHead>
              <TableHead class="w-[14%]">Agent</TableHead>
              <TableHead class="w-[14%]">Model</TableHead>
              <TableHead class="w-[14%]">Time</TableHead>
              <TableHead>Notes</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow
              v-for="(entry, index) in group.entries"
              :key="`${entry.timestamp ?? 'unknown'}-${entry.event}-${index}`"
              class="align-top"
            >
              <TableCell class="text-text-primary text-sm">{{ entry.event }}</TableCell>
              <TableCell class="font-mono text-sm text-text-primary">{{ entry.agent || '—' }}</TableCell>
              <TableCell class="font-mono text-sm text-text-primary">{{ entry.model || '—' }}</TableCell>
              <TableCell class="text-text-secondary text-xs whitespace-nowrap">
                <time :datetime="entry.timestamp ?? undefined">{{ formatTimestamp(entry.timestamp) }}</time>
              </TableCell>
              <TableCell class="text-text-primary text-sm [overflow-wrap:anywhere]">
                <MarkdownView v-if="entry.notes" :content="entry.notes" class="text-text-primary" />
                <span v-else class="text-text-secondary">—</span>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </details>
    </div>
  </section>
</template>
