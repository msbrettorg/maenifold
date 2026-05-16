<script setup lang="ts">
import type { PipelineRunSummary } from '../lib/pipeline-types-view';

defineProps<{ runs: PipelineRunSummary[] }>();

const STATUS_CHIP: Record<string, string> = {
  pending:   'bg-status-pending/15 text-status-pending',
  active:    'bg-status-active/15 text-status-active',
  completed: 'bg-status-completed/15 text-status-completed',
  abandoned: 'bg-status-abandoned/15 text-status-abandoned',
  stuck:     'bg-status-stuck/15 text-status-stuck',
  skipped:   'bg-status-pending/15 text-status-pending',
  error:     'bg-status-abandoned/15 text-status-abandoned',
};

const OUTCOME_CHIP: Record<string, string> = {
  succeeded: 'bg-status-completed/15 text-status-completed',
  failed:    'bg-status-abandoned/15 text-status-abandoned',
  skipped:   'bg-text-secondary/15 text-text-secondary',
};

function statusChipClass(status: string): string {
  return STATUS_CHIP[status] ?? 'bg-text-secondary/15 text-text-secondary';
}

function outcomeChipClass(outcome: string): string {
  return OUTCOME_CHIP[outcome] ?? 'bg-text-secondary/15 text-text-secondary';
}

function formatRelative(ms: number | null): string {
  if (ms === null) return '—';
  const date = new Date(ms);
  if (Number.isNaN(date.getTime())) return '—';
  const minutes = Math.floor((Date.now() - date.getTime()) / 60000);
  if (minutes < 0) return date.toLocaleString();
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days === 1) return 'yesterday';
  if (days < 7) return `${days}d ago`;
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function formatDuration(startedAt: number | null, completedAt: number | null): string | null {
  if (startedAt === null || completedAt === null) return null;
  const ms = completedAt - startedAt;
  if (ms < 0) return null;
  if (ms < 1000) return `${ms}ms`;
  const secs = ms / 1000;
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const mins = Math.floor(secs / 60);
  const rem = Math.round(secs % 60);
  return `${mins}m ${rem}s`;
}
</script>

<template>
  <section
    class="flex flex-col"
    aria-label="Run history"
  >
    <h4 class="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wide text-text-tertiary">
      Run History
    </h4>

    <p
      v-if="runs.length === 0"
      class="px-3 pb-3 text-sm text-text-secondary"
      data-testid="run-history-empty"
    >
      No runs recorded yet
    </p>

    <ul
      v-else
      class="m-0 list-none flex flex-col gap-1.5 px-3 pb-3"
      data-testid="run-history-list"
    >
      <li
        v-for="(run, idx) in runs"
        :key="`${run.nodeId}-${idx}`"
        class="flex flex-col gap-1 rounded-md border border-border-subtle bg-bg-surface p-2 text-xs"
        :data-testid="`run-row-${idx}`"
      >
        <div class="flex items-center justify-between gap-2">
          <span
            class="truncate font-medium text-text-primary"
            :title="run.nodeId"
            data-testid="run-node-id"
          >
            {{ run.nodeId }}
          </span>
          <span
            class="shrink-0 rounded px-1.5 py-0.5 text-[0.65rem] font-medium leading-none"
            :class="statusChipClass(run.status)"
            :data-status="run.status"
            data-testid="run-status-chip"
          >
            {{ run.status }}
          </span>
        </div>

        <div class="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-text-secondary">
          <span
            v-if="run.startedAt !== null"
            data-testid="run-started"
          >
            Started {{ formatRelative(run.startedAt) }}
          </span>
          <span
            v-else
            data-testid="run-started-empty"
          >—</span>

          <span
            v-if="run.status === 'completed' && formatDuration(run.startedAt, run.completedAt)"
            data-testid="run-duration"
          >
            {{ formatDuration(run.startedAt, run.completedAt) }}
          </span>
          <span
            v-else-if="run.completedAt !== null"
            data-testid="run-ended"
          >
            Ended {{ formatRelative(run.completedAt) }}
          </span>

          <span
            v-if="run.outcome !== null"
            class="ml-auto shrink-0 rounded px-1.5 py-0.5 text-[0.65rem] font-medium leading-none"
            :class="outcomeChipClass(run.outcome)"
            :data-outcome="run.outcome"
            data-testid="run-outcome-badge"
          >
            {{ run.outcome }}
          </span>
        </div>
      </li>
    </ul>
  </section>
</template>
