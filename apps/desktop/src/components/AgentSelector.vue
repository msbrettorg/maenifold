<script setup lang="ts">
import { computed, watch } from 'vue';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';

const emit = defineEmits<{
  select: [agentName: string];
}>();

const configStore = useConfigStore();
const capabilitiesStore = useAgentCapabilitiesStore();

const selectedAgent = defineModel<string>('selected', { default: '' });

const agents = computed(() => configStore.agentNames);
const hasAgents = computed(() => configStore.hasAgents);
const configPath = computed(() => configStore.configPath);
const selectedCapabilityBadges = computed(() => formatCapabilityBadges(selectedAgent.value));

// Auto-select first agent when agents are available and none selected
watch(agents, (newAgents) => {
  if (newAgents.length > 0 && !selectedAgent.value) {
    selectedAgent.value = newAgents[0];
    emit('select', newAgents[0]);
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

function formatModes(agentName: string): string[] {
  const modes = getProbe(agentName)?.modes ?? [];
  return modes.length > 2 ? [...modes.slice(0, 2), `+${modes.length - 2}`] : modes;
}

function formatModelSummary(agentName: string): string | null {
  const models = getProbe(agentName)?.models ?? [];
  if (models.length === 0) return null;
  return models.length === 1 ? models[0] : `${models.length} models`;
}

function formatCapabilityBadges(agentName: string): string[] {
  if (!agentName) return [];
  const probe = getProbe(agentName);
  if (!probe) return [];
  if (probe.error) return ['⚠ probe failed'];

  return [...formatModes(agentName), ...(formatModelSummary(agentName) ? [formatModelSummary(agentName)!] : [])];
}

function formatAgentOptionLabel(agentName: string): string {
  const probe = getProbe(agentName);
  if (!probe) return agentName;
  if (probe.error) return `${agentName} ⚠`;
  if (probe.modes.length <= 2 && probe.modes.length > 0) return `${agentName} — ${probe.modes.join(', ')}`;
  if (probe.modes.length > 2) return `${agentName} (${probe.modes.length} modes)`;
  return probe.models.length ? `${agentName} (${probe.models.length} model${probe.models.length === 1 ? '' : 's'})` : agentName;
}
</script>

<template>
  <div class="agent-selector">
    <label for="agent-select">Agent:</label>
    <select 
      id="agent-select" 
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

    <div
      v-if="selectedAgent && selectedCapabilityBadges.length"
      class="capability-badges"
    >
      <span v-for="badge in selectedCapabilityBadges" :key="badge" class="capability-badge">{{ badge }}</span>
    </div>
    
    <div v-if="!hasAgents" class="config-hint">
      <p>No agents found. Add agents to:</p>
      <code>{{ configPath }}</code>
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
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

select {
  padding: 0.5rem 0.625rem;
  border: 1px solid color-mix(in srgb, var(--border-color) 60%, transparent);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  background: var(--bg-input);
  color: var(--text-primary);
  font-family: var(--font-sans);
}

option {
  background: var(--bg-input);
  color: var(--text-primary);
}

.capability-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.capability-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.15rem 0.45rem;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  font-size: var(--text-xs);
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-input) 86%, transparent);
}

select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.config-hint {
  margin-top: 0.5rem;
  padding: 0.75rem;
  background: var(--bg-warning);
  border-radius: 4px;
  font-size: var(--text-sm);
}

.config-hint code {
  display: block;
  margin-top: 0.25rem;
  padding: 0.25rem 0.5rem;
  background: var(--bg-code);
  border-radius: 2px;
  font-family: var(--font-mono);
  word-break: break-all;
}
</style>
