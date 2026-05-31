<script setup lang="ts">
// Top-level tab strip. Drives the app-shell store's active tab. Each tab is an
// off-the-shelf Lucide icon + label. Kept deliberately presentation-only: all
// navigation state lives in the appShell store so HomeView quick-tiles and
// App.vue can switch tabs too.
import { House, MessageSquare, ShieldCheck, GitBranch, Workflow, Brain } from '@lucide/vue';
import { useAppShellStore, type AppShellTab } from '../stores/appShell';

interface TabDef {
  id: AppShellTab;
  label: string;
  icon: typeof House;
}

const tabs: TabDef[] = [
  { id: 'home', label: 'Home', icon: House },
  { id: 'agents', label: 'Agents', icon: MessageSquare },
  { id: 'gate', label: 'Gates', icon: ShieldCheck },
  { id: 'pipelines', label: 'Pipelines', icon: GitBranch },
  { id: 'workflows', label: 'Workflows', icon: Workflow },
];

const shell = useAppShellStore();
</script>

<template>
  <header class="tab-bar" role="tablist" aria-label="Primary">
    <div class="tab-bar-brand" aria-hidden="true">
      <Brain class="tab-bar-brand-icon" :size="20" :stroke-width="1.5" />
      <span class="tab-bar-brand-name">Maenifold</span>
    </div>

    <nav class="tab-bar-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        type="button"
        role="tab"
        class="tab-item"
        :class="{ active: shell.activeTab === tab.id }"
        :aria-selected="shell.activeTab === tab.id"
        @click="shell.setActiveTab(tab.id)"
      >
        <component :is="tab.icon" class="tab-item-icon" :size="17" :stroke-width="2" />
        <span class="tab-item-label">{{ tab.label }}</span>
      </button>
    </nav>
  </header>
</template>

<style scoped>
.tab-bar {
  flex-shrink: 0;
  height: var(--tabbar-height, 52px);
  display: flex;
  align-items: center;
  gap: 1.25rem;
  padding: 0 1rem;
  background: var(--bg-sidebar);
  border-bottom: 1px solid var(--border-color);
  -webkit-app-region: drag;
  user-select: none;
}

.tab-bar-brand {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding-right: 0.75rem;
}

.tab-bar-brand-icon {
  color: var(--text-accent);
  filter: drop-shadow(0 2px 6px color-mix(in srgb, var(--text-accent) 30%, transparent));
}

.tab-bar-brand-name {
  font-size: 0.95rem;
  font-weight: 700;
  letter-spacing: -0.01em;
  color: var(--text-primary);
}

.tab-bar-tabs {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  /* Tabs must be clickable even though the bar is a drag region (Tauri). */
  -webkit-app-region: no-drag;
}

.tab-item {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.4rem 0.75rem;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.tab-item:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.tab-item.active {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, var(--border-color));
}

.tab-item-icon {
  flex-shrink: 0;
}

@media (max-width: 640px) {
  .tab-bar {
    gap: 0.5rem;
    padding: 0 0.5rem;
  }
  .tab-bar-brand-name {
    display: none;
  }
  .tab-item-label {
    display: none;
  }
  .tab-item {
    padding: 0.4rem 0.55rem;
  }
}
</style>
