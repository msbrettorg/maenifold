<script setup lang="ts">
import { computed, watch } from 'vue';
import { resolvePreferredAgent } from '../lib/preferred-agent';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import AgentProbePreview from './AgentProbePreview.vue';

const emit = defineEmits<{
  select: [agentName: string];
}>();

const configStore = useConfigStore();
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

function handleSelect(event: Event) {
  const target = event.target as HTMLSelectElement;
  selectedAgent.value = target.value;
  if (target.value) {
    emit('select', target.value);
  }
}

function getProbe(agentName: string) {
  return capabilitiesStore.getProbe(agentName);
}

function formatAgentOptionLabel(agentName: string): string {
  const probe = getProbe(agentName);
  if (!probe) return agentName;
  if (probe.state === 'offline') return `${agentName} — offline`;
  if (probe.error) return `${agentName} — probe failed`;
  return agentName;
}
</script>

<template>
  <div class="agent-selector">
    <label for="agent-select">Agent</label>
     <select 
       id="agent-select" 
       role="listbox"
       aria-label="Agent selector"
       :value="selectedAgent"
       @change="handleSelect"
       :disabled="!hasAgents"
     >
      <option value="" disabled>
        {{ hasAgents ? 'Select an agent...' : 'No agents configured' }}
      </option>
      <option v-for="agent in agents" :key="agent" :value="agent">
        {{ formatAgentOptionLabel(agent) }}
      </option>
    </select>

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
  color: #707070;
}

select {
  padding: 0.5rem 0.625rem;
  border: none;
  border-radius: 4px;
  font-size: var(--text-caption);
  background: #201f1f;
  color: var(--foreground);
  font-family: var(--font-body);
}

option {
  background: var(--input);
  color: var(--foreground);
}

select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.config-hint {
  margin-top: 0.5rem;
  padding: 0.75rem;
  background: #201f1f;
  border-radius: 4px;
  font-size: var(--text-caption);
  color: var(--muted-foreground);
}
</style>
