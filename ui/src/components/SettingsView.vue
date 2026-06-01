<script setup lang="ts">
import { computed, onMounted, ref, type Component } from 'vue';
import { MessageSquare, Package, Server, Settings, SlidersHorizontal } from '@lucide/vue';
import { useAgentsAdminStore } from '../stores/agentsAdmin';
import SettingsAgents from './SettingsAgents.vue';
import SettingsAppSettings from './SettingsAppSettings.vue';
import SettingsEmbeddings from './SettingsEmbeddings.vue';
import SettingsMcpServers from './SettingsMcpServers.vue';

// Preserve the legacy event surface until T-413.7 rewires App.vue away from
// the old modal entry point. This component does not emit `close` anymore.
defineEmits<{
  (e: 'close'): void;
}>();

type SettingsArea = 'embeddings' | 'appSettings' | 'agents' | 'mcpServers';
type BadgeTone = 'count' | 'status';

interface RailItem {
  id: SettingsArea;
  label: string;
  icon: Component;
  badge: string;
  badgeTone: BadgeTone;
}

interface AreaCopy {
  title: string;
  subtitle: string;
}

const agentsAdminStore = useAgentsAdminStore();
const activeArea = ref<SettingsArea>('embeddings');

const areaCopy: Record<SettingsArea, AreaCopy> = {
  embeddings: {
    title: 'Embeddings',
    subtitle: 'Vector model powering semantic search. Live from EmbeddingModels list / status.',
  },
  appSettings: {
    title: 'App Settings',
    subtitle: 'Schema-driven settings editor backed by GetSettingsSchema and SetSettings.',
  },
  agents: {
    title: 'Agents',
    subtitle: 'Configured ACP agents from GetConfig.agents with gateway-backed CRUD commands.',
  },
  mcpServers: {
    title: 'MCP Servers',
    subtitle:
      'Configured MCP servers from GetConfig.mcpServers with gateway-backed CRUD commands.',
  },
};

const railItems = computed<RailItem[]>(() => [
  {
    id: 'embeddings',
    label: 'Embeddings',
    icon: Package,
    badge: 'Catalog',
    badgeTone: 'status',
  },
  {
    id: 'appSettings',
    label: 'App Settings',
    icon: SlidersHorizontal,
    badge: 'Schema',
    badgeTone: 'status',
  },
  {
    id: 'agents',
    label: 'Agents',
    icon: MessageSquare,
    badge: String(Object.keys(agentsAdminStore.agents).length),
    badgeTone: 'count',
  },
  {
    id: 'mcpServers',
    label: 'MCP Servers',
    icon: Server,
    badge: String(Object.keys(agentsAdminStore.mcpServers).length),
    badgeTone: 'count',
  },
]);

const activePanel = computed<AreaCopy>(() => areaCopy[activeArea.value]);

onMounted(() => {
  void agentsAdminStore.fetchConfig().catch(() => {
    // The tab panels surface gateway errors inline. The rail only needs a best-effort count.
  });
});

function selectArea(area: SettingsArea): void {
  activeArea.value = area;
}
</script>

<template>
  <div class="settings-view">
    <div class="settings-inner">
      <section class="settings-hero">
        <div class="hero-icon-wrap" aria-hidden="true">
          <Settings :size="34" :stroke-width="1.5" />
        </div>
        <div class="hero-copy">
          <h1 class="hero-title">Settings</h1>
          <p class="hero-description">
            Manage embeddings, app settings, ACP agents, and MCP servers.
          </p>
        </div>
      </section>

      <div class="settings-grid">
        <aside class="settings-rail" aria-label="Settings areas">
          <div class="rail-label">Settings</div>
          <button
            v-for="item in railItems"
            :id="`settings-tab-${item.id}`"
            :key="item.id"
            class="rail-item"
            :class="{ active: activeArea === item.id }"
            type="button"
            :aria-pressed="activeArea === item.id"
            @click="selectArea(item.id)"
          >
            <component :is="item.icon" class="rail-item-icon" :size="18" :stroke-width="1.9" />
            <span class="rail-item-label">{{ item.label }}</span>
            <span class="rail-item-badge" :class="`rail-item-badge--${item.badgeTone}`">
              {{ item.badge }}
            </span>
          </button>
        </aside>

        <section class="settings-content">
          <div class="panel-head">
            <div>
              <h2 class="panel-title">{{ activePanel.title }}</h2>
              <p class="panel-subtitle">{{ activePanel.subtitle }}</p>
            </div>
          </div>

          <SettingsEmbeddings v-if="activeArea === 'embeddings'" />
          <SettingsAppSettings v-else-if="activeArea === 'appSettings'" />
          <SettingsAgents v-else-if="activeArea === 'agents'" />
          <SettingsMcpServers v-else-if="activeArea === 'mcpServers'" />
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1.5rem 1.75rem 3rem;
  background:
    radial-gradient(70% 50% at 50% 0%, color-mix(in srgb, var(--text-accent) 9%, transparent) 0%, transparent 70%),
    var(--bg-main);
}

