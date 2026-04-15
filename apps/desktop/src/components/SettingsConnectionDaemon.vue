<script setup lang="ts">
import { computed, ref } from 'vue';
import type { AgentProbeResult } from '../lib/agent-probe-types';

interface AgentRow {
  name: string;
  command: string;
}

const props = defineProps<{
  daemonUrl: string;
  agents: AgentRow[];
  isRefreshing: boolean;
  getProbe: (agentName: string) => AgentProbeResult | undefined;
  onRefresh: () => void | Promise<void>;
}>();

const stats = computed(() =>
  props.agents.reduce(
    (summary, agent) => {
      const probe = props.getProbe(agent.name);
      summary.total += 1;

      if (!probe) {
        summary.pending += 1;
      } else if (probe.state === 'ready') {
        summary.ready += 1;
      } else if (probe.state === 'offline') {
        summary.offline += 1;
      } else {
        summary.error += 1;
      }

      return summary;
    },
    { total: 0, ready: 0, offline: 0, error: 0, pending: 0 },
  ),
);

const displayAddress = computed(() => {
  try {
    return new URL(props.daemonUrl).host;
  } catch {
    return props.daemonUrl;
  }
});

const copied = ref(false);

async function copyAddress() {
  try {
    await navigator.clipboard.writeText(displayAddress.value);
    copied.value = true;
    setTimeout(() => { copied.value = false; }, 1500);
  } catch { /* clipboard unavailable */ }
}
</script>

<template>
  <section class="info-grid">
    <article class="info-card">
      <div class="card-header">
        <div>
          <h4>Daemon</h4>
          <p>Desktop settings are loaded from the daemon-backed configuration API.</p>
        </div>
        <button class="refresh-btn" type="button" :disabled="isRefreshing" @click="props.onRefresh">
          {{ isRefreshing ? 'Refreshing…' : 'Refresh health' }}
        </button>
      </div>

      <div class="daemon-address">
        <code>{{ displayAddress }}</code>
        <button class="copy-btn" type="button" @click="copyAddress">
          {{ copied ? '✓' : 'Copy' }}
        </button>
      </div>
    </article>

    <article class="info-card">
      <h4>Agent health</h4>
      <p>Live capability probing tracks whether configured agents are reachable and ready.</p>

      <div class="stats-grid">
        <div class="stat-chip"><strong>{{ stats.total }}</strong><span>Configured</span></div>
        <div class="stat-chip success"><strong>{{ stats.ready }}</strong><span>Ready</span></div>
        <div class="stat-chip warning"><strong>{{ stats.pending }}</strong><span>Pending</span></div>
        <div class="stat-chip muted"><strong>{{ stats.offline }}</strong><span>Offline</span></div>
        <div class="stat-chip" :class="stats.error > 0 ? 'error' : 'muted'"><strong>{{ stats.error }}</strong><span>Errors</span></div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.info-grid,
.stats-grid {
  display: grid;
  gap: 1rem;
}

.info-grid {
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
}

.info-card {
  padding: 1rem;
  border-radius: 4px;
  background: var(--card);
}

.card-header {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  align-items: flex-start;
}

.info-card h4,
.info-card p {
  margin: 0;
}

.info-card h4 {
  font-family: var(--font-display);
  font-weight: 500;
}

.info-card p,
.stat-chip span {
  color: var(--muted-foreground);
}

.daemon-address {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-top: 0.75rem;
}

.daemon-address code {
  font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
  font-size: 0.75rem;
}

.copy-btn {
  padding: 0.2rem 0.5rem;
  border: none;
  border-radius: 4px;
  background: #2a2a2a;
  color: var(--muted-foreground);
  cursor: pointer;
  font-size: 0.6875rem;
}

.copy-btn:hover {
  background: #353534;
}

.stats-grid {
  grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
  margin-top: 0.9rem;
}

.stat-chip {
  display: grid;
  gap: 0.2rem;
  padding: 0.75rem;
  border-radius: 4px;
  background: #1c1b1b;
}

.stat-chip strong {
  font-size: 0.9375rem;
}

.stat-chip.success strong {
  color: var(--ring);
}

.stat-chip.warning strong {
  color: var(--foreground);
}

.stat-chip.muted strong {
  color: var(--muted-foreground);
}

.stat-chip.error strong {
  color: var(--destructive);
}

.refresh-btn {
  padding: 0.5rem 0.9rem;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: transparent;
  color: inherit;
  cursor: pointer;
}

@media (max-width: 640px) {
  .card-header {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
