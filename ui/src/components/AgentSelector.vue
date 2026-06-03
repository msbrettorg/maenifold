<script setup lang="ts">
import { computed, watch } from 'vue';
import { useConfigStore } from '../stores/config';
import { restrictedTransports } from '../lib/platform';

const emit = defineEmits<{
  select: [agentName: string];
}>();

const configStore = useConfigStore();

const selectedAgent = defineModel<string>('selected', { default: '' });

const agents = computed(() => configStore.agentNames);
const hasAgents = computed(() => configStore.hasAgents);
const configPath = computed(() => configStore.configPath);
// Preferred default agent name from `settings.defaultAgent` in
// `~/maenifold/maenifold.json`. The store returns `undefined` when
// unset or pointing at an unknown agent so we fall back to the first
// agent in config order.
const defaultAgentName = computed<string | undefined>(() => configStore.defaultAgentName);
// On mobile / web the config-file path is not actionable (sandbox /
// localStorage). Point users at the Settings dialog instead of the path.
const restricted = restrictedTransports();

/** Build the display label for each agent once, instead of calling
 * `getAgentTransportKind` twice per option in the template. */
const agentLabels = computed<Record<string, string>>(() => {
  const out: Record<string, string> = {};
  for (const name of agents.value) {
    const kind = configStore.getAgentTransportKind(name);
    out[name] = kind === 'stdio' ? name : `${name} (${kind})`;
  }
  return out;
});

// Auto-select the preferred default agent when none is selected yet.
// The preferred name comes from `settings.defaultAgent` in
// `~/maenifold/maenifold.json` -- no hardcoded fallback. If the
// configured default isn't installed (or no default is set), fall back
// to the first agent in config order so the picker is never empty.
watch(agents, (newAgents) => {
  if (newAgents.length > 0 && !selectedAgent.value) {
    const preferred = defaultAgentName.value && newAgents.includes(defaultAgentName.value)
      ? defaultAgentName.value
      : newAgents[0];
    selectedAgent.value = preferred;
    emit('select', preferred);
  }
}, { immediate: true });

function handleSelect(event: Event) {
  const target = event.target as HTMLSelectElement;
  selectedAgent.value = target.value;
  if (target.value) {
    emit('select', target.value);
  }
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
        {{ agentLabels[agent] }}
      </option>
    </select>
    
    <div v-if="!hasAgents" class="config-hint">
      <template v-if="restricted">
        <p>No remote agents configured.</p>
        <p class="hint-action">Open the Settings tab to add one.</p>
      </template>
      <template v-else>
        <p>No agents found. Add agents to:</p>
        <code>{{ configPath }}</code>
      </template>
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
  font-weight: 600;
  color: var(--text-secondary);
}

/* WebView2 (Tauri) leaves <select> native-styled by default, which on
   Windows draws white-on-white in OS dark mode. Force our own theme
   variables and lock the appearance so the dropdown is always readable. */
select {
  -webkit-appearance: none;
  -moz-appearance: none;
  appearance: none;
  padding: 0.5rem 1.75rem 0.5rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-size: 1rem;
  font-family: inherit;
  background-color: var(--bg-main);
  color: var(--text-primary);
  background-image: linear-gradient(45deg, transparent 50%, var(--text-muted) 50%),
                    linear-gradient(135deg, var(--text-muted) 50%, transparent 50%);
  background-position: calc(100% - 14px) 50%, calc(100% - 9px) 50%;
  background-size: 5px 5px, 5px 5px;
  background-repeat: no-repeat;
}

select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* The <option> elements pop up in a native OS menu; we can only color the
   text/background — the chrome is OS-owned. Setting them here so the
   options aren't unreadable when the OS menu draws in our app's theme. */
select option {
  background-color: var(--bg-main);
  color: var(--text-primary);
}

.config-hint {
  margin-top: 0.5rem;
  padding: 0.75rem;
  background: var(--bg-warning, #fff3cd);
  color: #5b4500;
  border-radius: 4px;
  font-size: 0.875rem;
}

.config-hint code {
  display: block;
  margin-top: 0.25rem;
  padding: 0.25rem 0.5rem;
  background: rgba(0, 0, 0, 0.08);
  border-radius: 2px;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  word-break: break-all;
}
</style>
