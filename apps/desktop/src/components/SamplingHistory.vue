<script setup lang="ts">
import { onMounted } from 'vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { useSamplingHistoryStore } from '../stores/sampling-history';
import type { SamplingHistoryEntry } from '../stores/sampling-history';

const store = useSamplingHistoryStore();

onMounted(() => {
  if (typeof store.init === 'function') {
    void store.init();
  }
});

function formatTime(createdAt: string): string {
  const normalized = createdAt.replace(/\.(\d{3})\d+([+-]\d{2}:\d{2}|Z)$/, '.$1$2');
  const date = new Date(normalized);
  if (Number.isNaN(date.getTime())) {
    return createdAt;
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'short',
    timeStyle: 'medium',
  }).format(date);
}

function formatTokens(entry: SamplingHistoryEntry): string {
  if (entry.tokenUsage === null) {
    return 'not reported';
  }
  return `${entry.tokenUsage.inputTokens} / ${entry.tokenUsage.outputTokens}`;
}

function budgetClass(entry: SamplingHistoryEntry): string {
  const used = entry.toolCallCount ?? 0;
  const budget = entry.toolLoopBudget ?? 10;
  if (entry.status === 'budgetExceeded') return 'bg-destructive/10 text-destructive';
  if (used >= budget * 0.8) return 'bg-warning/10 text-warning';
  return '';
}
</script>

<template>
  <section class="flex flex-col gap-3">
    <header class="flex flex-wrap items-start justify-between gap-3">
      <div class="flex flex-col gap-1">
        <h2 class="text-lg font-semibold text-text-primary">Sampling history</h2>
        <p class="text-sm text-text-secondary">
          Read-only mirror of recent MCP sampling dispatch sessions.
        </p>
      </div>
      <Button type="button" variant="outline" @click="store.refresh">
        Refresh
      </Button>
    </header>

    <div
      v-if="store.entries.length === 0"
      class="rounded-lg border border-border-subtle bg-bg-elevated p-4 text-sm text-text-secondary"
    >
      No sampling history.
    </div>

    <div v-else class="rounded-lg border border-border-subtle bg-bg-elevated">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Time</TableHead>
            <TableHead>Server</TableHead>
            <TableHead>Model</TableHead>
            <TableHead>Decision</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Tool calls</TableHead>
            <TableHead>Tokens</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow v-for="entry in store.entries" :key="entry.samplingRequestId">
            <TableCell class="text-xs text-text-secondary">
              {{ formatTime(entry.createdAt) }}
            </TableCell>
            <TableCell class="font-medium text-text-primary">
              {{ entry.serverName }}
            </TableCell>
            <TableCell class="font-mono text-xs text-text-primary">
              {{ entry.model }}
            </TableCell>
            <TableCell class="text-text-secondary">
              {{ entry.decision }}
            </TableCell>
            <TableCell class="text-text-secondary">
              {{ entry.status }}
            </TableCell>
            <TableCell>
              <Badge variant="secondary" :class="budgetClass(entry)">
                Tool calls: {{ entry.toolCallCount ?? 0 }}/{{ entry.toolLoopBudget ?? 10 }}
              </Badge>
            </TableCell>
            <TableCell class="font-mono text-xs text-text-secondary">
              {{ formatTokens(entry) }}
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </div>
  </section>
</template>
