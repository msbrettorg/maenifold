<script setup lang="ts">
import { ref } from 'vue';
import SettingsConnection from './SettingsConnection.vue';
import SettingsMaenifold from './SettingsMaenifold.vue';
import SettingsMcpServers from './SettingsMcpServers.vue';
import SettingsPreferences from './SettingsPreferences.vue';

const sections = [
  { id: 'connection', label: 'Agents', description: 'Manage ACP agents and their configuration.' },
  { id: 'mcp-servers', label: 'MCP servers', description: 'Configure global MCP server connections.' },
  { id: 'maenifold', label: 'Maenifold', description: 'Configure maenifold runtime behavior and tuning.' },
  { id: 'preferences', label: 'Preferences', description: 'Control persisted application defaults.' },
] as const;

type SectionId = (typeof sections)[number]['id'];

const activeSection = ref<SectionId>('connection');
</script>

<template>
  <div class="settings-panel">
    <header class="settings-header">
      <div>
        <h2>Settings</h2>
        <p>Configure your agent catalog and local application preferences.</p>
      </div>
    </header>

    <div class="settings-layout">
      <nav class="settings-nav" aria-label="Settings sections">
        <button
          v-for="section in sections"
          :key="section.id"
          class="nav-item"
          :class="{ active: activeSection === section.id }"
          @click="activeSection = section.id"
        >
          <span class="nav-label">{{ section.label }}</span>
          <span class="nav-description">{{ section.description }}</span>
        </button>
      </nav>

      <main class="settings-content">
        <SettingsConnection v-if="activeSection === 'connection'" />
        <SettingsMcpServers v-else-if="activeSection === 'mcp-servers'" />
        <SettingsMaenifold v-else-if="activeSection === 'maenifold'" />
        <SettingsPreferences v-else />
      </main>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  width: 100%;
  height: 100%;
  max-height: none;
  display: flex;
  flex-direction: column;
  background: var(--surface-container);
}

.settings-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.25rem;
}

.settings-header h2,
.settings-header p {
  margin: 0;
}

.settings-header h2 {
  font-family: var(--font-display);
  font-weight: 500;
}

.settings-header p {
  margin-top: 0.375rem;
  color: var(--color-text-secondary);
}

.settings-layout {
  display: grid;
  grid-template-columns: minmax(220px, 260px) minmax(0, 1fr);
  min-height: 0;
  flex: 1;
  overflow: hidden;
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 1rem;
  background: var(--surface);
  overflow-y: auto;
}

.nav-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.75rem 1rem;
  border-left: 4px solid transparent;
  border-radius: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.nav-item:hover {
  background: var(--surface-high);
}

.nav-item.active {
  background: var(--surface-high);
  border-left-color: var(--color-brand);
}

.nav-label {
  font-weight: 600;
}

.nav-description {
  color: var(--color-text-secondary);
  font-size: 0.75rem;
}

.settings-content {
  min-height: 0;
  overflow-y: auto;
  padding: 1.25rem;
}

@media (max-width: 720px) {
  .settings-layout {
    grid-template-columns: 1fr;
  }

  .settings-nav {
    border-bottom: none;
  }
}
</style>
