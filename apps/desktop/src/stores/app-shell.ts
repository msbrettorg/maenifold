import { computed, ref } from 'vue';
import { defineStore } from 'pinia';

export type AppShellTab = 'home' | 'agents' | 'pipelines' | 'gate' | 'knowledge' | 'workflows' | 'schedules' | 'settings';

export const useAppShellStore = defineStore('appShell', () => {
  const activeTab = ref<AppShellTab>('home');

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
        activeTab.value = 'home';
      }
    },
  });

  const showGate = computed({
    get: () => activeTab.value === 'gate',
    set: (value: boolean) => {
      if (value) {
        activeTab.value = 'gate';
      } else if (activeTab.value === 'gate') {
        activeTab.value = 'home';
      }
    },
  });

  const showKnowledge = computed({
    get: () => activeTab.value === 'knowledge',
    set: (value: boolean) => {
      if (value) {
        activeTab.value = 'knowledge';
      } else if (activeTab.value === 'knowledge') {
        activeTab.value = 'home';
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
    showKnowledge,
    setActiveTab,
    openAgents,
  };
});
