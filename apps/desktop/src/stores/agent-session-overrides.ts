import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useAgentSessionOverridesStore = defineStore('agent-session-overrides', () => {
  const selectedOverrides = ref<Map<string, Map<string, string>>>(new Map());

  function getSelectedOverride(agentName: string, key: string): string | undefined {
    return selectedOverrides.value.get(agentName)?.get(key);
  }

  function getSelectedOverrides(agentName: string): Record<string, string> {
    return Object.fromEntries(selectedOverrides.value.get(agentName)?.entries() ?? []);
  }

  function setSelectedOverride(agentName: string, key: string, value: string) {
    const next = new Map(selectedOverrides.value);
    const agentOverrides = new Map(next.get(agentName) ?? []);

    agentOverrides.set(key, value);
    next.set(agentName, agentOverrides);
    selectedOverrides.value = next;
  }

  function clearSelectedOverrides(agentName: string) {
    if (!selectedOverrides.value.has(agentName)) {
      return;
    }

    const next = new Map(selectedOverrides.value);
    next.delete(agentName);
    selectedOverrides.value = next;
  }

  return {
    selectedOverrides,
    getSelectedOverride,
    getSelectedOverrides,
    setSelectedOverride,
    clearSelectedOverrides,
  };
});
