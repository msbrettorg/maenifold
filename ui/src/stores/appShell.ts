// App-shell store: owns the top-level tab navigation state.
import { ref } from 'vue';
import { defineStore } from 'pinia';

export type AppShellTab =
  | 'home'
  | 'agents'
  | 'knowledge'
  | 'gate'
  | 'schedules'
  | 'pipelines'
  | 'workflows'
  | 'settings';

export const useAppShellStore = defineStore('appShell', () => {
  // Launch on Home so the themed landing page is the first thing the user
  // sees. App.vue auto-switches to Agents whenever a session becomes active
  // so a live chat is never left hidden behind the landing page.
  const activeTab = ref<AppShellTab>('home');

  function setActiveTab(tab: AppShellTab): void {
    activeTab.value = tab;
  }

  return { activeTab, setActiveTab };
});
