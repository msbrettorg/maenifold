import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

import { type AppShellTab, useAppShellStore } from './appShell';

beforeEach(() => {
  setActivePinia(createPinia());
});

describe('useAppShellStore', () => {
  it('includes settings in AppShellTab', () => {
    const settingsTab: AppShellTab = 'settings';

    expect(settingsTab).toBe('settings');
  });

  it('updates activeTab when set to settings', () => {
    const store = useAppShellStore();

    store.setActiveTab('settings');

    expect(store.activeTab).toBe('settings');
  });

  it('accepts each expected app-shell tab', () => {
    const store = useAppShellStore();
    const tabs: AppShellTab[] = [
      'home',
      'agents',
      'gate',
      'pipelines',
      'workflows',
      'schedules',
      'settings',
    ];

    for (const tab of tabs) {
      expect(() => store.setActiveTab(tab)).not.toThrow();
      expect(store.activeTab).toBe(tab);
    }
  });
});
