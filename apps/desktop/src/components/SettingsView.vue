<script setup lang="ts">
import { TabsRoot, TabsList, TabsTrigger, TabsContent } from 'reka-ui';
import SettingsConnection from './SettingsConnection.vue';
import SettingsMaenifold from './SettingsMaenifold.vue';
import SettingsMcpServers from './SettingsMcpServers.vue';
import SettingsEmbeddings from './SettingsEmbeddings.vue';
import SettingsPreferences from './SettingsPreferences.vue';

const sections = [
  { id: 'connection', label: 'Agents', description: 'Manage ACP agents and their configuration.' },
  { id: 'mcp-servers', label: 'MCP servers', description: 'Configure global MCP server connections.' },
  { id: 'maenifold', label: 'Maenifold', description: 'Configure maenifold runtime behavior and tuning.' },
  { id: 'embeddings', label: 'Embeddings', description: 'Manage embedding models and vector index.' },
  { id: 'preferences', label: 'Preferences', description: 'Control persisted application defaults.' },
];
</script>

<template>
  <div class="h-full p-6">
    <h2 class="text-lg font-semibold text-text-primary mb-1">Settings</h2>
    <p class="text-sm text-text-secondary mb-6">
      Configure your agent catalog and local application preferences.
    </p>

    <TabsRoot default-value="connection" orientation="vertical" class="settings-layout">
      <TabsList class="settings-nav" aria-label="Settings sections">
        <TabsTrigger
          v-for="s in sections"
          :key="s.id"
          :value="s.id"
          class="settings-nav-item"
        >
          <span class="nav-label">{{ s.label }}</span>
          <span class="nav-desc">{{ s.description }}</span>
        </TabsTrigger>
      </TabsList>

      <div class="settings-content">
        <TabsContent value="connection"><SettingsConnection /></TabsContent>
        <TabsContent value="mcp-servers"><SettingsMcpServers /></TabsContent>
        <TabsContent value="maenifold"><SettingsMaenifold /></TabsContent>
        <TabsContent value="embeddings"><SettingsEmbeddings /></TabsContent>
        <TabsContent value="preferences"><SettingsPreferences /></TabsContent>
      </div>
    </TabsRoot>
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

.settings-nav-item[data-state="active"] {
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
