<script setup lang="ts">
import { ref } from 'vue';
import SettingsConnection from './SettingsConnection.vue';
import SettingsMaenifold from './SettingsMaenifold.vue';
import SettingsMcpServers from './SettingsMcpServers.vue';
import SettingsModels from './SettingsModels.vue';
import SettingsPreferences from './SettingsPreferences.vue';

type SectionId = 'connection' | 'mcp-servers' | 'maenifold' | 'models' | 'preferences';

const sections = [
  { id: 'connection' as SectionId, label: 'Agents', description: 'Manage ACP agents and their configuration.' },
  { id: 'mcp-servers' as SectionId, label: 'MCP servers', description: 'Configure global MCP server connections.' },
  { id: 'maenifold' as SectionId, label: 'Maenifold', description: 'Configure maenifold runtime behavior and tuning.' },
  { id: 'models' as SectionId, label: 'Models', description: 'Download and manage embedding models.' },
  { id: 'preferences' as SectionId, label: 'Preferences', description: 'Control persisted application defaults.' },
];

const activeSection = ref<SectionId>('connection');
</script>

<template>
  <div class="h-full p-6">
    <h2 class="text-lg font-semibold text-text-primary mb-1">Settings</h2>
    <p class="text-sm text-text-secondary mb-6">
      Configure your agent catalog and local application preferences.
    </p>

    <div class="settings-layout">
      <nav class="settings-nav">
        <button
          v-for="s in sections"
          :key="s.id"
          :class="['settings-nav-item', { active: activeSection === s.id }]"
          @click="activeSection = s.id"
        >
          <span class="nav-label">{{ s.label }}</span>
          <span class="nav-desc">{{ s.description }}</span>
        </button>
      </nav>

      <main class="settings-content">
        <SettingsConnection v-if="activeSection === 'connection'" />
        <SettingsMcpServers v-else-if="activeSection === 'mcp-servers'" />
        <SettingsMaenifold v-else-if="activeSection === 'maenifold'" />
        <SettingsModels v-else-if="activeSection === 'models'" />
        <SettingsPreferences v-else-if="activeSection === 'preferences'" />
      </main>
    </div>
  </div>
</template>

<style scoped>
.settings-layout {
  display: flex;
  gap: 24px;
  height: calc(100% - 80px);
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 280px;
  flex-shrink: 0;
}

.settings-nav-item {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  text-align: left;
  gap: 4px;
  border-radius: 4px;
  border-left: 2px solid transparent;
  padding: 10px 12px;
  cursor: pointer;
  transition: background-color 0.15s;
  background: transparent;
  border-top: none;
  border-right: none;
  border-bottom: none;
}

.settings-nav-item:hover {
  background-color: var(--bg-surface-hover);
}

.settings-nav-item.active {
  border-left-color: var(--blue);
  background-color: var(--bg-surface-hover);
}

.nav-label {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text);
}

.nav-desc {
  font-size: 0.75rem;
  color: var(--text-secondary);
  line-height: 1.375;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.settings-content {
  flex: 1;
  overflow-y: auto;
}
</style>
