// App-shell store: owns the top-level tab navigation state.
import { ref } from 'vue';
import { defineStore } from 'pinia';

export type AppShellTab =
  | 'home'
  | 'agents'
  | 'knowledge'
  | 'schedules'
  | 'pipelines'
  | 'settings';

export const useAppShellStore = defineStore('appShell', () => {
  // Launch on Home so the themed landing page is the first thing the user
  // sees. App.vue auto-switches to Agents whenever a session becomes active
  // so a live chat is never left hidden behind the landing page.
  const activeTab = ref<AppShellTab>('home');

  // Optional navigation guard. A feature owning unsaved state (e.g. the
  // Pipelines design store) registers a predicate consulted before EVERY tab
  // switch; returning false aborts the switch. There is no vue-router here, so
  // this tab store is the single chokepoint for in-app navigation.
  let navigationGuard: (() => boolean) | null = null;

  function registerNavigationGuard(guard: () => boolean): void {
    navigationGuard = guard;
  }

  function clearNavigationGuard(): void {
    navigationGuard = null;
  }

  function setActiveTab(tab: AppShellTab): void {
    if (tab === activeTab.value) return;
    if (navigationGuard && !navigationGuard()) return;
    activeTab.value = tab;
  }

  return {
    activeTab,
    setActiveTab,
    registerNavigationGuard,
    clearNavigationGuard,
  };
});
