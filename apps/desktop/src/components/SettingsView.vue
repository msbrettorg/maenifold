<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue';
import SettingsConnection from './SettingsConnection.vue';
import SettingsMaenifold from './SettingsMaenifold.vue';
import SettingsMcpServers from './SettingsMcpServers.vue';
import SettingsPreferences from './SettingsPreferences.vue';

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const sections = [
  { id: 'connection', label: 'Agents', description: 'Manage ACP agents and their configuration.' },
  { id: 'mcp-servers', label: 'MCP servers', description: 'Configure global MCP server connections.' },
  { id: 'maenifold', label: 'Maenifold', description: 'Configure maenifold runtime behavior and tuning.' },
  { id: 'preferences', label: 'Preferences', description: 'Control persisted application defaults.' },
] as const;

type SectionId = (typeof sections)[number]['id'];

const activeSection = ref<SectionId>('connection');
const overlayRef = ref<HTMLElement | null>(null);

const FOCUSABLE = 'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    emit('close');
    return;
  }
  if (e.key === 'Tab') {
    const panel = overlayRef.value?.querySelector('.settings-panel') as HTMLElement | null;
    if (!panel) return;
    const focusable = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE));
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }
}

onMounted(async () => {
  document.addEventListener('keydown', onKeydown);
  await nextTick();
  const panel = overlayRef.value?.querySelector('.settings-panel') as HTMLElement | null;
  const first = panel?.querySelector<HTMLElement>(FOCUSABLE);
  first?.focus();
});

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <div ref="overlayRef" class="settings-overlay" @click.self="emit('close')">
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
          <SettingsMcpServers v-else-if="activeSection === 'mcp-servers'" />
          <SettingsMaenifold v-else-if="activeSection === 'maenifold'" />
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
  background: rgba(0, 0, 0, 0.6);
}

@supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
  .settings-overlay {
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    background: rgba(0, 0, 0, 0.6);
  }
}

.settings-panel {
  width: min(960px, 92vw);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  background: #201f1f;
  border-radius: 4px;
  box-shadow: 0 0 32px rgba(173, 198, 255, 0.08);
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

.settings-header p {
  margin-top: 0.375rem;
  color: var(--muted-foreground);
}

.close-btn {
  border: none;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 1.125rem;
  cursor: pointer;
}

.close-btn:hover {
  color: var(--foreground);
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
  background: #1c1b1b;
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
  background: #2a2a2a;
}

.nav-item.active {
  background: #2a2a2a;
  border-left-color: #4d8eff;
}

.nav-label {
  font-weight: 600;
}

.nav-description {
  color: var(--muted-foreground);
  font-size: 0.75rem;
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
    border-bottom: none;
  }
}
</style>
