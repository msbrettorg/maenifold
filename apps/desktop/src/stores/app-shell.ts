import { computed, ref } from 'vue';
import { defineStore } from 'pinia';

export type AppShellTab = 'agents' | 'pipelines' | 'gates' | 'workflows' | 'graph';

export const useAppShellStore = defineStore('appShell', () => {
  const activeTab = ref<AppShellTab>('agents');

  const showWorkflows = computed({
    get: () => activeTab.value === 'workflows',
    set: (value: boolean) => {
      if (value) {
        activeTab.value = 'workflows';
      } else if (activeTab.value === 'workflows') {
        activeTab.value = 'agents';
      }
    },
  });

  const showPipelines = computed({
    get: () => activeTab.value === 'pipelines',
    set: (value: boolean) => {
      if (value) {
        activeTab.value = 'pipelines';
      } else if (activeTab.value === 'pipelines') {
        activeTab.value = 'agents';
      }
    },
  });

  const showGate = computed({
    get: () => activeTab.value === 'gates',
    set: (value: boolean) => {
      if (value) {
        activeTab.value = 'gates';
      } else if (activeTab.value === 'gates') {
        activeTab.value = 'agents';
      }
    },
  });

  const showGraph = computed({
    get: () => activeTab.value === 'graph',
    set: (value: boolean) => {
      if (value) {
        activeTab.value = 'graph';
      } else if (activeTab.value === 'graph') {
        activeTab.value = 'agents';
      }
    },
  });

  function setActiveTab(tab: AppShellTab): void {
    activeTab.value = tab;
  }

  function openAgents(): void {
    activeTab.value = 'agents';
  }

  return {
    activeTab,
    showWorkflows,
    showPipelines,
    showGate,
    showGraph,
    setActiveTab,
    openAgents,
  };
});
