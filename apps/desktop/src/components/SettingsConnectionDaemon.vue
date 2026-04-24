<script setup lang="ts">
import { computed, ref } from 'vue';
import type { AgentProbeResult } from '../lib/agent-probe-types';
import { Button } from '@/components/ui/button';

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
  <section class="grid gap-4 grid-cols-[repeat(auto-fit,minmax(260px,1fr))]">
    <article class="p-4 rounded bg-bg-elevated">
      <div class="flex justify-between gap-3 items-start max-sm:flex-col max-sm:items-stretch">
        <div>
          <h4 class="m-0 font-display font-medium">Daemon</h4>
          <p class="m-0 text-text-secondary">Desktop settings are loaded from the daemon-backed configuration API.</p>
        </div>
        <Button variant="outline" class="px-[0.9rem] py-2 border border-border-subtle rounded bg-transparent text-inherit cursor-pointer" type="button" :disabled="isRefreshing" @click="props.onRefresh">
          {{ isRefreshing ? 'Refreshing…' : 'Refresh health' }}
        </Button>
      </div>

      <div class="flex items-center gap-2 mt-3">
        <code class="font-code text-[0.75rem]">{{ displayAddress }}</code>
        <Button variant="ghost" class="px-2 py-[0.2rem] border-none rounded bg-surface-high text-text-secondary cursor-pointer text-tiny hover:bg-surface-highest" type="button" size="sm" @click="copyAddress">
          {{ copied ? '✓' : 'Copy' }}
        </Button>
      </div>
    </article>

    <article class="p-4 rounded bg-bg-elevated">
      <h4 class="m-0 font-display font-medium">Agent health</h4>
      <p class="m-0 text-text-secondary">Live capability probing tracks whether configured agents are reachable and ready.</p>

      <div class="grid gap-4 grid-cols-[repeat(auto-fit,minmax(100px,1fr))] mt-[0.9rem]">
        <div class="grid gap-[0.2rem] p-3 rounded bg-surface"><strong class="text-[0.9375rem]">{{ stats.total }}</strong><span class="text-text-secondary">Configured</span></div>
        <div class="grid gap-[0.2rem] p-3 rounded bg-surface"><strong class="text-[0.9375rem] text-brand">{{ stats.ready }}</strong><span class="text-text-secondary">Ready</span></div>
        <div class="grid gap-[0.2rem] p-3 rounded bg-surface"><strong class="text-[0.9375rem] text-text-primary">{{ stats.pending }}</strong><span class="text-text-secondary">Pending</span></div>
        <div class="grid gap-[0.2rem] p-3 rounded bg-surface"><strong class="text-[0.9375rem] text-text-secondary">{{ stats.offline }}</strong><span class="text-text-secondary">Offline</span></div>
        <div class="grid gap-[0.2rem] p-3 rounded bg-surface"><strong class="text-[0.9375rem]" :class="stats.error > 0 ? 'text-danger' : 'text-text-secondary'">{{ stats.error }}</strong><span class="text-text-secondary">Errors</span></div>
      </div>
    </article>
  </section>
</template>
