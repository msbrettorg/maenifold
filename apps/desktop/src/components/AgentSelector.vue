<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { resolvePreferredAgent } from '../lib/preferred-agent';
import { useAgentStore } from '../stores/agent';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import AgentProbePreview from './AgentProbePreview.vue';

const emit = defineEmits<{
  select: [agentName: string];
}>();

const configStore = useConfigStore();
const agentStore = useAgentStore();
const capabilitiesStore = useAgentCapabilitiesStore();

const selectedAgent = defineModel<string>('selected', { default: '' });

const agents = computed(() => configStore.agentNames);
const hasAgents = computed(() => configStore.hasAgents);
const selectedProbe = computed(() =>
  selectedAgent.value ? capabilitiesStore.getProbe(selectedAgent.value) : undefined,
);
const selectedProbeLoading = computed(
  () => Boolean(selectedAgent.value) && !selectedProbe.value && capabilitiesStore.probeStatus === 'probing',
);
const activeSessionCountByAgent = computed(() => {
  const counts = new Map<string, number>();
  for (const session of agentStore.activeSessions) {
    counts.set(session.agentName, (counts.get(session.agentName) ?? 0) + 1);
  }
  return counts;
});

onMounted(() => {
  void agentStore.start();
});

onBeforeUnmount(() => {
  void agentStore.stop();
});

watch(agents, (newAgents) => {
  if (newAgents.length === 0) {
    selectedAgent.value = '';
    return;
  }

  if (selectedAgent.value && newAgents.includes(selectedAgent.value)) {
    return;
  }

  const nextAgent = resolvePreferredAgent(newAgents);
  if (nextAgent) {
    selectedAgent.value = nextAgent;
    emit('select', nextAgent);
  }
}, { immediate: true });

function handleSelect(value: string | number | bigint | Record<string, unknown> | null) {
  const strValue = String(value ?? '');
  selectedAgent.value = strValue;
  if (strValue) {
    emit('select', strValue);
  }
}

function getProbe(agentName: string) {
  return capabilitiesStore.getProbe(agentName);
}

function formatAgentOptionLabel(agentName: string): string {
  const activeCount = activeSessionCountByAgent.value.get(agentName) ?? 0;
  const activeSuffix = activeCount > 0 ? ` (${activeCount} active)` : '';
  const probe = getProbe(agentName);
  if (!probe) return `${agentName}${activeSuffix}`;
  if (probe.state === 'offline') return `${agentName} — offline${activeSuffix}`;
  if (probe.error) return `${agentName} — probe failed${activeSuffix}`;
  return `${agentName}${activeSuffix}`;
}
</script>

<template>
  <div class="agent-selector">
    <Label for="agent-select">Agent</Label>
    <Select
      :model-value="selectedAgent"
      :disabled="!hasAgents"
      @update:model-value="handleSelect"
    >
      <SelectTrigger id="agent-select" aria-label="Agent selector">
        <SelectValue :placeholder="hasAgents ? 'Select an agent...' : 'No agents configured'" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem v-for="agent in agents" :key="agent" :value="agent">
          {{ formatAgentOptionLabel(agent) }}
        </SelectItem>
      </SelectContent>
    </Select>

    <AgentProbePreview
      v-if="selectedAgent"
      :agent-name="selectedAgent"
      :probe="selectedProbe"
      :loading="selectedProbeLoading"
      compact
    />
    
    <div v-if="!hasAgents" class="config-hint">
      <p>No agents found. Add an agent in your config to get started.</p>
    </div>
  </div>
</template>

<style scoped>
.agent-selector {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

label {
  font-size: var(--text-tiny);
  font-weight: 400;
  color: var(--text-dim);
}

select {
  padding: 0.5rem 0.625rem;
  border: none;
  border-radius: 4px;
  font-size: var(--text-caption);
  background: var(--surface-container);
  color: var(--color-text-primary);
  font-family: var(--font-body);
}

option {
  background: var(--color-bg-surface);
  color: var(--color-text-primary);
}

select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.config-hint {
  margin-top: 0.5rem;
  padding: 0.75rem;
  background: var(--surface-container);
  border-radius: 4px;
  font-size: var(--text-caption);
  color: var(--color-text-secondary);
}
</style>
