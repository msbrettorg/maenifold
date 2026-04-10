import { computed, ref } from 'vue';
import { useAppSession } from './useAppSession';
import type { ConfigStoreLike, SessionStoreLike } from './useAppSession';

export function useAppShell({
  configStore,
  sessionStore,
}: { configStore: ConfigStoreLike; sessionStore: SessionStoreLike }) {
  const showSidebar = ref(true);
  const showSettings = ref(false);
  const showTrafficMonitor = ref(false);
  const showStartupDetails = ref(false);
  const showWorkflows = ref(false);
  const showPipelines = ref(false);
  const showGate = ref(false);
  const showGraph = ref(false);
  const activeTab = computed({
    get: () => {
      if (showPipelines.value) return 'pipelines';
      if (showGate.value) return 'gates';
      if (showGraph.value) return 'graph';
      if (showWorkflows.value) return 'workflows';
      return 'agents';
    },
    set: (tab: string) => {
      showWorkflows.value = tab === 'workflows';
      showPipelines.value = tab === 'pipelines';
      showGate.value = tab === 'gates';
      showGraph.value = tab === 'graph';
    },
  });

  const session = useAppSession({ configStore, sessionStore });

  function toggleSidebar() { showSidebar.value = !showSidebar.value; }
  function toggleWorkflows() {
    showWorkflows.value = !showWorkflows.value;
    if (showWorkflows.value) {
      showPipelines.value = false;
      showGate.value = false;
      showGraph.value = false;
    }
  }
  function togglePipelines() {
    showPipelines.value = !showPipelines.value;
    if (showPipelines.value) {
      showWorkflows.value = false;
      showGate.value = false;
      showGraph.value = false;
    }
  }
  function toggleGate() {
    showGate.value = !showGate.value;
    if (showGate.value) {
      showWorkflows.value = false;
      showPipelines.value = false;
      showGraph.value = false;
    }
  }
  function toggleGraph() {
    showGraph.value = !showGraph.value;
    if (showGraph.value) {
      showWorkflows.value = false;
      showPipelines.value = false;
      showGate.value = false;
    }
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
