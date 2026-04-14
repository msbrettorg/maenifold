<script setup lang="ts">
import { computed } from 'vue';
import type { AgentProbeResult } from '../lib/agent-probe-types';

interface AgentRow {
  name: string;
  command: string;
}

const props = defineProps<{
  daemonUrl: string;
  configPath: string;
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

      <dl class="info-list">
        <div>
          <dt>Base URL</dt>
          <dd><code>{{ daemonUrl }}</code></dd>
        </div>
        <div>
          <dt>Config file</dt>
          <dd><code>{{ configPath }}</code></dd>
        </div>
      </dl>
    </article>

    <article class="info-card">
      <h4>Agent health</h4>
      <p>Live capability probing tracks whether configured agents are reachable and ready.</p>

      <div class="stats-grid">
        <div class="stat-chip"><strong>{{ stats.total }}</strong><span>Configured</span></div>
        <div class="stat-chip success"><strong>{{ stats.ready }}</strong><span>Ready</span></div>
        <div class="stat-chip warning"><strong>{{ stats.pending }}</strong><span>Pending</span></div>
        <div class="stat-chip muted"><strong>{{ stats.offline }}</strong><span>Offline</span></div>
        <div class="stat-chip error"><strong>{{ stats.error }}</strong><span>Errors</span></div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.info-grid,
.info-list,
.stats-grid {
  display: grid;
  gap: 1rem;
}

.info-grid {
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
}

.info-card {
  padding: 1rem;
  border-radius: 8px;
  background: var(--card);
}

.card-header {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  align-items: flex-start;
}

.info-card h4,
.info-card p,
.info-list dt,
.info-list dd {
  margin: 0;
}

.info-card p,
.info-list dt,
.stat-chip span {
  color: var(--muted-foreground);
}

.info-list {
  margin-top: 0.9rem;
}

.info-list code {
  display: block;
  margin-top: 0.25rem;
  word-break: break-all;
  font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
  font-size: 0.75rem;
}

.stats-grid {
  grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
  margin-top: 0.9rem;
}

.stat-chip {
  display: grid;
  gap: 0.2rem;
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--background);
}

.stat-chip strong {
  font-size: 0.9375rem;
}

.stat-chip.success strong {
  color: var(--ring);
}

.stat-chip.warning strong,
.stat-chip.muted strong {
  color: var(--foreground);
}

.stat-chip.error strong {
  color: var(--destructive);
}

.refresh-btn {
  padding: 0.5rem 0.9rem;
  border: 1px solid var(--border);
  border-radius: 6px;
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
