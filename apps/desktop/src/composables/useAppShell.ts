import { computed, ref } from 'vue';
import { useAppSession } from './useAppSession';
import type { ConfigStoreLike, SessionStoreLike } from './useAppSession';
import { useAppShellStore, type AppShellTab } from '../stores/app-shell';

export function useAppShell({
  configStore,
  sessionStore,
}: { configStore: ConfigStoreLike; sessionStore: SessionStoreLike }) {
  const appShellStore = useAppShellStore();
  const showSidebar = ref(true);
  const showSettings = ref(false);
  const showTrafficMonitor = ref(false);
  const showStartupDetails = ref(false);
  const activeTab = computed({
    get: () => appShellStore.activeTab,
    set: (tab: string) => {
      appShellStore.setActiveTab(tab as AppShellTab);
    },
  });
  const showWorkflows = computed({
    get: () => appShellStore.showWorkflows,
    set: (value: boolean) => {
      appShellStore.showWorkflows = value;
    },
  });
  const showPipelines = computed({
    get: () => appShellStore.showPipelines,
    set: (value: boolean) => {
      appShellStore.showPipelines = value;
    },
  });
  const showGate = computed({
    get: () => appShellStore.showGate,
    set: (value: boolean) => {
      appShellStore.showGate = value;
    },
  });
  const showGraph = computed({
    get: () => appShellStore.showGraph,
    set: (value: boolean) => {
      appShellStore.showGraph = value;
    },
  });

  const session = useAppSession({ configStore, sessionStore });

  function toggleSidebar() { showSidebar.value = !showSidebar.value; }
  function toggleWorkflows() {
    showWorkflows.value = !showWorkflows.value;
  }
  function togglePipelines() {
    showPipelines.value = !showPipelines.value;
  }
  function toggleGate() {
    showGate.value = !showGate.value;
  }
  function toggleGraph() {
    showGraph.value = !showGraph.value;
  }
  function clearError() { sessionStore.clearError(); configStore.clearError(); }

  return {
    showSidebar,
    showSettings,
    showTrafficMonitor,
    showStartupDetails,
    showWorkflows,
    showPipelines,
    showGate,
    showGraph,
    activeTab,
    toggleSidebar,
    toggleWorkflows,
    togglePipelines,
    toggleGate,
    toggleGraph,
    clearError,
    ...session,
  };
}
