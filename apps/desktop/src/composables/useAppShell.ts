import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useAppSession } from './useAppSession';
import type { ConfigStoreLike, SessionStoreLike } from './useAppSession';
import { useAppShellStore, type AppShellTab } from '../stores/app-shell';

export const TAB_ORDER = [
  { value: 'home', label: 'Home', icon: 'Home' },
  { value: 'agents', label: 'Agents', icon: 'MessageSquare' },
  { value: 'pipelines', label: 'Pipelines', icon: 'GitBranch' },
  { value: 'gate', label: 'Gate', icon: 'ShieldCheck' },
  { value: 'knowledge', label: 'Knowledge', icon: 'Brain' },
  { value: 'workflows', label: 'Workflows', icon: 'Workflow' },
  { value: 'settings', label: 'Settings', icon: 'Settings' },
] as const satisfies ReadonlyArray<{ value: AppShellTab; label: string; icon: string }>;

export function useAppShell({
  configStore,
  sessionStore,
}: { configStore: ConfigStoreLike; sessionStore: SessionStoreLike }) {
  const appShellStore = useAppShellStore();
  const showSidebar = ref(true);
  const showTrafficMonitor = ref(false);
  const showStartupDetails = ref(false);
  const activeTab = computed({
    get: () => appShellStore.activeTab as AppShellTab,
    set: (tab: AppShellTab) => {
      appShellStore.setActiveTab(tab);
    },
  });
  const showHome = computed(() => activeTab.value === 'home' && !sessionStore.activeSessionId);

  const session = useAppSession({ configStore, sessionStore });

  function hasBlockingDialogOpen(): boolean {
    if (typeof document === 'undefined') {
      return false;
    }
    return document.querySelector('[role="dialog"]') !== null;
  }

  function isEditableTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) {
      return false;
    }
    return (
      target.isContentEditable
      || target.tagName === 'INPUT'
      || target.tagName === 'TEXTAREA'
      || target.tagName === 'SELECT'
    );
  }

  function handleTabShortcut(event: KeyboardEvent): void {
    if (!event.metaKey || event.altKey || event.ctrlKey || event.shiftKey) {
      return;
    }

    if (hasBlockingDialogOpen() || isEditableTarget(event.target)) {
      return;
    }

    const index = Number.parseInt(event.key, 10) - 1;
    if (Number.isNaN(index) || index < 0 || index >= TAB_ORDER.length) {
      return;
    }

    event.preventDefault();
    appShellStore.setActiveTab(TAB_ORDER[index].value);
  }

  onMounted(() => {
    window.addEventListener('keydown', handleTabShortcut);
  });

  onBeforeUnmount(() => {
    window.removeEventListener('keydown', handleTabShortcut);
  });

  function toggleSidebar() { showSidebar.value = !showSidebar.value; }
  function clearError() { sessionStore.clearError(); configStore.clearError(); }

  return {
    showSidebar,
    showTrafficMonitor,
    showStartupDetails,
    showHome,
    activeTab,
    toggleSidebar,
    clearError,
    ...session,
  };
}
