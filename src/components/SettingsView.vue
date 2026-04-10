<script setup lang="ts">
import { ref } from 'vue';
import SettingsConnection from './SettingsConnection.vue';
import SettingsPreferences from './SettingsPreferences.vue';

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const sections = [
  { id: 'connection', label: 'Connection', description: 'Manage agents and config file access.' },
  { id: 'preferences', label: 'Preferences', description: 'Control persisted application defaults.' },
] as const;

type SectionId = (typeof sections)[number]['id'];

const activeSection = ref<SectionId>('connection');
</script>

<template>
  <div class="settings-overlay" @click.self="emit('close')">
    <div class="settings-panel">
      <header class="settings-header">
        <div>
          <h2>Settings</h2>
          <p>Configure your agent catalog and local application preferences.</p>
        </div>
        <button class="close-btn" aria-label="Close settings" @click="emit('close')">✕</button>
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
          <SettingsPreferences v-else />
        </main>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-overlay);
}

@supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
  .settings-overlay {
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    background: rgba(0, 0, 0, 0.4);
  }
}

.settings-panel {
  width: min(960px, 92vw);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-main);
  border-radius: 12px;
  box-shadow: 0 4px 20px color-mix(in srgb, var(--bg-main) 30%, transparent);
  overflow: hidden;
}

.settings-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.25rem;
  border-bottom: 1px solid var(--border-color);
}

.settings-header h2,
.settings-header p {
  margin: 0;
}

.settings-header p {
  margin-top: 0.375rem;
  color: var(--text-secondary);
}

.close-btn {
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--text-xl);
  cursor: pointer;
}

.close-btn:hover {
  color: var(--text-primary);
}

.settings-layout {
  display: grid;
  grid-template-columns: minmax(220px, 260px) minmax(0, 1fr);
  min-height: 0;
  flex: 1;
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 1rem;
  border-right: 1px solid var(--border-color);
  background: var(--bg-sidebar);
}

.nav-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.875rem 1rem;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.nav-item:hover {
  background: var(--bg-hover);
}

.nav-item.active {
  background: var(--bg-main);
  border-color: var(--border-color);
}

.nav-label {
  font-weight: 600;
}

.nav-description {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.settings-content {
  min-height: 0;
  overflow-y: auto;
  padding: 1.25rem;
}

@media (max-width: 720px) {
  .settings-panel {
    width: 96vw;
  }

  .settings-layout {
    grid-template-columns: 1fr;
  }

  .settings-nav {
    border-right: none;
    border-bottom: 1px solid var(--border-color);
  }
}
</style>
