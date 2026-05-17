<script setup lang="ts">
import { Square } from 'lucide-vue-next';
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
import type { SamplingHistoryEntry } from '../lib/types';

defineProps<{
  entries: SamplingHistoryEntry[];
}>();

const emit = defineEmits<{
  stop: [samplingRequestId: string];
}>();

function formatTime(createdAt: string): string {
  const normalized = createdAt.replace(/\.(\d{3})\d+([+-]\d{2}:\d{2}|Z)$/, '.$1$2');
  const date = new Date(normalized);
  if (Number.isNaN(date.getTime())) return createdAt;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'short',
    timeStyle: 'medium',
  }).format(date);
}

function statusLabel(status: string): string {
  return status
    .replace(/([A-Z])/g, ' $1')
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function eventTypeLabel(entry: SamplingHistoryEntry): string {
  if (entry.decision === 'clientSampling') return 'Client sampling';
  if (entry.decision === 'inferencing') return 'Inferencing';
  if (entry.decision === 'allowOnce' || entry.decision === 'allowAlways') return 'MCP sampling';
  return statusLabel(entry.decision || 'unknown');
}

function payloadPreview(payload: string | undefined): string {
  if (!payload) return '—';
  const trimmed = payload.trim();
  if (!trimmed) return '—';
  const limit = 180;
  return trimmed.length > limit ? `${trimmed.slice(0, limit)}…` : trimmed;
}

function isActive(entry: SamplingHistoryEntry): boolean {
  return entry.status === 'dispatching' || entry.status === 'streaming' || entry.status === 'budgetExceeded';
}

function responsePreview(entry: SamplingHistoryEntry): string {
  if (entry.currentDelta && isActive(entry)) return payloadPreview(entry.currentDelta);
  return payloadPreview(entry.responsePayload);
}

function formatTokens(entry: SamplingHistoryEntry): string {
  if (entry.tokenUsage === null) return 'not reported';
  return `${entry.tokenUsage.inputTokens} / ${entry.tokenUsage.outputTokens}`;
}

function showToolBudget(entry: SamplingHistoryEntry): boolean {
  return (entry.toolLoopBudget ?? 0) > 0;
}

function budgetClass(entry: SamplingHistoryEntry): string {
  const used = entry.toolCallCount ?? 0;
  const budget = entry.toolLoopBudget ?? 10;
  if (entry.status === 'budgetExceeded') return 'bg-destructive/10 text-destructive';
  if (budget > 0 && used >= budget * 0.8) return 'bg-warning/10 text-warning';
  return '';
}
</script>

<template>
  <div class="overflow-x-auto rounded-lg border border-border-subtle bg-bg-elevated">
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Time</TableHead>
          <TableHead>Event type</TableHead>
          <TableHead>Source</TableHead>
          <TableHead>Request</TableHead>
          <TableHead>Response</TableHead>
          <TableHead>Status</TableHead>
          <TableHead>Tokens</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow v-if="entries.length === 0">
          <TableCell :colspan="7" class="py-6 text-center text-text-secondary">
            No GatewayIntent inference or MCP sampling requests recorded yet.
          </TableCell>
        </TableRow>
        <TableRow v-for="entry in entries" :key="entry.samplingRequestId">
          <TableCell class="whitespace-nowrap text-xs text-text-secondary">
            {{ formatTime(entry.createdAt) }}
          </TableCell>
          <TableCell class="align-top">
            <Badge variant="secondary" class="w-fit bg-info/10 text-info">
              {{ eventTypeLabel(entry) }}
            </Badge>
          </TableCell>
          <TableCell class="min-w-28">
            <div class="flex flex-col gap-1">
              <span class="font-medium text-text-primary">{{ entry.serverName }}</span>
              <span class="font-mono text-tiny text-text-secondary">{{ entry.runtimeKind }}</span>
            </div>
          </TableCell>
          <TableCell class="min-w-48 max-w-72 align-top">
            <pre class="m-0 max-h-28 overflow-auto whitespace-pre-wrap break-words rounded bg-bg-surface p-2 font-mono text-tiny text-text-secondary">{{ payloadPreview(entry.requestPayload) }}</pre>
          </TableCell>
          <TableCell class="min-w-48 max-w-72 align-top">
            <div class="flex flex-col gap-1">
              <Badge v-if="entry.currentDelta && isActive(entry)" variant="secondary" class="w-fit bg-info/10 text-info">
                Live stream
              </Badge>
              <pre class="m-0 max-h-28 overflow-auto whitespace-pre-wrap break-words rounded bg-bg-surface p-2 font-mono text-tiny text-text-secondary">{{ responsePreview(entry) }}</pre>
            </div>
          </TableCell>
          <TableCell class="align-top text-text-secondary">
            <div class="flex flex-col gap-1">
              <span>{{ statusLabel(entry.status) }}</span>
              <Badge v-if="showToolBudget(entry)" data-testid="tool-budget-badge" variant="secondary" :class="['w-fit', budgetClass(entry)]">
                Tool calls: {{ entry.toolCallCount ?? 0 }}/{{ entry.toolLoopBudget ?? 10 }}
              </Badge>
              <Button
                v-if="isActive(entry)"
                type="button"
                variant="ghost"
                size="sm"
                class="w-fit gap-1 text-destructive hover:bg-destructive/20 hover:text-destructive"
                :aria-label="`Stop sampling session ${entry.samplingRequestId}`"
                @click="emit('stop', entry.samplingRequestId)"
              >
                <Square class="size-3 fill-current" aria-hidden="true" />
                Stop
              </Button>
            </div>
          </TableCell>
          <TableCell class="font-mono text-xs text-text-secondary">
            {{ formatTokens(entry) }}
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
  </div>
</template>