.settings-inner {
  width: 100%;
  max-width: 1180px;
  margin: 0 auto;
}

.settings-hero {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1.1rem 1.25rem;
  margin-bottom: 1.25rem;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
}

.hero-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 60px;
  height: 60px;
  border-radius: 18px;
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--text-accent) 25%, var(--border-color));
  box-shadow: 0 10px 30px color-mix(in srgb, var(--text-accent) 16%, transparent);
}

.hero-copy {
  min-width: 0;
}

.hero-title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--text-primary);
}

.hero-description {
  margin: 0.25rem 0 0;
  color: var(--text-secondary);
  line-height: 1.5;
}

.settings-grid {
  display: grid;
  grid-template-columns: 228px minmax(0, 1fr);
  gap: 1.25rem;
  align-items: start;
}

.settings-rail {
  position: sticky;
  top: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 14px;
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
}

.rail-label {
  padding: 0.35rem 0.6rem 0.2rem;
  font-size: 0.68rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.rail-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  width: 100%;
  padding: 0.55rem 0.65rem;
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.86rem;
  font-weight: 500;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.rail-item:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.rail-item.active {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, var(--border-color));
}

.rail-item-icon {
  flex-shrink: 0;
}

.rail-item-label {
  min-width: 0;
}

.rail-item-badge {
  margin-left: auto;
  padding: 0.08rem 0.45rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--bg-sidebar);
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  line-height: 1.4;
}

.rail-item-badge--status {
  color: var(--text-muted);
}

.rail-item-badge--count {
  color: var(--text-primary);
}

.rail-item.active .rail-item-badge {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 8%, var(--bg-main));
}

.settings-content {
  min-width: 0;
  min-height: 380px;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--bg-main);
  overflow: hidden;
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem 1.15rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
}

.panel-title {
  margin: 0;
  font-size: 1.2rem;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--text-primary);
}

.panel-subtitle {
  margin: 0.2rem 0 0;
  color: var(--text-secondary);
  font-size: 0.88rem;
  line-height: 1.45;
}

.placeholder-card {
  display: flex;
  align-items: flex-start;
  gap: 1rem;
  margin: 1.15rem;
  padding: 1.2rem 1.15rem;
  border: 1px dashed color-mix(in srgb, var(--text-accent) 26%, var(--border-color));
  border-radius: 14px;
  background: color-mix(in srgb, var(--bg-sidebar) 45%, var(--bg-main));
}

.placeholder-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 56px;
  border-radius: 16px;
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--text-accent) 25%, var(--border-color));
  box-shadow: 0 8px 22px color-mix(in srgb, var(--text-accent) 12%, transparent);
  flex-shrink: 0;
}

.placeholder-copy {
  min-width: 0;
}

.placeholder-eyebrow {
  display: inline-flex;
  align-items: center;
  margin-bottom: 0.45rem;
  color: var(--text-muted);
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.placeholder-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--text-primary);
}

.placeholder-description {
  margin: 0.4rem 0 0;
  color: var(--text-secondary);
  line-height: 1.6;
}

@media (max-width: 900px) {
  .settings-view {
    padding: 1.25rem 1rem 2rem;
  }

  .settings-grid {
    grid-template-columns: 1fr;
  }

  .settings-rail {
    position: static;
    flex-direction: row;
    flex-wrap: wrap;
  }

  .rail-label {
    width: 100%;
  }

  .rail-item {
    flex: 1 1 calc(50% - 0.3rem);
  }
}

@media (max-width: 640px) {
  .settings-hero {
    align-items: flex-start;
  }

  .panel-head,
  .placeholder-card {
    flex-direction: column;
    align-items: flex-start;
  }

  .rail-item {
    flex-basis: 100%;
  }
}
</style>
