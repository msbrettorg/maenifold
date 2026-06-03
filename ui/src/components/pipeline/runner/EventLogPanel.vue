<script setup lang="ts">
// Event-log panel. Renders the store's live event log (appended by the runner's
// poll loop) newest-at-bottom in a scrollable pane, auto-scrolling to the tail
// as entries arrive. Surfaces the live polling indicator and the most recent
// poll error (after the 3-strike tolerance).
import { nextTick, ref, watch } from 'vue';
import { ScrollText } from '@lucide/vue';
import type { LogEntry } from './runnerLog';

const props = defineProps<{
  entries: LogEntry[];
  pollingActive: boolean;
  pollError: string | null;
}>();

const scroller = ref<HTMLElement | null>(null);

function timeLabel(at: number): string {
  return new Date(at).toLocaleTimeString();
}

// Keep the newest entry in view as the log grows.
watch(
  () => props.entries.length,
  async () => {
    await nextTick();
    const el = scroller.value;
    if (el) el.scrollTop = el.scrollHeight;
  }
);
</script>

<template>
  <section class="elog">
    <header class="elog-head">
      <span class="elog-title">
        <ScrollText :size="14" :stroke-width="2" />
        <strong>Event log</strong>
      </span>
      <span class="elog-state" :class="{ live: pollingActive }">
        <span class="dot" aria-hidden="true"></span>
        {{ pollingActive ? 'Polling' : 'Idle' }}
      </span>
    </header>

    <p v-if="pollError" class="elog-error" role="status">{{ pollError }}</p>

    <div ref="scroller" class="elog-body">
      <p v-if="entries.length === 0" class="elog-empty">No events yet.</p>
      <ul v-else class="elog-list">
        <li v-for="entry in entries" :key="entry.id" class="elog-row" :class="entry.level">
          <span class="elog-time">{{ timeLabel(entry.at) }}</span>
          <span class="elog-msg">{{ entry.message }}</span>
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.elog {
  display: flex;
  flex-direction: column;
  min-height: 0;
  gap: 0.4rem;
}

.elog-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.elog-title {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.elog-title strong {
  color: var(--text-primary);
  font-size: 0.84rem;
}

.elog-state {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.72rem;
  color: var(--text-muted);
}

.elog-state .dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--text-muted);
}

.elog-state.live {
  color: var(--node-status-active, var(--text-accent));
}

.elog-state.live .dot {
  background: var(--node-status-active, var(--text-accent));
  animation: elog-pulse 1.2s ease-in-out infinite;
}

@keyframes elog-pulse {
  50% {
    opacity: 0.3;
  }
}

.elog-error {
  margin: 0;
  padding: 0.4rem 0.5rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-danger) 14%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.74rem;
}

.elog-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-main);
  padding: 0.4rem 0.5rem;
}

.elog-empty {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.78rem;
}

.elog-list {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.elog-row {
  display: flex;
  gap: 0.5rem;
  font-size: 0.78rem;
  line-height: 1.35;
}

.elog-time {
  flex-shrink: 0;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  font-size: 0.72rem;
}

.elog-msg {
  color: var(--text-primary);
}

.elog-row.success .elog-msg {
  color: var(--node-status-completed, var(--text-primary));
}

.elog-row.warn .elog-msg {
  color: var(--node-status-stuck, var(--text-secondary));
}

.elog-row.error .elog-msg {
  color: var(--bg-danger);
}
</style>
