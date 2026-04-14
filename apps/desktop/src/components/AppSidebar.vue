<script setup lang="ts">
import AgentSelector from './AgentSelector.vue';
import SessionList from './SessionList.vue';
import StartupProgress from './StartupProgress.vue';
import { useResizeHandle } from '../composables/useResizeHandle';
import { getSidebarSizingTokens } from '../lib/sidebarSizing';
import type { SavedSession } from '../lib/types';

const visible = defineModel<boolean>('visible', { required: true });
const selectedAgent = defineModel<string>('selectedAgent', { required: true });
const showTrafficMonitor = defineModel<boolean>('showTrafficMonitor', { required: true });
const showStartupDetails = defineModel<boolean>('showStartupDetails', { required: true });

defineProps<{
  selectedCwd: string;
  hasAgents: boolean;
  isConnecting: boolean;
  startupPhase: string;
  startupLogs: string[];
  startupElapsed: number;
}>();

const emit = defineEmits<{
  'open-settings': [];
  'agent-select': [agentName: string];
  'select-folder': [];
  'new-session': [];
  'resume-session': [session: SavedSession];
  'delete-session': [sessionId: string];
  'cancel-connection': [];
}>();

const sidebarSizing = getSidebarSizingTokens();
const resize = useResizeHandle({
  initial: sidebarSizing.width,
  min: sidebarSizing.minWidth,
  max: sidebarSizing.maxWidth,
});
</script>

<template>
  <!-- Sidebar panel -->
  <aside
    v-if="visible"
    class="sidebar flex shrink-0 flex-col border-r border-[var(--border)] bg-[var(--card)]"
    role="navigation"
    aria-label="Main navigation"
    :style="{ width: resize.width.value + 'px', minWidth: resize.width.value + 'px' }"
  >
    <div class="flex items-center justify-between border-b border-[var(--border)] p-4">
      <h1 class="m-0 font-[var(--font-display)] text-[var(--text-title)] tracking-[0.06em]">maenifold</h1>
      <div class="flex gap-1">
        <button
          :class="[
            'cursor-pointer border-none bg-transparent px-2 py-1 text-[var(--text-caption)] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]',
            showTrafficMonitor && 'rounded bg-[var(--muted)] text-[var(--primary)]',
          ]"
          @click="showTrafficMonitor = !showTrafficMonitor"
          title="ACP Traffic Monitor"
        >📡</button>
        <button
          class="cursor-pointer border-none bg-transparent px-2 py-1 text-[var(--text-caption)] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
          @click="emit('open-settings')"
          title="Settings"
        >⚙</button>
        <button
          class="cursor-pointer border-none bg-transparent px-2 py-1 text-[var(--text-caption)] text-[var(--muted-foreground)] transition-colors hover:text-[var(--foreground)]"
          @click="visible = false"
        >◀</button>
      </div>
    </div>

    <div class="sidebar-content">
      <!-- Agent Selection -->
      <div class="section px-4 py-3">
        <AgentSelector
          v-model:selected="selectedAgent"
          @select="(name: string) => emit('agent-select', name)"
        />

        <!-- Working Directory Picker -->
        <div class="mt-3">
          <label
            class="mb-1 block text-[var(--text-tiny)] font-[var(--weight-title)] tracking-[0.06em] text-[var(--muted-foreground)] uppercase"
          >Working Directory:</label>
          <div class="flex items-center gap-2">
            <span
              class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap rounded border border-[var(--border)] bg-[var(--background)] px-2 py-1.5 text-[var(--text-caption)] text-[var(--foreground)]"
              :title="selectedCwd || 'Current directory'"
            >
              {{ selectedCwd ? selectedCwd.split(/[\\/]/).pop() : '.' }}
            </span>
            <button
              class="cursor-pointer rounded border border-[var(--border)] bg-transparent px-2 py-1.5 text-[var(--text-body-sm)] transition-colors hover:bg-[var(--muted)] disabled:cursor-not-allowed disabled:opacity-50"
              @click="emit('select-folder')"
              title="Select folder"
            >
              📁
            </button>
          </div>
        </div>

        <button
          v-if="hasAgents"
          class="mt-3 w-full cursor-pointer rounded-md border border-[var(--primary)] bg-transparent px-4 py-2.5 text-[var(--text-body-sm)] font-[var(--weight-display)] text-[var(--primary)] transition-colors hover:bg-[color-mix(in_srgb,var(--primary)_12%,transparent)] disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="!selectedAgent || isConnecting"
          @click="emit('new-session')"
        >
          New Session
        </button>

        <!-- Startup Progress -->
        <StartupProgress
          v-if="isConnecting"
          :agent-name="selectedAgent"
          :phase="startupPhase"
          :logs="startupLogs"
          :elapsed-seconds="startupElapsed"
          :show-details="showStartupDetails"
          @cancel="emit('cancel-connection')"
          @toggle-details="showStartupDetails = !showStartupDetails"
        />
      </div>

      <!-- Session List -->
      <div class="section px-4 py-3">
        <SessionList
          @resume="(session: SavedSession) => emit('resume-session', session)"
          @delete="(id: string) => emit('delete-session', id)"
        />
      </div>

    </div>
  </aside>

  <!-- Sidebar resize handle -->
  <div
    v-if="visible"
    class="relative z-10 -mx-[3px] w-[6px] shrink-0 cursor-col-resize transition-colors hover:bg-[var(--ring)] active:bg-[var(--ring)]"
    @mousedown="resize.startResize($event, 'right')"
  />

  <!-- Collapsed sidebar toggle -->
  <button
    v-if="!visible"
    class="fixed top-1/2 left-0 cursor-pointer rounded-r border border-l-0 border-[var(--border)] bg-[var(--card)] p-2 text-[var(--muted-foreground)] -translate-y-1/2 transition-colors hover:text-[var(--foreground)]"
    @click="visible = true"
  >
    ▶
  </button>
</template>

<style scoped>
.sidebar-content {
  flex: 1;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: var(--border) transparent;
}

.sidebar-content::-webkit-scrollbar {
  width: 8px;
}

.sidebar-content::-webkit-scrollbar-track {
  background: transparent;
}

.sidebar-content::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 4px;
}

.sidebar-content::-webkit-scrollbar-thumb:hover {
  background: var(--muted-foreground);
}

.section + .section {
  border-top: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
}

.section :deep(.agent-selector label),
.section :deep(.probe-preview.compact .field-label),
.section :deep(.probe-preview.compact .option-count) {
  font-size: var(--text-tiny);
}

.section :deep(.agent-selector select),
.section :deep(.probe-preview.compact .preview-select),
.section :deep(.probe-preview.compact .field-current),
.section :deep(.probe-preview.compact .default-summary),
.section :deep(.probe-preview.compact .status-message),
.section :deep(.probe-preview.compact .field-description),
.section :deep(.probe-preview.compact .warning-note),
.section :deep(.probe-preview.compact .degraded-note),
.section :deep(.probe-preview.compact .technical-detail) {
  font-size: var(--text-caption);
}
</style>
