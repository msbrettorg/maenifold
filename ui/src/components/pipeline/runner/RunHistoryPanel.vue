<script setup lang="ts">
// Run-history panel. Renders one row per node — node / status / outcome /
// duration — derived from the live view (and the list item's per-run outcomes)
// via the pure `runnerHistory` helpers. Sortable newest / oldest / node-name.
import { computed, ref } from 'vue';
import { ArrowDownUp, History } from '@lucide/vue';
import type { PipelineListItem, PipelineView } from '../../../lib/pipeline-types';
import {
  buildHistoryRows,
  formatDuration,
  sortHistory,
  type HistorySort,
} from './runnerHistory';

const props = defineProps<{
  view: PipelineView;
  listItem?: PipelineListItem | null;
}>();

const sort = ref<HistorySort>('newest');

const rows = computed(() =>
  sortHistory(buildHistoryRows(props.view, props.listItem), sort.value)
);
</script>

<template>
  <section class="rhist">
    <header class="rhist-head">
      <span class="rhist-title">
        <History :size="14" :stroke-width="2" />
        <strong>Run history</strong>
      </span>
      <label class="rhist-sort">
        <ArrowDownUp :size="13" :stroke-width="2" />
        <select v-model="sort" aria-label="Sort run history">
          <option value="newest">Newest</option>
          <option value="oldest">Oldest</option>
          <option value="node">Node name</option>
        </select>
      </label>
    </header>

    <div class="rhist-body">
      <p v-if="rows.length === 0" class="rhist-empty">No runs yet.</p>
      <table v-else class="rhist-table">
        <thead>
          <tr>
            <th>Node</th>
            <th>Status</th>
            <th>Outcome</th>
            <th class="num">Duration</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in rows" :key="row.nodeId">
            <td class="node">{{ row.label }}</td>
            <td><span class="badge" :class="row.status">{{ row.status }}</span></td>
            <td class="outcome">{{ row.outcome ?? '—' }}</td>
            <td class="num">{{ formatDuration(row.durationMs) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.rhist {
  display: flex;
  flex-direction: column;
  min-height: 0;
  gap: 0.4rem;
}

.rhist-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.rhist-title {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.rhist-title strong {
  color: var(--text-primary);
  font-size: 0.84rem;
}

.rhist-sort {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  color: var(--text-muted);
}

.rhist-sort select {
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-primary);
  font: inherit;
  font-size: 0.76rem;
  padding: 0.2rem 0.35rem;
  outline: none;
}

.rhist-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-main);
}

.rhist-empty {
  margin: 0;
  padding: 0.5rem;
  color: var(--text-muted);
  font-size: 0.78rem;
}

.rhist-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.78rem;
}

.rhist-table th {
  position: sticky;
  top: 0;
  text-align: left;
  padding: 0.35rem 0.5rem;
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.rhist-table td {
  padding: 0.35rem 0.5rem;
  border-top: 1px solid var(--border-color);
  color: var(--text-primary);
  vertical-align: top;
}

.rhist-table .num {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.node {
  font-weight: 550;
}

.outcome {
  color: var(--text-secondary);
}

.badge {
  display: inline-block;
  padding: 0.1rem 0.4rem;
  border-radius: 999px;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-muted) 18%, transparent);
}

.badge.completed {
  color: var(--node-status-completed, var(--text-primary));
  background: color-mix(in srgb, var(--node-status-completed, var(--text-accent)) 18%, transparent);
}

.badge.active,
.badge.retrying {
  color: var(--node-status-active, var(--text-accent));
  background: color-mix(in srgb, var(--node-status-active, var(--text-accent)) 18%, transparent);
}

.badge.failed,
.badge.stuck {
  color: var(--node-status-stuck, var(--bg-danger));
  background: color-mix(in srgb, var(--node-status-stuck, var(--bg-danger)) 18%, transparent);
}

.badge.abandoned,
.badge.cancelled,
.badge.skipped {
  color: var(--bg-danger);
  background: color-mix(in srgb, var(--bg-danger) 14%, transparent);
}
</style>
