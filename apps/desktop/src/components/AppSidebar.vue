<script setup lang="ts">
import AgentSelector from './AgentSelector.vue';
import SessionList from './SessionList.vue';
import StartupProgress from './StartupProgress.vue';
import { Button } from '@/components/ui/button';
import { useResizeHandle } from '../composables/useResizeHandle';
import { getSidebarSizingTokens } from '../lib/sidebarSizing';
import type { SavedSession } from '../lib/types';
import { Radio, PanelLeftClose, PanelLeftOpen, FolderOpen } from 'lucide-vue-next';

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
  <!-- audit-ok: dynamic-binding (resizable sidebar width) -->
  <aside
    v-if="visible"
    class="sidebar flex shrink-0 flex-col bg-[var(--surface)] text-[var(--text)]"
    role="complementary"
    aria-label="Agents panel"
    :style="{ width: resize.width.value + 'px', minWidth: resize.width.value + 'px' }"
  >
    <div class="flex items-center justify-between px-4 pt-3 pb-2">
      <h2 class="m-0 text-xs font-medium tracking-[0.02em] text-text-secondary">Agents</h2>
      <div class="flex gap-1">
        <Button
          variant="ghost"
          size="icon-sm"
          :class="[
            'cursor-pointer rounded-[4px] border-none bg-transparent px-2 py-1 text-[var(--text-caption)] text-[var(--text-secondary)] transition-colors hover:bg-[var(--surface-high)] hover:text-[var(--text)]',
            showTrafficMonitor && 'bg-[var(--surface-high)] text-[var(--color-brand)]',
          ]"
          @click="showTrafficMonitor = !showTrafficMonitor"
          title="ACP Traffic Monitor"
          aria-label="Toggle ACP traffic monitor"
        ><Radio class="size-4" /></Button>
        <Button
          variant="ghost"
          size="icon-sm"
          class="cursor-pointer rounded-[4px] border-none bg-transparent px-2 py-1 text-[var(--text-caption)] text-[var(--text-secondary)] transition-colors hover:bg-[var(--surface-high)] hover:text-[var(--text)]"
          @click="visible = false"
          title="Collapse sidebar"
          aria-label="Collapse sidebar"
        ><PanelLeftClose class="size-4" /></Button>
      </div>
    </div>

    <div class="sidebar-content flex-1 flex flex-col gap-1 overflow-y-auto [scrollbar-width:thin] [scrollbar-color:var(--color-border-subtle)_transparent] [&::-webkit-scrollbar]:w-2 [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar-thumb]:bg-[var(--color-border-subtle)] [&::-webkit-scrollbar-thumb]:rounded-[4px] [&::-webkit-scrollbar-thumb:hover]:bg-[var(--color-text-secondary)]">
      <!-- Agent Selection -->
      <div class="section px-4 pt-1 pb-3">
        <AgentSelector
          v-model:selected="selectedAgent"
          @select="(name: string) => emit('agent-select', name)"
        />

        <!-- Working Directory Picker -->
        <div class="mt-3">
          <label
            class="mb-1.5 block text-[var(--text-tiny)] font-normal tracking-normal text-[var(--text-secondary)]"
          >Working directory</label>
          <div class="flex items-center gap-2">
            <span
              class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap rounded-[4px] bg-[var(--surface-container)] px-2.5 py-2 text-[var(--text-caption)] text-[var(--text)]"
              :title="selectedCwd || 'Current directory'"
            >
              {{ selectedCwd ? selectedCwd.split(/[\\/]/).pop() : '.' }}
            </span>
            <Button
              variant="ghost"
              size="icon-sm"
              class="cursor-pointer rounded-[4px] border-none bg-[var(--surface-container)] px-2.5 py-2 text-[var(--text-body-sm)] transition-colors hover:bg-[var(--surface-high)] disabled:cursor-not-allowed disabled:opacity-50"
              @click="emit('select-folder')"
              title="Select folder"
            >
              <FolderOpen class="size-4" />
            </Button>
          </div>
        </div>

        <Button
          v-if="hasAgents"
          variant="default"
          class="mt-4 w-full cursor-pointer rounded-[4px] border-none bg-[var(--color-brand)] px-4 py-2.5 text-[var(--text-body-sm)] font-[var(--weight-display)] text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="!selectedAgent || isConnecting"
          @click="emit('new-session')"
        >
          New session
        </Button>

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
      <div class="section px-4 pt-2 pb-4">
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
    class="relative z-10 -mx-[3px] w-[6px] shrink-0 cursor-col-resize transition-colors hover:bg-[var(--color-brand)] active:bg-[var(--color-brand)]"
    @mousedown="resize.startResize($event, 'right')"
  />

  <!-- Collapsed sidebar toggle -->
  <!--
    NOTE: This must remain a native <button>, not the shadcn <Button> component.
    See T-205.702 — when this was converted to <Button> in 8bc192360 the click
    handler stopped restoring the sidebar in the live Electron app even though
    the compiled render function looked correct. Reverting to a native button
    restores expected behavior; do not "polish" this back to <Button>.
  -->
  <!-- audit-ok: no-raw-button -->
  <button
    v-if="!visible"
    type="button"
    class="fixed top-1/2 left-0 z-50 inline-flex size-9 -translate-y-1/2 cursor-pointer items-center justify-center rounded-r border-none bg-[var(--surface-container)] p-2 text-[var(--text-secondary)] transition-colors hover:bg-[var(--surface-high)] hover:text-[var(--text)]"
    @click="visible = true"
    title="Expand sidebar"
    aria-label="Expand sidebar"
  >
    <PanelLeftOpen class="size-4" />
  </button>
</template>

<!-- audit-ok: deep-third-party — :deep() targets AgentSelector and ProbePreview internal label/control sizing -->
<style scoped>
.section :deep(.agent-selector label),
.section :deep(.probe-preview.compact .field-label) {
  font-size: var(--text-tiny);
}

.section :deep(.agent-selector select),
.section :deep(.probe-preview.compact .preview-select),
.section :deep(.probe-preview.compact .status-message),
.section :deep(.probe-preview.compact .warning-note),
.section :deep(.probe-preview.compact .degraded-note),
.section :deep(.probe-preview.compact .technical-detail) {
  font-size: var(--text-caption);
}
</style>
